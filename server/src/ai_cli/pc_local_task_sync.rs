//! Cloud admission for durable snapshots emitted by the node-local task plane.

use homecli_proto::{CliCompletionEnvelope, CliLocalTaskSnapshot, ServerToAgent};

use super::{
    support::{validate_authenticated_producer, validate_local_offline_scope},
    ReplayFailure, LOCAL_OFFLINE_ORIGIN,
};
use crate::{
    store::{PcLocalTaskStartApply, ProjectExecutionSessionStart},
    types::AppState,
};

pub(crate) fn handle(
    state: &AppState,
    authenticated_node_id: &str,
    authenticated_owner_user_id: Option<&str>,
    authenticated_install_id: Option<&str>,
    snapshot: CliLocalTaskSnapshot,
) -> ServerToAgent {
    let task_id = snapshot.task_id.clone();
    let revision = snapshot.revision.clone();
    match apply(
        state,
        authenticated_node_id,
        authenticated_owner_user_id,
        authenticated_install_id,
        &snapshot,
    ) {
        Ok(cloud_task_id) => ack(task_id, revision, true, false, Some(cloud_task_id), None),
        Err(ReplayFailure::Retry(error)) => ack(task_id, revision, false, true, None, Some(error)),
        Err(ReplayFailure::Reject(error)) => {
            ack(task_id, revision, false, false, None, Some(error))
        }
    }
}

fn apply(
    state: &AppState,
    authenticated_node_id: &str,
    authenticated_owner_user_id: Option<&str>,
    authenticated_install_id: Option<&str>,
    snapshot: &CliLocalTaskSnapshot,
) -> Result<String, ReplayFailure> {
    validate_snapshot(snapshot)?;
    let scope_envelope = scope_envelope(snapshot);
    validate_authenticated_producer(
        authenticated_node_id,
        authenticated_owner_user_id,
        authenticated_install_id,
        &scope_envelope,
    )?;
    let owner_user_id = snapshot.producer_identity.owner_user_id.as_str();
    let channel_id =
        validate_local_offline_scope(state, authenticated_node_id, owner_user_id, &scope_envelope)?;

    let session_bound = state
        .store
        .record_project_execution_started(ProjectExecutionSessionStart {
            project_id: &snapshot.project_context.project_id,
            conversation_id: &snapshot.project_context.conversation_id,
            user_id: owner_user_id,
            node_id: authenticated_node_id,
            request_id: &snapshot.task_id,
            requested_workspace_path: Some(&snapshot.workspace_path),
            model: Some(&snapshot.cli),
        })
        .map_err(ReplayFailure::retry)?;
    if !session_bound {
        return Err(ReplayFailure::reject("本机任务同步的项目执行会话身份冲突"));
    }

    let outcome = state
        .store
        .apply_pc_local_task_start(PcLocalTaskStartApply {
            request_id: &snapshot.task_id,
            revision: &snapshot.revision,
            project_id: &snapshot.project_context.project_id,
            channel_id: &channel_id,
            conversation_id: &snapshot.project_context.conversation_id,
            user_id: owner_user_id,
            node_id: authenticated_node_id,
            prompt: &snapshot.prompt,
            workspace_path: &snapshot.workspace_path,
            cli: &snapshot.cli,
            status: &snapshot.status,
            codex_session_id: snapshot.session_id.as_deref(),
        })
        .map_err(ReplayFailure::retry)?;
    let task_bound = state
        .store
        .bind_project_execution_task_id(&snapshot.task_id, &outcome.task_id)
        .map_err(ReplayFailure::retry)?;
    if !task_bound {
        return Err(ReplayFailure::reject(
            "本机任务同步不能改写既有项目执行 task_id 绑定",
        ));
    }

    if let Some(session_id) = snapshot
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        state
            .store
            .upsert_native_agent_session(
                &snapshot.project_context.project_id,
                owner_user_id,
                Some(&snapshot.project_context.conversation_id),
                "codex",
                authenticated_node_id,
                &snapshot.workspace_path,
                session_id,
            )
            .map_err(ReplayFailure::retry)?;
    }

    if outcome.changed {
        crate::project_space::publish_channel_message_updated(
            state,
            &outcome.project_id,
            &outcome.channel_id,
            Some(&outcome.conversation_id),
            Some(&outcome.task_id),
            "ai_task",
        );
    }
    Ok(outcome.task_id)
}

fn validate_snapshot(snapshot: &CliLocalTaskSnapshot) -> Result<(), ReplayFailure> {
    for (name, value, max) in [
        ("task_id", snapshot.task_id.as_str(), 200_usize),
        ("revision", snapshot.revision.as_str(), 500),
        ("prompt", snapshot.prompt.as_str(), 80_000),
        ("workspace_path", snapshot.workspace_path.as_str(), 4_096),
        ("status", snapshot.status.as_str(), 100),
    ] {
        let len = value.chars().count();
        let invalid_control = value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'));
        if value.trim().is_empty() || len > max || invalid_control {
            return Err(ReplayFailure::reject(format!(
                "本机任务同步字段 {name} 无效"
            )));
        }
    }
    if !snapshot.cli.to_ascii_lowercase().contains("codex") {
        return Err(ReplayFailure::reject("本机任务同步只接受 Codex 任务"));
    }
    if snapshot.updated_at_ms < snapshot.started_at_ms {
        return Err(ReplayFailure::reject("本机任务同步时间顺序无效"));
    }
    if snapshot
        .session_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty() || value.len() > 256)
    {
        return Err(ReplayFailure::reject("Codex 本机会话 ID 无效"));
    }
    Ok(())
}

fn scope_envelope(snapshot: &CliLocalTaskSnapshot) -> CliCompletionEnvelope {
    CliCompletionEnvelope {
        event_id: format!("local-task-sync:{}", snapshot.task_id),
        req_id: snapshot.task_id.clone(),
        cli: snapshot.cli.clone(),
        origin: LOCAL_OFFLINE_ORIGIN.to_string(),
        producer_identity: Some(snapshot.producer_identity.clone()),
        project_context: Some(snapshot.project_context.clone()),
        channel_id: snapshot.channel_id.clone(),
        prompt: Some(snapshot.prompt.clone()),
        final_output: String::new(),
        exit_ok: false,
        error: None,
        session_id: snapshot.session_id.clone(),
        prompt_tokens: None,
        cached_input_tokens: None,
        completion_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        model: None,
        workspace_status: None,
        created_at_ms: snapshot.updated_at_ms,
    }
}

fn ack(
    task_id: String,
    revision: String,
    accepted: bool,
    retryable: bool,
    cloud_task_id: Option<String>,
    error: Option<String>,
) -> ServerToAgent {
    ServerToAgent::CliLocalTaskSyncAck {
        task_id,
        revision,
        accepted,
        retryable,
        cloud_task_id,
        error,
    }
}
