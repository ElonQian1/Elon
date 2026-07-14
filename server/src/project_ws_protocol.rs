use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference, store::TaskSnapshot, types::WsMessage,
    ui_design_tasks::UiDesignTaskInput,
};

pub const PROJECT_WS_BACKLOG_LIMIT: usize = 512;

#[derive(Deserialize)]
pub struct ProjectChatRequest {
    pub op: Option<String>,
    pub trace_id: Option<String>,
    pub task_id: Option<String>,
    pub client_request_id: Option<String>,
    pub message: String,
    pub agent: Option<String>,
    #[serde(
        default,
        alias = "runtimeRoute",
        alias = "pcRuntimeRoute",
        alias = "pc_runtime_route",
        alias = "pcRoute",
        alias = "pc_route"
    )]
    pub runtime_route: Option<String>,
    pub execution_mode: Option<String>,
    pub plan_mode: Option<bool>,
    pub conversation_id: Option<String>,
    pub conversation_title: Option<String>,
    #[serde(
        default,
        alias = "localNodeId",
        alias = "currentNodeId",
        alias = "preferredNodeId",
        alias = "nodeId"
    )]
    pub local_node_id: Option<String>,
    #[serde(
        default,
        alias = "localWorkspacePath",
        alias = "currentWorkspacePath",
        alias = "preferredWorkspacePath",
        alias = "workspacePath"
    )]
    pub local_workspace_path: Option<String>,
    #[serde(default, alias = "projectIconDataUrl")]
    pub project_icon_data_url: Option<String>,
    pub attachments: Option<Vec<ProjectAttachmentRef>>,
    #[serde(default, alias = "uiDesignTask")]
    pub ui_design_task: Option<UiDesignTaskInput>,
    #[serde(default, alias = "directPcCli", alias = "pcDirectCli")]
    pub direct_pc_cli: Option<bool>,
    /// 方案8: 客户端声明的 WS 协议版本，旧客户端为 None（服务器按 v1 处理）
    pub protocol_version: Option<u32>,
    /// 仅闲聊：true 时强制走轻量 casual chat，绝不进入项目 Codex 开发工作流。
    /// 悬浮球语音 agent 借用 AI 对话能力时设为 true，避免误判为开发任务而启动 Codex 导致超时。
    pub chat_only: Option<bool>,
}

impl ProjectChatRequest {
    pub(crate) fn pc_runtime_route(&self) -> Result<Option<PcRuntimeRoutePreference>, String> {
        self.runtime_route
            .as_deref()
            .map(PcRuntimeRoutePreference::from_request)
            .transpose()
            .map(Option::flatten)
    }
}

#[derive(Deserialize)]
pub struct ProjectPrewarmRequest {
    pub trace_id: Option<String>,
    pub agent: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_title: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ProjectAttachmentAnnotation {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
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
    pub duration_seconds: Option<u32>,
    pub transcription: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<ProjectAttachmentAnnotation>,
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
        .or_else(|| value.get("text"))
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
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                project_id,
                user_id,
                conversation_id,
                request.agent.as_deref().unwrap_or(""),
                request.runtime_route.as_deref().unwrap_or(""),
                request.execution_mode.as_deref().unwrap_or(""),
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
        runtime_route: None,
        execution_mode: None,
        plan_mode: None,
        conversation_id: None,
        conversation_title: None,
        local_node_id: None,
        local_workspace_path: None,
        project_icon_data_url: None,
        attachments: None,
        ui_design_task: None,
        direct_pc_cli: None,
        protocol_version: None,
        chat_only: None,
    })
}

fn terminal_event_from_task(task: &TaskSnapshot) -> String {
    if task.status == "done" {
        WsMessage::Done {
            message: "任务已完成，正在恢复之前保存的结果。".into(),
            apk_url: task.apk_url.clone(),
            image_url: None,
            model_used: None,
            node_id: None,
        }
        .to_json()
    } else {
        WsMessage::error(
            task.error
                .clone()
                .unwrap_or_else(|| "任务已结束，但没有保存详细错误。".into()),
        )
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
#[path = "project_ws_protocol_tests.rs"]
mod tests;
