//! Localhost control plane for owner-only offline Codex tasks.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use homecli_proto::{
    AgentToServer, CancelRequestAudit, CliCompletionEnvelope, CliCompletionProducerIdentity,
    CliProjectContext,
};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::{
    node_agent_cli_task_dispatch::{spawn_cli_task, CliTaskDispatchRequest},
    node_agent_local_task_store::LocalTaskStart,
    NodeRuntime,
};

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
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Default, Deserialize)]
struct CancelLocalTaskRequest {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    requested_at_ms: Option<u128>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/local-tasks", get(list_tasks).post(create_task))
        .route("/api/local-tasks/:task_id", get(get_task))
        .route("/api/local-tasks/:task_id/cancel", post(cancel_task))
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
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return internal_error(error),
    };
    let snapshot = match runtime.task_journal.snapshot(
        &record.task_id,
        query.since.unwrap_or(0),
        query.limit.unwrap_or(200),
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
    if !local_identity_is_valid(project_id) {
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
        if !local_identity_is_valid(value) {
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
    let supervision = match request.supervision {
        Some(input) => match crate::node_agent_local_task_supervision::normalize_contract(input) {
            Ok(contract) => Some(contract),
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
        },
        None => None,
    };
    let executor_prompt =
        crate::node_agent_local_task_supervision::executor_prompt(prompt, supervision.as_ref());
    let runtime_permission = request
        .runtime_permission
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("full_access");
    if runtime_permission != "full_access" {
        return json_error(
            StatusCode::BAD_REQUEST,
            "离线本机任务首版只允许已显式授权的 full_access 工作目录。",
        );
    }
    if supervision
        .as_ref()
        .is_some_and(|contract| contract.task_role == "post_task_improvement")
    {
        let contract = supervision.expect("post-task improvement contract checked");
        return match crate::node_agent_self_evolution::enqueue(
            &runtime,
            &creds,
            crate::node_agent_self_evolution::SelfEvolutionEnqueue {
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
            Ok(item) => (
                StatusCode::ACCEPTED,
                Json(json!({
                    "ok": true,
                    "task_id": item.logical_id,
                    "status": item.status,
                    "sync_state": "local_only",
                    "self_evolution": item,
                })),
            )
                .into_response(),
            Err(error) => json_error(error.status, error.message),
        };
    }
    let mut resume_workspace =
        match crate::node_agent_local_task_resume_routes::resolve_supervised_resume_workspace(
            &runtime,
            &creds,
            project_id,
            workspace_path,
            supervision.as_ref(),
        )
        .await
        {
            Ok(workspace) => workspace,
            Err(response) => return response,
        };
    let record_workspace_path = resume_workspace
        .as_ref()
        .map(|workspace| workspace.authorized_workspace_path.as_str())
        .unwrap_or(workspace_path)
        .to_string();
    let execution_workspace_path = resume_workspace
        .as_ref()
        .map(|workspace| workspace.inherited_workspace.workspace_path.as_str())
        .unwrap_or(workspace_path)
        .to_string();
    let workspace_inheritance = resume_workspace.as_ref().map(|workspace| {
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
    });
    let frozen_codex_home =
        match crate::node_agent_codex_child_env::FrozenCodexHome::capture_unmanaged_for_local_task()
        {
            Ok(home) => home,
            Err(error) => return json_error(StatusCode::CONFLICT, error.to_string()),
        };
    if let Err(error) = runtime.resolve_cli("codex").await {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            format!("本机 Codex CLI 不可用：{error}"),
        );
    }

    let task_id = format!("local-{}", Uuid::new_v4());
    let conversation_id = request
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("offline-{}", Uuid::new_v4()));
    let channel_id = request
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
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
    let record = match runtime.local_tasks.create(LocalTaskStart {
        task_id: &task_id,
        owner_user_id: &creds.owner_user_id,
        agent_id: &creds.agent_id,
        install_id: &runtime.install_id,
        project_id,
        channel_id: channel_id.as_deref(),
        conversation_id: &conversation_id,
        workspace_path: &record_workspace_path,
        prompt,
        cli: "codex",
        runtime_permission,
    }) {
        Ok(record) => record,
        Err(error) => return internal_error(error),
    };
    let resume_admission = resume_workspace
        .as_mut()
        .and_then(|workspace| workspace.resume_admission.take());
    let inherited_workspace = resume_workspace.map(|workspace| workspace.inherited_workspace);
    dispatch_local_task_record(
        runtime.clone(),
        &record,
        executor_prompt,
        execution_workspace_path,
        supervision.as_ref(),
        inherited_workspace,
        resume_admission,
        frozen_codex_home,
    );
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "ok": true,
            "task_id": task_id,
            "status": "running",
            "sync_state": "local_only",
            "record": record,
            "supervision": supervision,
            "workspace_inheritance": workspace_inheritance,
        })),
    )
        .into_response()
}

pub(crate) fn dispatch_local_task_record(
    runtime: Arc<NodeRuntime>,
    record: &crate::node_agent_local_task_store::LocalTaskRecord,
    executor_prompt: String,
    execution_workspace_path: String,
    supervision: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    inherited_workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
    resume_admission: Option<crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>,
    frozen_codex_home: crate::node_agent_codex_child_env::FrozenCodexHome,
) {
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    spawn_local_output_consumer(
        runtime.clone(),
        record.owner_user_id.clone(),
        record.task_id.clone(),
        out_rx,
    );
    spawn_cli_task(
        runtime.clone(),
        out_tx,
        CliTaskDispatchRequest {
            req_id: record.task_id.clone(),
            cli: "codex".to_string(),
            extra_args: Vec::new(),
            cwd: Some(execution_workspace_path),
            project_context: Some(CliProjectContext {
                project_id: record.project_id.clone(),
                conversation_id: record.conversation_id.clone(),
                runtime_permission: Some(record.runtime_permission.clone()),
            }),
            codex_credential_binding: None,
            requires_cloud_control: false,
            cloud_control_deadline: None,
            cloud_control_issued_at: None,
            cloud_control_ttl_ms: None,
            prompt: executor_prompt,
            completion_context: crate::node_agent_cli_done::CliCompletionContext::local_offline(
                CliCompletionProducerIdentity {
                    owner_user_id: record.owner_user_id.clone(),
                    agent_id: record.agent_id.clone(),
                    install_id: record.install_id.clone(),
                },
                CliProjectContext {
                    project_id: record.project_id.clone(),
                    conversation_id: record.conversation_id.clone(),
                    runtime_permission: Some(record.runtime_permission.clone()),
                },
                record.channel_id.clone(),
                record.prompt.clone(),
                supervision.map(|contract| contract.protocol.clone()),
            ),
            inherited_workspace,
            resume_admission,
            allow_codex_auth_switch: false,
            frozen_codex_home: Some(frozen_codex_home),
        },
    );
}

async fn cancel_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    request: Option<Json<CancelLocalTaskRequest>>,
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
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return internal_error(error),
    };
    if record.status != "running" {
        return Json(json!({
            "ok": true,
            "task_id": record.task_id,
            "status": record.status,
            "message": "任务已经结束，无需重复停止。",
        }))
        .into_response();
    }
    let request = request.map(|Json(value)| value).unwrap_or_default();
    let mut audit = CancelRequestAudit::now(
        creds.owner_user_id.clone(),
        request.source.as_deref().unwrap_or("pc_ui"),
        request.reason.as_deref().unwrap_or("user_requested"),
    );
    if request.requested_at_ms.is_some() {
        audit.requested_at_ms = request.requested_at_ms;
    }
    if !runtime
        .cancel_cli_prompt_with_audit(&record.task_id, &audit)
        .await
    {
        return json_error(
            StatusCode::CONFLICT,
            "任务记录仍在运行，但当前进程没有可停止的控制句柄。",
        );
    }
    let _ = runtime
        .local_tasks
        .mark_canceled(&creds.owner_user_id, &record.task_id);
    Json(json!({
        "ok": true,
        "task_id": record.task_id,
        "status": "cancel_requested",
        "cancel": audit,
    }))
    .into_response()
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

pub(crate) fn spawn_local_output_consumer(
    runtime: Arc<NodeRuntime>,
    owner_user_id: String,
    task_id: String,
    mut out_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
    tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            let Message::Text(text) = message else {
                continue;
            };
            let Ok(message) = serde_json::from_str::<AgentToServer>(&text) else {
                continue;
            };
            match message {
                AgentToServer::CliDone { req_id, .. } if req_id == task_id => {
                    let completion = match durable_completion_for_local_display(
                        &runtime.completion_outbox,
                        &req_id,
                    ) {
                        Ok(Some(completion)) => completion,
                        Ok(None) => {
                            tracing::warn!(
                                %task_id,
                                "received local CliDone without durable outbox row; leaving terminal state to durable producer/startup repair"
                            );
                            break;
                        }
                        Err(error) => {
                            tracing::warn!(%task_id, %error, "failed to read durable local completion");
                            break;
                        }
                    };
                    if let Err(error) = runtime.local_tasks.finish(&owner_user_id, &completion) {
                        tracing::warn!(%task_id, %error, "failed to persist local task completion");
                    }
                    break;
                }
                _ => {}
            }
        }
    });
}

fn durable_completion_for_local_display(
    outbox: &crate::node_agent_completion_outbox::CliCompletionOutbox,
    req_id: &str,
) -> anyhow::Result<Option<CliCompletionEnvelope>> {
    outbox.latest_for_req_id(req_id)
}

async fn bound_credentials(runtime: &Arc<NodeRuntime>) -> Result<crate::Credentials, Response> {
    runtime.creds().await.ok_or_else(|| {
        json_error(
            StatusCode::UNAUTHORIZED,
            "本机节点尚未绑定账号，不能创建或读取离线任务。",
        )
    })
}

fn local_identity_is_valid(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.chars().count() <= MAX_LOCAL_ID_CHARS
        && !value.chars().any(char::is_control)
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
mod tests {
    use super::{durable_completion_for_local_display, local_identity_is_valid};

    #[test]
    fn missing_durable_completion_never_synthesizes_local_terminal_event() {
        let path = std::env::temp_dir().join(format!(
            "elon-local-missing-outbox-{}.sqlite3",
            uuid::Uuid::new_v4().simple()
        ));
        let outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(path.clone());

        assert!(
            durable_completion_for_local_display(&outbox, "local-missing")
                .unwrap()
                .is_none()
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn local_identity_matches_server_replay_bounds() {
        assert!(local_identity_is_valid("project-1"));
        assert!(local_identity_is_valid(&"x".repeat(200)));
        assert!(!local_identity_is_valid(""));
        assert!(!local_identity_is_valid(&"x".repeat(201)));
        assert!(!local_identity_is_valid("project\nother"));
    }
}
