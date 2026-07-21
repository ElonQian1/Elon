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
    conversation_id: &str,
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
    let parent_contract = match crate::node_agent_local_task_supervision::load_supervision_contract(
        &runtime.task_journal,
        &parent.task_id,
    ) {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            return Err(json_error(
                StatusCode::CONFLICT,
                "父任务没有可验证的桌面监督契约，已拒绝继承工作区。",
            ))
        }
        Err(error) => return Err(internal_error(error)),
    };
    let journal_snapshot = runtime
        .task_journal
        .snapshot(&parent.task_id, 0, 1)
        .map_err(internal_error)?;
    let receipt = runtime
        .update_recovery
        .receipt_for_task(&parent.task_id)
        .map_err(internal_error)?;
    crate::node_agent_local_tasks::root_workspace::resolve_resume_authorized_workspace(
        &runtime.local_tasks,
        &runtime.task_journal,
        &runtime.update_recovery,
        creds,
        &runtime.install_id,
        project_id,
        &parent,
        contract,
    )
    .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    let mut resolved = crate::node_agent_local_task_resume::resolve_resume_workspace(
        contract,
        &parent,
        Some(&parent_contract),
        journal_snapshot.record.as_ref(),
        project_id,
        requested_workspace_path,
        receipt.as_ref(),
        crate::node_agent_local_task_resume::ResumeWorkspaceMode::Inspect,
    )
    .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    validate_migration_lineage(runtime, &parent, &parent_contract, &resolved)
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;

    let admission_base = resolved.authorized_workspace_path.clone();
    let admission = tokio::task::spawn_blocking(move || {
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(FsPath::new(
            &admission_base,
        ))
    })
    .await
    .map_err(|error| internal_error(anyhow::Error::from(error)))?
    .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    resolved = crate::node_agent_local_task_resume::resolve_resume_workspace(
        contract,
        &parent,
        Some(&parent_contract),
        journal_snapshot.record.as_ref(),
        project_id,
        requested_workspace_path,
        receipt.as_ref(),
        crate::node_agent_local_task_resume::ResumeWorkspaceMode::Inspect,
    )
    .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    validate_migration_lineage(runtime, &parent, &parent_contract, &resolved)
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    if resolved.snapshot_continue_required {
        validate_snapshot_continue_safety(runtime, creds, &parent, &journal_snapshot)
            .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    }

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
        Some(&parent),
    )
    .await
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;

    if !runtime
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
    if live_sidecar_occupies_workspace(runtime, &resolved.inherited_workspace.workspace_path)
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
            Some(&parent_contract),
            journal_snapshot.record.as_ref(),
            project_id,
            requested_workspace_path,
            receipt.as_ref(),
            crate::node_agent_local_task_resume::ResumeWorkspaceMode::Acquire,
        )
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    }
    if resolved.snapshot_continue_required {
        let data_paths = runtime.node_data_root.read().await.paths.clone();
        let workspace_root = data_paths
            .as_ref()
            .map(|paths| paths.workspaces())
            .ok_or_else(|| json_error(StatusCode::CONFLICT, "节点缺少平台 workspace root。"))?;
        let prepared = crate::pc_workspace_provisioner::prepare_conversation_workspace_in_with_supervision_at_ref(
            &workspace_root,
            &resolved.authorized_workspace_path,
            project_id,
            conversation_id,
            contract.root_task_id.as_deref(),
            &resolved.git_head,
        )
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
        resolved.inherited_workspace = prepared;
        resolved.derivation = "missing_active_snapshot_continue_created".to_string();
        resolved.snapshot_continue_required = false;
    }
    commit_validated_lease_migration(&resolved, &admission)
        .map_err(|error| json_error(StatusCode::CONFLICT, error.to_string()))?;
    resolved.resume_admission = Some(admission);
    Ok(Some(resolved))
}

pub(crate) async fn inspect_resume_workspace_status(
    runtime: &Arc<NodeRuntime>,
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    journal_record: Option<&crate::node_agent_task_journal::TaskJournalRecord>,
    parent_contract: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
) -> serde_json::Value {
    let Some(parent_contract) = parent_contract else {
        return json!({"eligible": false, "reason": "missing_supervision_contract"});
    };
    let root_task_id = parent_contract
        .root_task_id
        .clone()
        .unwrap_or_else(|| parent.task_id.clone());
    let contract = crate::node_agent_local_task_supervision::SupervisionContract {
        protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some(parent.task_id.clone()),
        root_task_id: Some(root_task_id),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_or_unblock".to_string(),
    };
    let receipt = runtime
        .update_recovery
        .receipt_for_task(&parent.task_id)
        .ok()
        .flatten();
    let creds = crate::Credentials {
        owner_user_id: parent.owner_user_id.clone(),
        agent_id: parent.agent_id.clone(),
        agent_secret: String::new(),
        user_token: None,
    };
    if let Err(error) =
        crate::node_agent_local_tasks::root_workspace::resolve_resume_authorized_workspace(
            &runtime.local_tasks,
            &runtime.task_journal,
            &runtime.update_recovery,
            &creds,
            &runtime.install_id,
            &parent.project_id,
            parent,
            &contract,
        )
    {
        return json!({"eligible": false, "reason": error.to_string()});
    }
    match crate::node_agent_local_task_resume::resolve_resume_workspace(
        &contract,
        parent,
        Some(parent_contract),
        journal_record,
        &parent.project_id,
        &parent.workspace_path,
        receipt.as_ref(),
        crate::node_agent_local_task_resume::ResumeWorkspaceMode::Inspect,
    ) {
        Ok(resolved) => {
            if let Err(error) =
                validate_migration_lineage(runtime, parent, parent_contract, &resolved)
            {
                return json!({"eligible": false, "reason": error.to_string()});
            }
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
                "lease_migration_required": resolved.lease_migration.is_some(),
                "lease_migration_from": resolved.lease_migration.as_ref().map(|migration| migration.legacy_task_id.as_str()),
            })
        }
        Err(error) => json!({"eligible": false, "reason": error.to_string()}),
    }
}

fn validate_migration_lineage(
    runtime: &NodeRuntime,
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    parent_contract: &crate::node_agent_local_task_supervision::SupervisionContract,
    resolved: &ResolvedResumeWorkspace,
) -> anyhow::Result<()> {
    let Some(migration) = resolved.lease_migration.as_ref() else {
        return Ok(());
    };
    crate::node_agent_local_task_resume_lineage::validate_full_lineage(
        &runtime.local_tasks,
        &runtime.task_journal,
        parent,
        parent_contract,
        migration,
    )
}

fn validate_snapshot_continue_safety(
    runtime: &NodeRuntime,
    creds: &crate::Credentials,
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    initial_snapshot: &crate::node_agent_task_journal::TaskJournalSnapshot,
) -> anyhow::Result<()> {
    let snapshot = runtime.task_journal.snapshot(&parent.task_id, 0, 10_000)?;
    let mut duplicate_child = false;
    for child in runtime
        .local_tasks
        .list_all_for_owner_for_safety(&creds.owner_user_id)?
    {
        if child.task_id == parent.task_id {
            continue;
        }
        let Some(contract) = crate::node_agent_local_task_supervision::load_supervision_contract(
            &runtime.task_journal,
            &child.task_id,
        )?
        else {
            continue;
        };
        if contract.task_role == "resume_original"
            && contract.parent_task_id.as_deref() == Some(parent.task_id.as_str())
        {
            duplicate_child = true;
            break;
        }
    }
    validate_snapshot_continue_evidence(
        &snapshot.events,
        initial_snapshot.approvals.pending_count,
        snapshot.has_more,
        duplicate_child,
    )
}

fn validate_snapshot_continue_evidence(
    events: &[crate::node_agent_task_journal::TaskJournalEventView],
    pending_approvals: usize,
    journal_has_more: bool,
    duplicate_child: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        pending_approvals == 0,
        "父任务仍有待处理审批，禁止 snapshot_continue。"
    );
    anyhow::ensure!(!journal_has_more, "父任务 journal 超出安全检查上限。");
    let terminal_evidence_seen = events.iter().any(|view| {
        matches!(
            view.event.get("type").and_then(serde_json::Value::as_str),
            Some("finished" | "done" | "failed" | "canceled" | "interrupted" | "resume_required")
        )
    });
    anyhow::ensure!(
        terminal_evidence_seen,
        "父任务 journal 缺少完整终态证据，禁止 snapshot_continue。"
    );
    let unsafe_action = events.iter().any(|view| {
        view.event
            .pointer("/payload/safety/non_repeatable_action")
            .or_else(|| view.event.pointer("/safety/non_repeatable_action"))
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
    });
    anyhow::ensure!(
        !unsafe_action,
        "父任务包含不可重复动作，禁止 snapshot_continue。"
    );
    anyhow::ensure!(
        !duplicate_child,
        "父任务已存在 resume_original 子任务，禁止重复续跑。"
    );
    Ok(())
}

pub(crate) fn commit_validated_lease_migration(
    resolved: &ResolvedResumeWorkspace,
    admission: &crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard,
) -> anyhow::Result<()> {
    if let Some(migration) = resolved.lease_migration.as_ref() {
        return crate::node_agent_supervision_worktree_lease::migrate_legacy_child_lease(
            admission,
            FsPath::new(&resolved.authorized_workspace_path),
            FsPath::new(&resolved.inherited_workspace.workspace_path),
            &migration.legacy_task_id,
            &migration.root_task_id,
        );
    }
    let root_task_id = resolved
        .inherited_workspace
        .supervision_root_task_id
        .as_deref()
        .ok_or_else(|| {
            anyhow::anyhow!("Resume inherited workspace is missing root lease identity")
        })?;
    crate::node_agent_supervision_worktree_lease::acquire(
        FsPath::new(&resolved.authorized_workspace_path),
        FsPath::new(&resolved.inherited_workspace.workspace_path),
        root_task_id,
    )
}

fn live_sidecar_occupies_workspace(
    runtime: &NodeRuntime,
    workspace_path: &str,
) -> anyhow::Result<bool> {
    let expected = std::fs::canonicalize(workspace_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(workspace_path));
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
                    .map(|path| {
                        std::fs::canonicalize(path)
                            .unwrap_or_else(|_| std::path::PathBuf::from(path))
                    })
                    .is_some_and(|path| {
                        crate::node_agent_update_checkpoint::same_path(&path, &expected)
                    })
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

#[cfg(test)]
mod snapshot_continue_tests {
    use super::*;

    fn event(
        seq: usize,
        value: serde_json::Value,
    ) -> crate::node_agent_task_journal::TaskJournalEventView {
        crate::node_agent_task_journal::TaskJournalEventView { seq, event: value }
    }

    #[test]
    fn snapshot_continue_evidence_accepts_only_complete_terminal_journal() {
        let events = vec![event(1, json!({"type":"failed"}))];
        validate_snapshot_continue_evidence(&events, 0, false, false).unwrap();
        assert!(
            validate_snapshot_continue_evidence(&events, 1, false, false)
                .unwrap_err()
                .to_string()
                .contains("审批")
        );
        assert!(validate_snapshot_continue_evidence(&events, 0, false, true)
            .unwrap_err()
            .to_string()
            .contains("重复续跑"));
    }

    #[test]
    fn snapshot_continue_evidence_rejects_incomplete_or_irreversible_journal() {
        let incomplete = vec![event(1, json!({"type":"tool_call"}))];
        assert!(validate_snapshot_continue_evidence(&incomplete, 0, false, false).is_err());
        let irreversible = vec![event(
            1,
            json!({"type":"failed", "payload":{"safety":{"non_repeatable_action":"publish"}}}),
        )];
        assert!(
            validate_snapshot_continue_evidence(&irreversible, 0, false, false)
                .unwrap_err()
                .to_string()
                .contains("不可重复")
        );
    }
}
