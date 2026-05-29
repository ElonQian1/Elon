//! 项目任务完成事件广播 — 让所有项目成员（非发起人）收到 done 通知。
//!
//! 架构与 `friend_events` 相同：静态 broadcast 频道，`global_ws` 订阅后
//! 按 `member_user_ids` 精准路由到各在线成员的 WebSocket 连接。
//!
//! 典型场景：A 在群项目中触发构建，B、C 同为成员且在线，
//! 任务完成后 B、C 的 `/ws/app` 连接会自动收到 `project_task_done` 消息。

use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;

use serde::Serialize;

use crate::types::AppState;

static PROJECT_TASK_DONE_TX: LazyLock<broadcast::Sender<ProjectTaskDoneEvent>> =
    LazyLock::new(|| {
        let (tx, _) = broadcast::channel(64);
        tx
    });

/// 项目任务完成推送事件。
///
/// `member_user_ids` 不序列化到 JSON — 仅用于 `global_ws` 路由过滤。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectTaskDoneEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    #[serde(rename = "projectId")]
    pub project_id: String,
    /// 触发本次任务的用户 ID（发起人已通过自己的 WS 连接收到结果，不重复推送）
    #[serde(rename = "triggeredByUserId")]
    pub triggered_by_user_id: String,
    /// done 消息文本（来自 Codex 最终回复）
    pub message: String,
    /// APK 下载链接（若本次任务产出了 APK）
    #[serde(rename = "apkUrl", skip_serializing_if = "Option::is_none")]
    pub apk_url: Option<String>,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// 不序列化 — 仅供 global_ws 路由判断目标用户
    #[serde(skip)]
    pub member_user_ids: Vec<String>,
}

impl ProjectTaskDoneEvent {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<ProjectTaskDoneEvent> {
    PROJECT_TASK_DONE_TX.subscribe()
}

/// 广播项目任务完成事件给除发起人以外的所有在线项目成员。
///
/// - 解析 `done_raw` JSON 提取 `message` 和 `apk_url`
/// - 查询项目成员列表（排除 `triggered_by_user_id`）
/// - 若无其他成员则直接返回，不广播
pub fn publish_task_done(
    state: &Arc<AppState>,
    project_id: &str,
    triggered_by_user_id: &str,
    conversation_id: &str,
    done_raw: &str,
) {
    let parsed = serde_json::from_str::<serde_json::Value>(done_raw).ok();
    let message = parsed
        .as_ref()
        .and_then(|v| v["message"].as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("项目任务已完成")
        .chars()
        .take(400)
        .collect::<String>();
    let apk_url = parsed
        .as_ref()
        .and_then(|v| v["apk_url"].as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let member_ids = state
        .store
        .list_project_members(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.user_id)
        .filter(|uid| uid != triggered_by_user_id)
        .collect::<Vec<_>>();

    if member_ids.is_empty() {
        return;
    }

    let event = ProjectTaskDoneEvent {
        event_type: "project_task_done",
        project_id: project_id.to_string(),
        triggered_by_user_id: triggered_by_user_id.to_string(),
        message,
        apk_url,
        conversation_id: conversation_id.to_string(),
        member_user_ids: member_ids,
    };
    let _ = PROJECT_TASK_DONE_TX.send(event);
}
