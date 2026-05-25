use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{store::TaskSnapshot, types::WsMessage};

pub const PROJECT_WS_BACKLOG_LIMIT: usize = 512;

#[derive(Deserialize)]
pub struct ProjectChatRequest {
    pub op: Option<String>,
    pub trace_id: Option<String>,
    pub task_id: Option<String>,
    pub client_request_id: Option<String>,
    pub message: String,
    pub agent: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_title: Option<String>,
    pub attachments: Option<Vec<ProjectAttachmentRef>>,
}

#[derive(Deserialize)]
pub struct ProjectPrewarmRequest {
    pub trace_id: Option<String>,
    pub agent: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectAttachmentRef {
    pub attachment_id: Option<String>,
    pub kind: Option<String>,
    pub display_name: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub path: Option<String>,
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
}

pub fn enrich_project_ws_event(raw: String, task_id: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return raw;
    };
    let Some(obj) = value.as_object_mut() else {
        return raw;
    };
    obj.entry("task_id")
        .or_insert_with(|| serde_json::json!(task_id));
    let event_kind = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    obj.entry("event")
        .or_insert_with(|| serde_json::json!(event_kind));
    obj.insert(
        "emitted_at_ms".into(),
        serde_json::json!(current_wall_time_ms()),
    );
    serde_json::to_string(&value).unwrap_or(raw)
}

pub fn task_control_event(
    event: &str,
    task_id: Option<&str>,
    client_request_id: Option<&str>,
    conversation_id: Option<&str>,
    message: &str,
) -> String {
    serde_json::json!({
        "type": "task_event",
        "event": event,
        "task_id": task_id,
        "client_request_id": client_request_id,
        "conversation_id": conversation_id,
        "message": message,
    })
    .to_string()
}

pub fn server_message_details(value: &serde_json::Value, bytes: usize) -> serde_json::Value {
    let message = value
        .get("message")
        .and_then(|message| message.as_str())
        .unwrap_or_default();
    serde_json::json!({
        "type": value.get("type").and_then(|kind| kind.as_str()).unwrap_or("unknown"),
        "bytes": bytes,
        "message_chars": message.chars().count(),
        "message_preview": preview_text(message, 180),
        "has_apk_url": value
            .get("apk_url")
            .and_then(|url| url.as_str())
            .map(|url| !url.is_empty())
            .unwrap_or(false),
        "has_image_url": value
            .get("image_url")
            .and_then(|url| url.as_str())
            .map(|url| !url.is_empty())
            .unwrap_or(false),
    })
}

pub fn project_client_request_id(
    request: &ProjectChatRequest,
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    message: &str,
) -> String {
    request
        .client_request_id
        .as_deref()
        .or(request.trace_id.as_deref())
        .map(|value| safe_request_id_part(value, 80))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            stable_request_id(&format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                project_id,
                user_id,
                conversation_id,
                request.agent.as_deref().unwrap_or(""),
                message
            ))
        })
}

pub fn terminal_backlog_from_task(task: &TaskSnapshot, mut events: Vec<String>) -> Vec<String> {
    if events.is_empty() {
        return vec![terminal_event_from_task(task)];
    }

    if !events
        .iter()
        .any(|event| is_terminal_project_ws_message(event))
    {
        events.push(terminal_event_from_task(task));
        if events.len() > PROJECT_WS_BACKLOG_LIMIT {
            let overflow = events.len() - PROJECT_WS_BACKLOG_LIMIT;
            events.drain(0..overflow);
        }
    }

    events
}

pub fn is_terminal_task_status(status: &str) -> bool {
    matches!(status, "done" | "failed" | "error")
}

pub fn is_terminal_project_ws_message(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|message_type| message_type.as_str())
                .map(|message_type| message_type == "done" || message_type == "error")
        })
        .unwrap_or(false)
}

pub fn is_done_project_ws_message(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|message_type| message_type.as_str())
                .map(|message_type| message_type == "done")
        })
        .unwrap_or(false)
}

pub fn parse_project_message(raw: &str) -> ProjectChatRequest {
    serde_json::from_str::<ProjectChatRequest>(raw).unwrap_or_else(|_| ProjectChatRequest {
        op: None,
        trace_id: None,
        task_id: None,
        client_request_id: None,
        message: raw.to_string(),
        agent: None,
        conversation_id: None,
        conversation_title: None,
        attachments: None,
    })
}

fn terminal_event_from_task(task: &TaskSnapshot) -> String {
    if task.status == "done" {
        WsMessage::Done {
            message: "任务已完成，正在恢复之前保存的结果。".into(),
            apk_url: task.apk_url.clone(),
            image_url: None,
        }
        .to_json()
    } else {
        WsMessage::Error {
            message: task
                .error
                .clone()
                .unwrap_or_else(|| "任务已结束，但没有保存详细错误。".into()),
        }
        .to_json()
    }
}

fn stable_request_id(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("auto_{}", hex::encode(&digest[..12]))
}

fn safe_request_id_part(value: &str, max_len: usize) -> String {
    let mut safe = value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .take(max_len)
        .collect::<String>();
    if safe.is_empty() {
        safe = "request".into();
    }
    safe
}

fn preview_text(value: &str, max_chars: usize) -> String {
    let mut preview = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

fn current_wall_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_attachment_refs() {
        let request = parse_project_message(
            r#"{
                "op":"run",
                "task_id":"tsk_legacy",
                "trace_id":"ui_123",
                "client_request_id":"req_123",
                "message":"please inspect this file",
                "attachments":[
                    {
                        "kind":"image",
                        "attachment_id":"att_123",
                        "display_name":"screenshot.png",
                        "file_name":"screenshot.png",
                        "mime_type":"image/png",
                        "path":"D:/workspace/attachments/c1/screenshot.png",
                        "sha256":"abc123",
                        "size_bytes":128,
                        "image_width":640,
                        "image_height":480
                    }
                ]
            }"#,
        );

        let attachment = request
            .attachments
            .as_ref()
            .and_then(|items| items.first())
            .expect("attachment ref should be parsed");
        assert_eq!(request.op.as_deref(), Some("run"));
        assert_eq!(request.task_id.as_deref(), Some("tsk_legacy"));
        assert_eq!(request.trace_id.as_deref(), Some("ui_123"));
        assert_eq!(request.client_request_id.as_deref(), Some("req_123"));
        assert_eq!(request.message, "please inspect this file");
        assert_eq!(attachment.kind.as_deref(), Some("image"));
        assert_eq!(attachment.attachment_id.as_deref(), Some("att_123"));
        assert_eq!(attachment.display_name.as_deref(), Some("screenshot.png"));
        assert_eq!(
            attachment.path.as_deref(),
            Some("D:/workspace/attachments/c1/screenshot.png")
        );
        assert_eq!(attachment.sha256.as_deref(), Some("abc123"));
        assert_eq!(attachment.size_bytes, Some(128));
        assert_eq!(attachment.image_width, Some(640));
        assert_eq!(attachment.image_height, Some(480));
    }

    #[test]
    fn derives_stable_client_request_id_from_trace() {
        let request = parse_project_message(
            r#"{
                "trace_id":"ui_123_abc",
                "message":"build apk"
            }"#,
        );

        let id =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");

        assert_eq!(id, "ui_123_abc");
    }

    #[test]
    fn derives_fallback_client_request_id_when_trace_missing() {
        let request = parse_project_message(r#"{"message":"build apk"}"#);

        let first =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");
        let second =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");

        assert!(first.starts_with("auto_"));
        assert_eq!(first, second);
    }

    #[test]
    fn enriches_project_ws_event_with_task_id_and_event() {
        let raw = WsMessage::progress("running").to_json();
        let enriched = enrich_project_ws_event(raw, "tsk_123");
        let value: serde_json::Value =
            serde_json::from_str(&enriched).expect("enriched payload should be valid json");
        assert_eq!(value["task_id"], "tsk_123");
        assert_eq!(value["event"], "progress");
        assert!(value["emitted_at_ms"].as_u64().is_some());
    }

    #[test]
    fn terminal_backlog_appends_done_when_replay_window_lacks_terminal() {
        let task = TaskSnapshot {
            id: "tsk_1".into(),
            project_id: "project".into(),
            user_id: "user".into(),
            conversation_id: Some("conversation".into()),
            message: "build apk".into(),
            status: "done".into(),
            apk_url: Some("http://example.test/app.apk".into()),
            error: None,
        };
        let events = (0..PROJECT_WS_BACKLOG_LIMIT)
            .map(|step| WsMessage::progress(format!("step {step}")).to_json())
            .collect::<Vec<_>>();

        let backlog = terminal_backlog_from_task(&task, events);

        assert_eq!(backlog.len(), PROJECT_WS_BACKLOG_LIMIT);
        assert!(is_terminal_project_ws_message(backlog.last().unwrap()));
        assert!(!backlog.iter().any(|raw| raw.contains("step 0")));
    }

    #[test]
    fn terminal_backlog_keeps_existing_terminal_event() {
        let task = TaskSnapshot {
            id: "tsk_1".into(),
            project_id: "project".into(),
            user_id: "user".into(),
            conversation_id: Some("conversation".into()),
            message: "build apk".into(),
            status: "done".into(),
            apk_url: Some("http://example.test/app.apk".into()),
            error: None,
        };
        let done = WsMessage::Done {
            message: "finished".into(),
            apk_url: task.apk_url.clone(),
            image_url: None,
        }
        .to_json();

        let backlog = terminal_backlog_from_task(&task, vec![done.clone()]);

        assert_eq!(backlog, vec![done]);
    }
}
