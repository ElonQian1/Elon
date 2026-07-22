//! Localhost control plane for owner-only offline Codex tasks.

#[path = "node_agent_local_task_cancel.rs"]
mod cancel;
#[path = "node_agent_local_task_create_plan.rs"]
mod create_plan;
#[path = "node_agent_local_task_dispatch.rs"]
mod dispatch;
#[path = "node_agent_local_tasks_idempotency.rs"]
pub(crate) mod idempotency;
#[path = "node_agent_local_task_output_consumer.rs"]
mod output_consumer;
#[path = "node_agent_local_task_provision.rs"]
mod provision;
#[path = "node_agent_local_task_root_workspace.rs"]
pub(crate) mod root_workspace;
#[path = "node_agent_local_tasks_support.rs"]
mod support;
pub(crate) use dispatch::dispatch_local_task_record;
#[cfg(test)]
use output_consumer::durable_completion_for_local_display;
pub(crate) use output_consumer::spawn_local_output_consumer;
pub(crate) use provision::{
    provision_record_and_dispatch_supervised_task, SupervisedLocalTaskProvision,
};
use support::{bound_credentials, internal_error, json_error};

use std::sync::Arc;

use crate::{node_agent_local_task_store::LocalTaskStart, NodeRuntime};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use homecli_proto::CliProjectContext;
use serde::{Deserialize, Serialize};
use serde_json::json;

const MAX_LOCAL_PROMPT_CHARS: usize = 80_000;
const MAX_LOCAL_ID_CHARS: usize = 200;

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DetailQuery {
    since: Option<usize>,
    limit: Option<usize>,
    expected_cursor_epoch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateLocalTaskRequest {
    project_id: String,
    #[serde(default)]
    channel_id: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    workspace_path: String,
    prompt: String,
    #[serde(default)]
    runtime_permission: Option<String>,
    #[serde(default)]
    supervision: Option<crate::node_agent_local_task_supervision::SupervisionContractInput>,
}

#[derive(Debug, Deserialize)]
struct ApprovalDecisionRequest {
    decision: String,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/local-tasks", get(list_tasks).post(create_task))
        .route("/api/local-tasks/:task_id", get(get_task))
        .route(
            "/api/local-tasks/:task_id/cancel",
            post(cancel::cancel_task),
        )
        .route(
            "/api/local-tasks/:task_id/tool-approvals/:approval_id/decision",
            post(decide_approval),
        )
        .merge(crate::node_agent_local_task_supervision::routes())
        .merge(crate::node_agent_self_evolution::routes())
        .merge(crate::node_agent_update_recovery_api::routes())
}

async fn list_tasks(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<ListQuery>,
) -> Response {
    let creds = match bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    match runtime
        .local_tasks
        .list_for_owner(&creds.owner_user_id, query.limit.unwrap_or(50))
    {
        Ok(tasks) => {
            let pending_sync_count = runtime
                .local_tasks
                .pending_count(&creds.owner_user_id)
                .unwrap_or(0);
            Json(json!({
                "ok": true,
                "tasks": tasks,
                "pending_sync_count": pending_sync_count,
                "cloud_connected": runtime.status.read().await.connected,
            }))
            .into_response()
        }
        Err(error) => internal_error(error),
    }
}

async fn get_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Query(query): Query<DetailQuery>,
) -> Response {
    let creds = match bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    let record = match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(record)) => record,
        Ok(None) => {
            return crate::node_agent_local_task_detached_view::response_or_not_found(
                &runtime,
                task_id.trim(),
                query.since.unwrap_or(0),
                query.limit.unwrap_or(200),
                query.expected_cursor_epoch.as_deref(),
            )
            .await
        }
        Err(error) => return internal_error(error),
    };
    let snapshot = match runtime.task_journal.snapshot_with_epoch(
        &record.task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(200),
        query.expected_cursor_epoch.as_deref(),
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => return internal_error(error),
    };
    let task_status = snapshot
        .record
        .as_ref()
        .map(|record| record.status.as_str());
    let active_approval_ids = runtime
        .tool_approvals
        .pending_for_req(&record.task_id)
        .await
        .into_iter()
        .map(|approval| approval.approval_id)
        .collect::<Vec<_>>();
    let approval_state = snapshot.approvals.resolve_runtime_state_for_task_status(
        &active_approval_ids,
        true,
        task_status,
    );
    let mut runtime_status =
        crate::node_agent_task_journal::runtime_status_payload(snapshot.record.as_ref());
    if approval_state.actionable_count > 0 {
        runtime_status["phase"] = serde_json::Value::String("approval".to_string());
    }
    let supervision = match crate::node_agent_local_task_supervision::load_supervision_state(
        &runtime.task_journal,
        &record.task_id,
    ) {
        Ok(supervision) => supervision,
        Err(error) => return internal_error(error),
    };
    let update_recovery = match runtime.update_recovery.receipt_for_task(&record.task_id) {
        Ok(receipt) => receipt,
        Err(error) => return internal_error(error),
    };
    let resume_workspace_status =
        crate::node_agent_local_task_resume_routes::inspect_resume_workspace_status(
            &runtime,
            &record,
            snapshot.record.as_ref(),
            supervision.contract(),
        )
        .await;
    Json(json!({
        "ok": true,
        "record": record,
        "events": snapshot.events,
        "last_event_seq": snapshot.last_event_seq,
        "has_more": snapshot.has_more,
        "cursor_epoch": snapshot.cursor_epoch,
        "requested_cursor_epoch": snapshot.requested_cursor_epoch,
        "previous_cursor_epoch": snapshot.previous_cursor_epoch,
        "cursor_reset": snapshot.cursor_reset,
        "requested_cursor": snapshot.requested_cursor,
        "old_cursor": snapshot.old_cursor,
        "new_cursor": snapshot.new_cursor,
        "resume_cursor": snapshot.resume_cursor,
        "sidecar_update_epoch": snapshot.sidecar_update_epoch,
        "approval_state": approval_state,
        "runtime": runtime_status,
        "supervision": supervision,
        "update_recovery": update_recovery,
        "resume_workspace_status": resume_workspace_status,
    }))
    .into_response()
}

async fn create_task(
    State(runtime): State<Arc<NodeRuntime>>,
    headers: HeaderMap,
    Json(request): Json<CreateLocalTaskRequest>,
) -> Response {
    let creds = match bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    let project_id = request.project_id.trim();
    let workspace_path = request.workspace_path.trim();
    let prompt = request.prompt.trim();
    if project_id.is_empty() || workspace_path.is_empty() || prompt.is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "project_id、workspace_path 和 prompt 均不能为空。",
        );
    }
    if !support::local_identity_is_valid(project_id, MAX_LOCAL_ID_CHARS) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "project_id 最多 200 个字符且不能包含控制字符。",
        );
    }
    for (field, value) in [
        ("conversation_id", request.conversation_id.as_deref()),
        ("channel_id", request.channel_id.as_deref()),
    ] {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        if !support::local_identity_is_valid(value, MAX_LOCAL_ID_CHARS) {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("{field} 最多 200 个字符且不能包含控制字符。"),
            );
        }
    }
    if project_id.eq_ignore_ascii_case("chat") {
        return json_error(
            StatusCode::BAD_REQUEST,
            "本机离线任务必须绑定已授权的真实项目，不能使用保留项目 chat。",
        );
    }
    if !std::path::Path::new(workspace_path).is_absolute() {
        return json_error(StatusCode::BAD_REQUEST, "本机工作目录必须是绝对路径。");
    }
    if prompt.chars().count() > MAX_LOCAL_PROMPT_CHARS {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "本机任务内容过长。");
    }
    let mut supervision = match request.supervision.clone() {
        Some(input) => match crate::node_agent_local_task_supervision::normalize_contract(input) {
            Ok(contract) => Some(contract),
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
        },
        None => None,
    };
    if let Some(contract) = supervision
        .as_ref()
        .filter(|contract| contract.task_role == "resume_original")
    {
        if let Err(error) = crate::node_agent_local_task_resume_context::validate_parent_role_before_resume_side_effects(
            &runtime, &creds, contract,
        ) {
            return json_error(StatusCode::CONFLICT, error.to_string());
        }
    }
    let runtime_permission = request
        .runtime_permission
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("full_access");
    if !matches!(runtime_permission, "full_access" | "danger_full_access") {
        return json_error(
            StatusCode::BAD_REQUEST,
            "PROJECT_FULL_ACCESS_DISABLED: 本机监督任务要求项目启用完全访问。",
        );
    }
    let (task_id, idempotency_binding) =
        match idempotency::begin(&runtime, &creds.owner_user_id, &headers, &request) {
            idempotency::Begin::Unbound { task_id } => (task_id, None),
            idempotency::Begin::Bound(binding) => (binding.task_id.clone(), Some(binding)),
            idempotency::Begin::Response(response) => return response,
        };
    let recovering_idempotent_record = idempotency_binding
        .as_ref()
        .is_some_and(|binding| binding.recover_existing);
    let workspace_path = match root_workspace::resolve_request_workspace(
        &runtime,
        &creds,
        project_id,
        workspace_path,
        supervision.as_ref(),
    ) {
        Ok(path) => path,
        Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
    };
    let workspace_path = workspace_path.as_str();
    if supervision
        .as_ref()
        .is_some_and(|contract| contract.task_role == "post_task_improvement")
    {
        let contract = supervision.expect("post-task improvement contract checked");
        return match crate::node_agent_self_evolution::enqueue(
            &runtime,
            &creds,
            crate::node_agent_self_evolution::SelfEvolutionEnqueue {
                logical_id: task_id.clone(),
                conversation_id: format!("self-evolution-{task_id}"),
                project_id: project_id.to_string(),
                channel_id: request
                    .channel_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
                workspace_path: workspace_path.to_string(),
                prompt: prompt.to_string(),
                runtime_permission: runtime_permission.to_string(),
                contract,
            },
        )
        .await
        {
            Ok(item) => {
                let body = json!({
                    "ok": true,
                    "task_id": item.logical_id,
                    "status": item.status,
                    "sync_state": "local_only",
                    "self_evolution": item,
                });
                if let Err(response) = idempotency::save_state(
                    &runtime,
                    &creds.owner_user_id,
                    idempotency_binding.as_ref(),
                    &json!({
                        "schema": "elon.local_post_self_evolution.v1",
                        "logical_id": task_id,
                        "response_body": body,
                    }),
                ) {
                    return response;
                }
                if let Err(response) = idempotency::complete(
                    &runtime,
                    &creds.owner_user_id,
                    idempotency_binding.as_ref(),
                    StatusCode::ACCEPTED,
                    &body,
                ) {
                    return response;
                }
                (StatusCode::ACCEPTED, Json(body)).into_response()
            }
            Err(error) => json_error(error.status, error.message),
        };
    }
    if let Some(contract) = supervision.as_mut() {
        if contract.parent_task_id.is_none() && contract.root_task_id.is_none() {
            contract.root_task_id = Some(task_id.clone());
        }
    }
    let conversation_id = request
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("offline-{task_id}"));
    let channel_id = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let resume_context_seed = if let Some(contract) = supervision
        .as_mut()
        .filter(|contract| contract.task_role == "resume_original")
    {
        match crate::node_agent_local_task_resume_context::resolve_seed(
            &runtime, &creds, project_id, contract,
        ) {
            Ok(seed) => Some(seed),
            Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
        }
    } else {
        None
    };
    let mut resume_workspace =
        match crate::node_agent_local_task_resume_routes::resolve_supervised_resume_workspace(
            &runtime,
            &creds,
            project_id,
            &conversation_id,
            workspace_path,
            supervision.as_ref(),
        )
        .await
        {
            Ok(workspace) => workspace,
            Err(response) => return response,
        };
    let fresh_workspace = if resume_workspace.is_none() && supervision.is_some() {
        let data_paths = runtime.node_data_root.read().await.paths.clone();
        let supervision_root = supervision
            .as_ref()
            .and_then(|contract| contract.root_task_id.as_deref().or(Some(task_id.as_str())));
        let prepared =
            match crate::node_agent_cli_runner::prepare_cli_prompt_cwd_in_with_supervision(
                data_paths.as_ref(),
                Some(workspace_path.to_string()),
                Some(CliProjectContext {
                    project_id: project_id.to_string(),
                    conversation_id: conversation_id.clone(),
                    runtime_permission: Some(runtime_permission.to_string()),
                }),
                supervision_root,
            ) {
                Ok(prepared) => prepared,
                Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
            };
        let workspace = match prepared.conversation_workspace {
            Some(workspace)
                if workspace.isolated
                    && !crate::node_agent_update_checkpoint::same_path(
                        std::path::Path::new(workspace_path),
                        std::path::Path::new(&workspace.workspace_path),
                    ) =>
            {
                workspace
            }
            _ => {
                return json_error(
                    StatusCode::CONFLICT,
                    "监督写任务未取得独立 worktree，已在派发前拒绝。",
                )
            }
        };
        Some(workspace)
    } else {
        None
    };
    let mut execution_workspace_path = resume_workspace
        .as_ref()
        .map(|workspace| workspace.inherited_workspace.workspace_path.as_str())
        .or_else(|| {
            fresh_workspace
                .as_ref()
                .map(|workspace| workspace.workspace_path.as_str())
        })
        .unwrap_or(workspace_path)
        .to_string();
    let mut workspace_inheritance = resume_workspace.as_ref().map(|workspace| {
        json!({
            "inherited": true,
            "parent_task_id": supervision.as_ref().and_then(|contract| contract.parent_task_id.as_deref()),
            "authorized_workspace_path": workspace.authorized_workspace_path.as_str(),
            "active_workspace_path": workspace.inherited_workspace.workspace_path.as_str(),
            "derivation": workspace.derivation.as_str(),
            "git_head": workspace.git_head.as_str(),
            "lease_migrated_from": workspace.lease_migration.as_ref().map(|migration| migration.legacy_task_id.as_str()),
            "lease_root_task_id": workspace.lease_migration.as_ref().map(|migration| migration.root_task_id.as_str()),
        })
    }).or_else(|| fresh_workspace.as_ref().map(|workspace| json!({
        "inherited": false,
        "isolated": true,
        "authorized_workspace_path": workspace.base_workspace_path.as_deref().unwrap_or(workspace_path),
        "active_workspace_path": workspace.workspace_path.as_str(),
        "branch": workspace.branch.as_deref(),
    })));
    let compiled_resume_context = match resume_context_seed.as_ref() {
        Some(seed) => {
            let Some(workspace) = resume_workspace.as_ref() else {
                return json_error(
                    StatusCode::CONFLICT,
                    "resume context 未取得经过验证的继承工作区。",
                );
            };
            match crate::node_agent_local_task_resume_context::compile(
                seed,
                supervision.as_ref().expect("resume contract exists"),
                workspace,
            ) {
                Ok(context) => Some(context),
                Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
            }
        }
        None => None,
    };
    let mut inherited_authorization_record = resume_context_seed
        .as_ref()
        .map(|seed| seed.inherited_authorization_record());
    let mut record_prompt = compiled_resume_context
        .as_ref()
        .map(|context| context.record_prompt.as_str())
        .unwrap_or(prompt)
        .to_string();
    let mut executor_prompt = compiled_resume_context
        .as_ref()
        .map(|context| context.executor_prompt.clone())
        .unwrap_or_else(|| {
            crate::node_agent_local_task_supervision::executor_prompt(prompt, supervision.as_ref())
        });
    #[cfg(test)]
    let capture_dispatch = tests::should_capture_dispatch(prompt);
    #[cfg(not(test))]
    let capture_dispatch = false;
    let frozen_codex_home = if capture_dispatch {
        None
    } else {
        match crate::node_agent_codex_child_env::FrozenCodexHome::capture_unmanaged_for_local_task()
        {
            Ok(home) => Some(home),
            Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
        }
    };
    if !capture_dispatch {
        if let Err(error) = runtime.resolve_cli("codex").await {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("本机 Codex CLI 不可用：{error}"),
            );
        }
    }

    let computed_inherited_workspace = resume_workspace
        .as_ref()
        .map(|workspace| workspace.inherited_workspace.clone())
        .or_else(|| fresh_workspace.clone());
    let computed_plan = create_plan::DurableCreatePlan::prepared(create_plan::PreparedPlan {
        task_id: &task_id,
        supervision: supervision.clone(),
        conversation_id: &conversation_id,
        channel_id: channel_id.clone(),
        resolved_workspace_path: workspace_path,
        execution_workspace_path: &execution_workspace_path,
        record_prompt: &record_prompt,
        executor_prompt: &executor_prompt,
        inherited_workspace: computed_inherited_workspace,
        inherited_authorization_record: inherited_authorization_record.as_ref(),
        workspace_inheritance: workspace_inheritance.clone(),
        resume_context: compiled_resume_context.as_ref(),
    });
    let mut durable_plan = match create_plan::DurableCreatePlan::persist_or_recover(
        &runtime,
        &creds.owner_user_id,
        idempotency_binding.as_ref(),
        computed_plan,
    ) {
        Ok(plan) => plan,
        Err(response) => return response,
    };
    execution_workspace_path = durable_plan.execution_workspace_path.clone();
    record_prompt = durable_plan.record_prompt.clone();
    executor_prompt = durable_plan.executor_prompt.clone();
    workspace_inheritance = durable_plan.workspace_inheritance.clone();
    if let Some(parent_task_id) = durable_plan.inherited_authorization_task_id.as_deref() {
        inherited_authorization_record = match runtime
            .local_tasks
            .get_for_owner(&creds.owner_user_id, parent_task_id)
        {
            Ok(Some(record)) => Some(record),
            Ok(None) => {
                return json_error(
                    StatusCode::CONFLICT,
                    "Resume 持久授权父任务已不可读取，拒绝恢复派发。",
                )
            }
            Err(error) => return internal_error(error),
        };
    }

    if fresh_workspace.is_none() && !recovering_idempotent_record {
        if let Some(contract) = supervision.as_ref() {
            if let Err(error) = crate::node_agent_local_task_supervision::record_supervision_event(
                &runtime.task_journal,
                &task_id,
                "supervision_contract",
                crate::node_agent_local_task_supervision::contract_payload(contract),
            ) {
                return internal_error(error);
            }
        }
        if let Some(context) = durable_plan.resume_context_journal.as_ref() {
            if let Err(error) = crate::node_agent_local_task_supervision::record_supervision_event(
                &runtime.task_journal,
                &task_id,
                "resume_context",
                context.clone(),
            ) {
                return internal_error(error);
            }
        }
    }
    let recovered_idempotent_record = idempotency_binding
        .as_ref()
        .filter(|binding| binding.recover_existing)
        .and_then(|binding| {
            runtime
                .local_tasks
                .get_for_owner(&creds.owner_user_id, &binding.task_id)
                .ok()
                .flatten()
        });
    let record = if let Some(record) = recovered_idempotent_record {
        if record.workspace_path != durable_plan.execution_workspace_path
            || record.prompt != durable_plan.record_prompt
            || record.project_id != project_id
            || record.conversation_id != durable_plan.conversation_id
            || record.runtime_permission != runtime_permission
        {
            return json_error(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_DURABLE_RECORD_DRIFT: 已持久任务与 dispatch 计划身份不一致。",
            );
        }
        record
    } else if let (Some(workspace), Some(contract)) =
        (fresh_workspace.as_ref(), supervision.as_ref())
    {
        let request = SupervisedLocalTaskProvision {
            task_id: &task_id,
            owner_user_id: &creds.owner_user_id,
            agent_id: &creds.agent_id,
            install_id: &runtime.install_id,
            project_id,
            channel_id: channel_id.as_deref(),
            conversation_id: &conversation_id,
            base_workspace_path: workspace_path,
            prompt: &record_prompt,
            runtime_permission,
            root_task_id: contract.root_task_id.as_deref().unwrap_or(&task_id),
            contract,
        };
        match provision::record_provisioned_supervised_task(
            &runtime.local_tasks,
            &runtime.task_journal,
            workspace,
            &request,
        ) {
            Ok(record) => record,
            Err(error) => return internal_error(error),
        }
    } else {
        match provision::record_local_or_resumed_task(
            &runtime.local_tasks,
            LocalTaskStart {
                task_id: &task_id,
                owner_user_id: &creds.owner_user_id,
                agent_id: &creds.agent_id,
                install_id: &runtime.install_id,
                project_id,
                channel_id: channel_id.as_deref(),
                conversation_id: &conversation_id,
                workspace_path: &execution_workspace_path,
                prompt: &record_prompt,
                cli: "codex",
                runtime_permission,
            },
            resume_workspace.as_ref(),
            supervision.as_ref(),
        ) {
            Ok(record) => record,
            Err(error) => return internal_error(error),
        }
    };
    let resume_admission = resume_workspace
        .as_mut()
        .and_then(|workspace| workspace.resume_admission.take());
    let inherited_workspace = durable_plan.inherited_workspace.clone();
    let response_body = json!({
        "ok": true,
        "task_id": task_id,
        "status": "running",
        "sync_state": "local_only",
        "record": record,
        "supervision": durable_plan.supervision,
        "workspace_inheritance": workspace_inheritance,
        "resume_context": durable_plan.resume_context_response,
    });
    if let Err(response) = durable_plan.persist_response(
        &runtime,
        &creds.owner_user_id,
        idempotency_binding.as_ref(),
        &response_body,
    ) {
        return response;
    }
    #[cfg(test)]
    if capture_dispatch {
        tests::record_captured_dispatch(
            prompt,
            &executor_prompt,
            &execution_workspace_path,
            supervision.as_ref(),
            inherited_authorization_record.as_ref(),
        );
    }
    if !capture_dispatch {
        dispatch_local_task_record(
            runtime.clone(),
            &record,
            executor_prompt,
            execution_workspace_path,
            supervision.as_ref(),
            inherited_workspace,
            resume_admission,
            inherited_authorization_record,
            frozen_codex_home.expect("non-test dispatch captured CODEX_HOME"),
        );
    }
    if let Err(response) = idempotency::complete(
        &runtime,
        &creds.owner_user_id,
        idempotency_binding.as_ref(),
        StatusCode::ACCEPTED,
        &response_body,
    ) {
        return response;
    }
    (StatusCode::ACCEPTED, Json(response_body)).into_response()
}

async fn decide_approval(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((task_id, approval_id)): Path<(String, String)>,
    Json(request): Json<ApprovalDecisionRequest>,
) -> Response {
    let creds = match bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(record)) if record.status == "running" => {}
        Ok(Some(_)) => return json_error(StatusCode::CONFLICT, "任务已结束，审批不再可操作。"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return internal_error(error),
    }
    let decision = match request.decision.trim() {
        "approve" => "approve",
        "deny" => "deny",
        _ => return json_error(StatusCode::BAD_REQUEST, "decision 只能是 approve 或 deny。"),
    };
    if !runtime
        .decide_tool_approval(task_id.trim(), approval_id.trim(), decision)
        .await
    {
        return json_error(
            StatusCode::CONFLICT,
            "审批已失效，或运行时已不存在对应等待项。",
        );
    }
    Json(json!({
        "ok": true,
        "task_id": task_id,
        "approval_id": approval_id,
        "decision": decision,
    }))
    .into_response()
}

#[cfg(test)]
#[path = "node_agent_local_tasks_tests.rs"]
mod tests;
