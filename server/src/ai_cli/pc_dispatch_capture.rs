use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{truncate_chars, AiCliRequestMode, NativeSessionScope};
use crate::types::AppState;

#[derive(Debug, Clone)]
pub(crate) struct PcAgentWorkspaceCaptureRequest {
    pub agent_id: String,
    pub user_id: String,
    pub workspace_path: String,
    pub user_message: String,
    pub preflight_note: Option<String>,
    pub request_mode: AiCliRequestMode,
    pub native_session_scope: Option<NativeSessionScope>,
    pub cli_name: String,
    pub copilot_model: Option<String>,
    pub codex_reasoning_effort: Option<String>,
    pub model_label: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PcAgentWorkspaceCaptureResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub pc_req_id: Option<String>,
    pub compute_call_id: Option<String>,
    pub transcript: String,
    pub model_used: Option<String>,
    pub node_id: Option<String>,
    pub event_count: usize,
    pub progress_messages: Vec<String>,
}

pub(crate) async fn run_pc_agent_workspace_capture(
    request: PcAgentWorkspaceCaptureRequest,
    state: &Arc<AppState>,
) -> PcAgentWorkspaceCaptureResult {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let collector = tokio::spawn(async move {
        let mut capture = PcAgentWorkspaceCaptureResult::default();
        while let Some(event) = rx.recv().await {
            ingest_capture_event(&mut capture, &event);
        }
        capture
    });

    let run_result = super::run_with_pc_agent_workspace(
        &request.agent_id,
        &request.user_id,
        &request.workspace_path,
        &request.user_message,
        request.preflight_note.as_deref(),
        request.request_mode,
        request.native_session_scope.clone(),
        Some(&request.cli_name),
        request.copilot_model.as_deref(),
        request.codex_reasoning_effort.as_deref(),
        request.model_label.as_deref(),
        state,
        &tx,
    )
    .await;
    drop(tx);

    let mut capture = collector.await.unwrap_or_default();
    capture.success = run_result.is_ok();
    capture.error_message = run_result.err().map(|error| error.to_string());
    capture.transcript = truncate_chars(capture.transcript.trim(), 12_000);
    if capture.compute_call_id.is_none() {
        capture.compute_call_id = capture
            .pc_req_id
            .as_deref()
            .map(|pc_req_id| format!("pc_agent_cli:{pc_req_id}"));
    }
    capture
}

fn ingest_capture_event(capture: &mut PcAgentWorkspaceCaptureResult, event: &str) {
    capture.event_count += 1;
    let Ok(value) = serde_json::from_str::<Value>(event) else {
        return;
    };
    match value.get("type").and_then(Value::as_str) {
        Some("pc_dispatch_started") => {
            if let Some(pc_req_id) = string_field(&value, "pc_req_id") {
                capture.compute_call_id = Some(format!("pc_agent_cli:{pc_req_id}"));
                capture.pc_req_id = Some(pc_req_id);
            }
            set_if_empty(&mut capture.node_id, string_field(&value, "agent_id"));
        }
        Some("assistant_message") => {
            append_text(capture, string_field(&value, "text"));
            set_if_empty(&mut capture.model_used, string_field(&value, "model_used"));
            set_if_empty(&mut capture.node_id, string_field(&value, "node_id"));
        }
        Some("assistant_chunk") => append_text(capture, string_field(&value, "text")),
        Some("done") => {
            append_text(capture, string_field(&value, "message"));
            set_if_empty(&mut capture.model_used, string_field(&value, "model_used"));
            set_if_empty(&mut capture.node_id, string_field(&value, "node_id"));
        }
        Some("progress") => {
            if capture.progress_messages.len() < 8 {
                if let Some(message) = string_field(&value, "message") {
                    capture.progress_messages.push(message);
                }
            }
        }
        _ => {}
    }
}

fn append_text(capture: &mut PcAgentWorkspaceCaptureResult, text: Option<String>) {
    let Some(text) = text else {
        return;
    };
    if text.trim().is_empty() {
        return;
    }
    if !capture.transcript.is_empty() && !capture.transcript.ends_with('\n') {
        capture.transcript.push('\n');
    }
    capture.transcript.push_str(&text);
}

fn set_if_empty(target: &mut Option<String>, value: Option<String>) {
    if target.is_none() {
        *target = value;
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_extracts_pc_dispatch_and_stream_text() {
        let mut capture = PcAgentWorkspaceCaptureResult::default();
        ingest_capture_event(
            &mut capture,
            r#"{"type":"pc_dispatch_started","pc_req_id":"req-1","agent_id":"node-a"}"#,
        );
        ingest_capture_event(
            &mut capture,
            r#"{"type":"assistant_message","text":"完成 A","model_used":"codex","node_id":"node-a"}"#,
        );
        ingest_capture_event(
            &mut capture,
            r#"{"type":"assistant_chunk","text":"完成 B"}"#,
        );

        assert_eq!(capture.pc_req_id.as_deref(), Some("req-1"));
        assert_eq!(
            capture.compute_call_id.as_deref(),
            Some("pc_agent_cli:req-1")
        );
        assert_eq!(capture.node_id.as_deref(), Some("node-a"));
        assert_eq!(capture.model_used.as_deref(), Some("codex"));
        assert!(capture.transcript.contains("完成 A"));
        assert!(capture.transcript.contains("完成 B"));
    }
}
