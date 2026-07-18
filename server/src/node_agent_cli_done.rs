use anyhow::{bail, Context, Result};
use homecli_proto::{
    AgentToServer, CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext,
    CliWorkspaceStatus,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::{
    cli_usage, node_agent_codex_session, node_agent_task_journal,
    node_agent_task_journal_events::completion_terminal_status, NodeRuntime,
};

// Worst-case UTF-8 remains below the server inbox 1 MiB payload ceiling, leaving
// room for workspace/error/usage metadata.
const MAX_COMPLETION_OUTPUT_CHARS: usize = 200_000;
// Keep the persisted envelope comfortably below the server's 1 MiB receipt
// ceiling. JSON escaping can expand control characters far beyond UTF-8 size,
// so the final check must use the actual serialized representation.
const MAX_DURABLE_COMPLETION_PAYLOAD_BYTES: usize = 900_000;
const COMPLETION_TRUNCATION_MARKER: &str = "\n[内容过长，已由本机节点截断后同步]";

#[derive(Clone, Debug)]
pub(crate) struct CliCompletionContext {
    pub origin: String,
    pub producer_identity: CliCompletionProducerIdentity,
    pub local_owner_user_id: Option<String>,
    pub supervision_protocol: Option<String>,
    pub project_context: Option<CliProjectContext>,
    pub channel_id: Option<String>,
    pub prompt: Option<String>,
}

impl CliCompletionContext {
    pub(crate) fn cloud(
        producer_identity: CliCompletionProducerIdentity,
        project_context: Option<CliProjectContext>,
    ) -> Self {
        Self {
            origin: "cloud_dispatch".to_string(),
            producer_identity,
            local_owner_user_id: None,
            supervision_protocol: None,
            project_context,
            channel_id: None,
            prompt: None,
        }
    }

    pub(crate) fn local_offline(
        producer_identity: CliCompletionProducerIdentity,
        project_context: CliProjectContext,
        channel_id: Option<String>,
        prompt: String,
        supervision_protocol: Option<String>,
    ) -> Self {
        let owner_user_id = producer_identity.owner_user_id.clone();
        Self {
            origin: "local_offline".to_string(),
            producer_identity,
            local_owner_user_id: Some(owner_user_id),
            supervision_protocol,
            project_context: Some(project_context),
            channel_id,
            prompt: Some(prompt),
        }
    }

    pub(crate) fn is_desktop_supervised(&self) -> bool {
        self.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN
            && self.supervision_protocol.as_deref()
                == Some(crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL)
    }
}

pub(crate) fn cli_prompt_accepted(
    req_id: String,
    cli: Option<String>,
    cwd: Option<String>,
    runtime_permission: Option<String>,
) -> AgentToServer {
    AgentToServer::CliPromptAccepted {
        req_id,
        cli,
        cwd,
        runtime_permission,
    }
}

pub(crate) fn cli_done_message(
    req_id: String,
    exit_ok: bool,
    error: Option<String>,
    usage: Option<cli_usage::CliTokenUsage>,
    model: Option<String>,
    workspace_status: Option<CliWorkspaceStatus>,
    session_id: Option<String>,
) -> AgentToServer {
    let usage = usage.and_then(cli_usage::CliTokenUsage::normalized);
    AgentToServer::CliDone {
        req_id,
        exit_ok,
        error,
        session_id,
        prompt_tokens: usage.as_ref().map(|u| u.input_tokens.max(0) as u64),
        cached_input_tokens: usage.as_ref().map(|u| u.cached_input_tokens.max(0) as u64),
        completion_tokens: usage.as_ref().map(|u| u.output_tokens.max(0) as u64),
        reasoning_tokens: usage.as_ref().map(|u| u.reasoning_tokens.max(0) as u64),
        total_tokens: usage.as_ref().map(|u| u.total_tokens.max(0) as u64),
        model,
        workspace_status,
    }
}

/// Build a terminal message from every byte already collected from the child.
/// Cancellation and timeout paths must use the same parser as normal exit: a
/// failed/canceled process may still have completed billable model work before
/// it was stopped.
pub(crate) fn cli_done_message_from_output(
    req_id: String,
    exit_ok: bool,
    error: Option<String>,
    stdout_text: &str,
    stderr_text: &str,
    fallback_model: Option<String>,
    workspace_status: Option<CliWorkspaceStatus>,
    session_id: Option<String>,
) -> (AgentToServer, String) {
    let combined_output = format!("{stdout_text}\n{stderr_text}");
    let usage = cli_usage::parse_cli_usage(&combined_output);
    let model = usage
        .as_ref()
        .and_then(|usage| usage.model.clone())
        .or(fallback_model);
    (
        cli_done_message(
            req_id,
            exit_ok,
            error,
            usage,
            model,
            workspace_status,
            session_id,
        ),
        combined_output,
    )
}

/// Persist the exact terminal payload before exposing it to either the live cloud
/// socket or the local workbench consumer. A failed outbox transaction is fatal to
/// delivery: sending an unjournaled terminal event would recreate the result/token
/// loss this path is designed to prevent.
pub(crate) fn persist_and_send_cli_done(
    runtime: &NodeRuntime,
    context: &CliCompletionContext,
    cli_name: &str,
    final_output: Option<&str>,
    message: AgentToServer,
    out_tx: &mpsc::UnboundedSender<Message>,
) -> Result<CliCompletionEnvelope> {
    let AgentToServer::CliDone {
        req_id,
        exit_ok,
        error,
        session_id,
        prompt_tokens,
        cached_input_tokens,
        completion_tokens,
        reasoning_tokens,
        total_tokens,
        model,
        workspace_status,
    } = &message
    else {
        bail!("durable CLI completion helper requires CliDone");
    };
    let final_output = final_output
        .map(str::to_string)
        .or_else(|| {
            runtime
                .task_journal
                .completion_output(req_id, MAX_COMPLETION_OUTPUT_CHARS)
                .ok()
        })
        .unwrap_or_default();
    let mut envelope = CliCompletionEnvelope {
        event_id: Uuid::new_v4().to_string(),
        req_id: req_id.clone(),
        cli: cli_name.to_string(),
        origin: context.origin.clone(),
        producer_identity: Some(context.producer_identity.clone()),
        project_context: context.project_context.clone(),
        channel_id: context.channel_id.clone(),
        prompt: context.prompt.clone(),
        final_output: truncate_chars(&final_output, MAX_COMPLETION_OUTPUT_CHARS),
        exit_ok: *exit_ok,
        error: error.clone(),
        session_id: session_id.clone(),
        prompt_tokens: *prompt_tokens,
        cached_input_tokens: *cached_input_tokens,
        completion_tokens: *completion_tokens,
        reasoning_tokens: *reasoning_tokens,
        total_tokens: *total_tokens,
        model: model.clone(),
        workspace_status: workspace_status.clone(),
        created_at_ms: now_ms(),
    };
    compact_completion_payload(&mut envelope)?;
    runtime
        .completion_outbox
        .enqueue(&envelope)
        .with_context(|| format!("persist CLI completion {}", envelope.event_id))?;
    if context.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN {
        match context.local_owner_user_id.as_deref() {
            Some(owner_user_id) => match runtime.local_tasks.finish(owner_user_id, &envelope) {
                Ok(true) => {}
                Ok(false) => tracing::warn!(
                    %req_id,
                    event_id = %envelope.event_id,
                    "durable local completion did not match a local task row"
                ),
                Err(error) => tracing::warn!(
                    %req_id,
                    event_id = %envelope.event_id,
                    %error,
                    "failed to bind durable completion to local task; startup/ACK will retry"
                ),
            },
            None => tracing::error!(
                %req_id,
                event_id = %envelope.event_id,
                "local completion context is missing its owner binding"
            ),
        }
    }
    if let Err(error) = runtime.task_journal.record_finished_with_outcome(
        req_id,
        completion_terminal_status(*exit_ok, error.as_deref()),
        error.as_deref(),
    ) {
        tracing::warn!(%req_id, %error, "failed to update display task journal after durable outbox commit");
    }
    let text = serde_json::to_string(&message).context("serialize durable CliDone")?;
    out_tx
        .send(Message::Text(text))
        .map_err(|_| anyhow::anyhow!("CLI completion consumer closed"))?;
    Ok(envelope)
}

pub(crate) fn latest_codex_session_id(
    cli_name: &str,
    codex_plan: &node_agent_codex_session::CodexSessionPlan,
    task_journal: &node_agent_task_journal::TaskJournal,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }
    codex_plan
        .scope_key
        .as_deref()
        .and_then(|key| task_journal.load_codex_session(key).ok().flatten())
        .or_else(|| codex_plan.session_id.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(1)
        .max(1)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn compact_completion_payload(envelope: &mut CliCompletionEnvelope) -> Result<()> {
    for _ in 0..64 {
        let serialized_len = serde_json::to_vec(envelope)
            .context("计算 durable CLI completion 大小")?
            .len();
        if serialized_len <= MAX_DURABLE_COMPLETION_PAYLOAD_BYTES {
            return Ok(());
        }

        let output_chars = envelope.final_output.chars().count();
        let prompt_chars = envelope
            .prompt
            .as_deref()
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0);
        let output_cost = escaped_json_string_len(&envelope.final_output);
        let prompt_cost = envelope
            .prompt
            .as_deref()
            .map(escaped_json_string_len)
            .unwrap_or(0);

        if output_chars > 256 && (output_cost >= prompt_cost || prompt_chars <= 1) {
            envelope.final_output =
                truncate_with_marker(&envelope.final_output, (output_chars / 2).max(256));
            continue;
        }
        if prompt_chars > 1 {
            if let Some(prompt) = envelope.prompt.as_deref() {
                envelope.prompt = Some(truncate_with_marker(prompt, (prompt_chars / 2).max(1)));
                continue;
            }
        }
        if output_chars > 0 {
            envelope.final_output.clear();
            continue;
        }
        if envelope.error.is_some() {
            envelope.error = None;
            continue;
        }
        if let Some(workspace) = envelope.workspace_status.as_mut() {
            if workspace.merge_message.take().is_some() {
                continue;
            }
            if workspace.base_workspace_path.take().is_some() {
                continue;
            }
            if workspace.branch.take().is_some() {
                continue;
            }
            if workspace.active_workspace_path.chars().count() > 256 {
                workspace.active_workspace_path = truncate_with_marker(
                    &workspace.active_workspace_path,
                    workspace.active_workspace_path.chars().count() / 2,
                );
                continue;
            }
        }
        bail!(
            "durable CLI completion metadata exceeds {} bytes after safe compaction",
            MAX_DURABLE_COMPLETION_PAYLOAD_BYTES
        );
    }
    bail!("durable CLI completion payload compaction did not converge")
}

fn escaped_json_string_len(value: &str) -> usize {
    serde_json::to_string(value)
        .map(|json| json.len())
        .unwrap_or(usize::MAX)
}

fn truncate_with_marker(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let marker_chars = COMPLETION_TRUNCATION_MARKER.chars().count();
    if count <= marker_chars + 1 {
        return value
            .chars()
            .take(max_chars.max(1).min(count - 1))
            .collect();
    }
    // Always make strict progress even when the requested cap is smaller than
    // the marker itself; otherwise repeated adaptive compaction can oscillate.
    let keep_chars = max_chars.min(count - marker_chars - 1);
    let mut truncated: String = value.chars().take(keep_chars).collect();
    truncated.push_str(COMPLETION_TRUNCATION_MARKER);
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion(output: String, prompt: Option<String>) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: "event-payload".to_string(),
            req_id: "request-payload".to_string(),
            cli: "codex".to_string(),
            origin: if prompt.is_some() {
                "local_offline".to_string()
            } else {
                "cloud_dispatch".to_string()
            },
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: "owner-a".to_string(),
                agent_id: "node-a".to_string(),
                install_id: "install-a".to_string(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "project-a".to_string(),
                conversation_id: "conversation-a".to_string(),
                runtime_permission: Some("full_access".to_string()),
            }),
            channel_id: None,
            prompt,
            final_output: output,
            exit_ok: true,
            error: None,
            session_id: Some("session-a".to_string()),
            prompt_tokens: Some(10),
            cached_input_tokens: Some(2),
            completion_tokens: Some(5),
            reasoning_tokens: Some(1),
            total_tokens: Some(15),
            model: Some("gpt-test".to_string()),
            workspace_status: None,
            created_at_ms: 1,
        }
    }

    #[test]
    fn small_completion_payload_is_unchanged() {
        let mut completion = completion("done".to_string(), Some("build it".to_string()));
        let original = serde_json::to_string(&completion).unwrap();

        compact_completion_payload(&mut completion).unwrap();

        assert_eq!(serde_json::to_string(&completion).unwrap(), original);
    }

    #[test]
    fn actual_json_payload_is_compacted_below_replay_ceiling() {
        // Control characters expand to six bytes each in JSON, exercising the
        // case that a character-count-only limit cannot safely bound.
        let mut completion = completion(
            "\u{0001}".repeat(MAX_COMPLETION_OUTPUT_CHARS),
            Some("\u{0002}".repeat(80_000)),
        );

        compact_completion_payload(&mut completion).unwrap();

        assert!(
            serde_json::to_vec(&completion).unwrap().len() <= MAX_DURABLE_COMPLETION_PAYLOAD_BYTES
        );
        assert!(!completion.final_output.is_empty());
        assert!(completion
            .prompt
            .as_deref()
            .is_some_and(|value| !value.is_empty()));
    }

    #[test]
    fn compaction_with_tiny_prompt_still_makes_progress_to_auxiliary_metadata() {
        let mut completion = completion(String::new(), Some("ab".to_string()));
        completion.error = Some("\u{0001}".repeat(200_000));

        compact_completion_payload(&mut completion).unwrap();

        assert!(
            serde_json::to_vec(&completion).unwrap().len() <= MAX_DURABLE_COMPLETION_PAYLOAD_BYTES
        );
    }

    #[test]
    fn canceled_terminal_message_keeps_usage_already_emitted_by_child() {
        let stdout = r#"{"type":"token_count","model":"gpt-5.4","usage":{"input_tokens":120,"cached_input_tokens":20,"output_tokens":30,"total_tokens":150}}"#;
        let (done, combined_output) = cli_done_message_from_output(
            "req-canceled".to_string(),
            false,
            Some("用户已停止 PC CLI 任务".to_string()),
            stdout,
            "canceled after output",
            Some("fallback-model".to_string()),
            None,
            None,
        );

        let AgentToServer::CliDone {
            prompt_tokens,
            cached_input_tokens,
            completion_tokens,
            total_tokens,
            model,
            ..
        } = done
        else {
            panic!("expected CliDone");
        };
        assert_eq!(prompt_tokens, Some(120));
        assert_eq!(cached_input_tokens, Some(20));
        assert_eq!(completion_tokens, Some(30));
        assert_eq!(total_tokens, Some(150));
        assert_eq!(model.as_deref(), Some("gpt-5.4"));
        assert!(combined_output.contains("canceled after output"));
    }
}
