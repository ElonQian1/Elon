use std::time::{Duration, Instant};

use crate::{
    project_space_ai_progress::{
        pc_cli_communication_recovering_progress, pc_cli_recovery_timeout_progress,
    },
    project_space_task_result::result_message,
    project_ws_protocol::enrich_project_ws_event,
    types::WsMessage,
};

use super::{insert_channel_ai_progress, insert_channel_ai_result, ChannelAiTask};

pub(super) enum ChannelAiRecoveryTick {
    Healthy,
    StartRecovery { timeout_secs: u64 },
    RecoveryTimeout { timeout_secs: u64, recovery_secs: u64 },
}

pub(super) struct ChannelAiRecoveryWatchdog {
    heartbeat_only_timeout: Duration,
    last_effective_progress_at: Instant,
    communication_recovery_started_at: Option<Instant>,
}

impl ChannelAiRecoveryWatchdog {
    pub(super) fn new(heartbeat_only_timeout: Duration) -> Self {
        Self {
            heartbeat_only_timeout,
            last_effective_progress_at: Instant::now(),
            communication_recovery_started_at: None,
        }
    }

    pub(super) fn note_cli_event(&mut self, is_heartbeat: bool) {
        if is_heartbeat {
            return;
        }
        self.last_effective_progress_at = Instant::now();
        self.communication_recovery_started_at = None;
    }

    pub(super) fn tick(&mut self) -> ChannelAiRecoveryTick {
        if self.last_effective_progress_at.elapsed() < self.heartbeat_only_timeout {
            return ChannelAiRecoveryTick::Healthy;
        }
        let timeout_secs = self.heartbeat_only_timeout.as_secs();
        match self.communication_recovery_started_at {
            None => {
                self.communication_recovery_started_at = Some(Instant::now());
                self.last_effective_progress_at = Instant::now();
                ChannelAiRecoveryTick::StartRecovery { timeout_secs }
            }
            Some(started_at) => ChannelAiRecoveryTick::RecoveryTimeout {
                timeout_secs,
                recovery_secs: started_at.elapsed().as_secs(),
            },
        }
    }
}

pub(super) fn record_recovery_started(task: &ChannelAiTask, timeout_secs: u64) {
    if let Some(raw_status) = pc_cli_communication_recovering_progress(timeout_secs) {
        let _ = task.state.store.record_task_event(
            &task.task_id,
            &enrich_project_ws_event(raw_status.clone(), &task.task_id),
        );
        insert_channel_ai_progress(task, &raw_status);
    }
}

pub(super) fn record_recovery_timeout(
    task: &ChannelAiTask,
    timeout_secs: u64,
    recovery_secs: u64,
) -> String {
    if let Some(raw_status) = pc_cli_recovery_timeout_progress(timeout_secs, recovery_secs) {
        let _ = task.state.store.record_task_event(
            &task.task_id,
            &enrich_project_ws_event(raw_status.clone(), &task.task_id),
        );
        insert_channel_ai_progress(task, &raw_status);
    }
    let msg = format!(
        "通信自动恢复超时：本机 AI 已被 PC 节点确认接收，但在 {} 秒无新的公开输出后，又等待 {} 秒仍未恢复。服务器正在更新升级、Win 端正在更新升级/重启或节点通信中断时会出现这种情况；本轮已停止等待，避免重复执行。请确认 Win 端在线后在会话中继续。",
        timeout_secs,
        recovery_secs
    );
    let raw_error = WsMessage::error(&msg).to_json();
    let _ = task
        .state
        .store
        .record_task_event(&task.task_id, &enrich_project_ws_event(raw_error, &task.task_id));
    insert_channel_ai_result(task, &result_message(&msg, None, Some("自动恢复超时")));
    msg
}

pub(super) fn pc_cli_communication_error_result(raw_reason: &str) -> (&'static str, String, &'static str) {
    if is_pc_cli_communication_interruption(raw_reason) {
        return (
            "interrupted",
            format!(
                "PC 节点通信临时中断：服务器正在更新升级、Win 端正在更新升级/重启或节点连接重建时，会短暂打断 Codex CLI 通信。本轮已停止等待，避免重复执行。请确认 Win 端在线后在会话中继续。技术原因：{}",
                raw_reason
            ),
            "通信中断",
        );
    }
    ("failed", raw_reason.to_string(), "失败")
}

fn is_pc_cli_communication_interruption(message: &str) -> bool {
    let compact = message.replace(' ', "");
    let lower = message.to_ascii_lowercase();
    compact.contains("PC节点已断线")
        || compact.contains("PC节点通信")
        || compact.contains("节点重新注册")
        || compact.contains("旧连接已关闭")
        || compact.contains("没有重新连接")
        || message.contains("连接中断")
        || message.contains("等待终态超时")
        || message.contains("未收到 CliDone")
        || lower.contains("agent not connected")
        || lower.contains("agent ws read timeout")
        || lower.contains("connection reset")
}
