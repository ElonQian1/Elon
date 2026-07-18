//! Resume admission and status guards for Desktop-supervised local tasks.

use std::{path::Path as FsPath, sync::Arc};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use homecli_proto::CliProjectContext;
use serde_json::json;

use crate::{node_agent_local_task_resume::ResolvedResumeWorkspace, NodeRuntime};

pub(crate) async fn resolve_supervised_resume_workspace(
    runtime: &Arc<NodeRuntime>,
    creds: &crate::Credentials,
    project_id: &str,
    requested_workspace_path: &str,
    supervision: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
) -> Result<Option<ResolvedResumeWorkspace>, Response> {
    let Some(contract) = supervision.filter(|contract| contract.task_role == "resume_original")
    else {
        return Ok(None);
    };
    let Some(parent_task_id) = contract.parent_task_id.as_deref() else {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "resume_original 缺少 parent_task_id。",
        ));
    };
    let parent = match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, parent_task_id)
    {
        Ok(Some(parent)) => parent,
        Ok(None) => {
            return Err(json_error(
                StatusCode::NOT_FOUND,
                "resume_original 的父任务不存在或不属于当前账号。",
            ))
        }
        Err(error) => return Err(internal_error(error)),
    };
    if parent.agent_id != creds.agent_id || parent.install_id != runtime.install_id {
        return Err(json_error(
            StatusCode::CONFLICT,
            "父任务不属于当前节点和安装实例，已拒绝继承工作区。",
        ));
    }
    match crate::node_agent_local_task_supervision::load_supervision_state(
        &runtime.task_journal,
        &parent.task_id,
    ) {
        Ok(state) if state.enabled => {}
        Ok(_) => {
            return Err(json_error(
                StatusCode::CONFLICT,
                "父任务没有可验证的桌面监督契约，已拒绝继承工作区。",
            ))
        }
        Err(error) => return Err(internal_error(error)),
    }
    let journal_snapshot = runtime
        .task_journal
        .snapshot(&parent.task_id, 0, 1)
        .map_err(internal_error)?;
    let receipt = runtime
        .update_recovery
        .receipt_for_task(&parent.task_id)
        .map_err(internal_error)?;
    let mut resolved = crate::node_agent_local_task_resume::resolve_resume_workspace(
        contract,
        &parent,
        journal_snapshot.record.as_ref(),
        project_id,
        requested_workspace_path,
        receipt.as_ref(),
        crate::node_agent_local_task_resume::ResumeWorkspaceMode::Inspect,
    )
    .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;

    let identity = crate::node_agent_full_access::FullAccessGrantIdentity::new(
        &creds.owner_user_id,
        &creds.agent_id,
        &runtime.install_id,
    )
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    let authorization_context = CliProjectContext {
        project_id: project_id.to_string(),
        conversation_id: parent.conversation_id.clone(),
        runtime_permission: Some("full_access".to_string()),
    };
    crate::node_agent_full_access::require_route_a_full_access_grant(
        &runtime.full_access_grants,
        &identity,
        "codex",
        Some("full_access"),
        Some(&authorization_context),
        Some(&resolved.authorized_workspace_path),
        false,
    )
    .await
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;

    let recorded_workspace_exists =
        FsPath::new(&resolved.inherited_workspace.workspace_path).exists();
    if recorded_workspace_exists
        && !runtime
            .active_cli_prompt_views_for_workspace(FsPath::new(
                &resolved.inherited_workspace.workspace_path,
            ))
            .await
            .is_empty()
    {
        return Err(json_error(
            StatusCode::CONFLICT,
            "父任务隔离 worktree 已被活跃任务占用，已拒绝续跑。",
        ));
    }
    if recorded_workspace_exists
        && live_sidecar_occupies_workspace(runtime, &resolved.inherited_workspace.workspace_path)
            .map_err(internal_error)?
    {
        return Err(json_error(
            StatusCode::CONFLICT,
            "父任务隔离 worktree 仍被存活 sidecar 占用，已拒绝续跑。",
        ));
    }
    if resolved.requires_recreation {
        resolved = crate::node_agent_local_task_resume::resolve_resume_workspace(
            contract,
            &parent,
            journal_snapshot.record.as_ref(),
            project_id,
            requested_workspace_path,
            receipt.as_ref(),
            crate::node_agent_local_task_resume::ResumeWorkspaceMode::Acquire,
        )
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    }
    Ok(Some(resolved))
}

pub(crate) async fn inspect_resume_workspace_status(
    runtime: &Arc<NodeRuntime>,
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    supervision_enabled: bool,
) -> serde_json::Value {
    if !supervision_enabled {
        return json!({"eligible": false, "reason": "missing_supervision_contract"});
    }
    let contract = crate::node_agent_local_task_supervision::SupervisionContract {
        protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some(parent.task_id.clone()),
        root_task_id: Some(parent.task_id.clone()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_or_unblock".to_string(),
    };
    let receipt = runtime
        .update_recovery
        .receipt_for_task(&parent.task_id)
        .ok()
        .flatten();
    match crate::node_agent_local_task_resume::resolve_resume_workspace(
        &contract,
        parent,
        journal_record,
        &parent.project_id,
        &parent.workspace_path,
        receipt.as_ref(),
        crate::node_agent_local_task_resume::ResumeWorkspaceMode::Inspect,
    ) {
        Ok(resolved) => {
            let recorded_workspace_exists =
                FsPath::new(&resolved.inherited_workspace.workspace_path).exists();
            let prompt_occupied = recorded_workspace_exists
                && !runtime
                    .active_cli_prompt_views_for_workspace(FsPath::new(
                        &resolved.inherited_workspace.workspace_path,
                    ))
                    .await
                    .is_empty();
            let sidecar_occupied = recorded_workspace_exists
                && live_sidecar_occupies_workspace(
                    runtime,
                    &resolved.inherited_workspace.workspace_path,
                )
                .unwrap_or(true);
            json!({
                "eligible": !prompt_occupied && !sidecar_occupied,
                "derivation": resolved.derivation,
                "authorized_workspace_path": resolved.authorized_workspace_path,
                "active_workspace_path": resolved.inherited_workspace.workspace_path,
                "branch": resolved.inherited_workspace.branch,
                "git_head": resolved.git_head,
                "occupied": prompt_occupied || sidecar_occupied,
                "recovery_required": resolved.requires_recreation,
                "requires_recreation": resolved.requires_recreation,
            })
        }
        Err(error) => json!({"eligible": false, "reason": error.to_string()}),
    }
}

fn live_sidecar_occupies_workspace(
    runtime: &NodeRuntime,
    workspace_path: &str,
) -> anyhow::Result<bool> {
    let expected = std::fs::canonicalize(workspace_path)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(runtime
        .cli_sidecars
        .all_sessions()?
        .into_iter()
        .any(|session| {
            session.is_live_at(now)
                && session
                    .cwd
                    .as_deref()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                    .is_some_and(|path| path == expected)
        }))
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": message.into() })),
    )
        .into_response()
}

fn internal_error(error: anyhow::Error) -> Response {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
