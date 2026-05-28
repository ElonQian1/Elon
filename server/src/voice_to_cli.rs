//! 方案 B：转写完成后把文本投递回现有的项目聊天/CLI 链路。
//!
//! 为避免与 `project_chat` / `ai_cli` 紧耦合，本模块只做"协议层翻译"：
//! - 入参：user_id / project_id / conversation_id / transcript
//! - 出参：把文本转成现有 HTTP / 内部 API 期望的形式
//!
//! 当前实现保守：仅把转写文本写到 trace 日志并返回成功标记；
//! 真正接入 `project_chat::ws_user_project_handler` 的内部派发由后续 PR 完成。
//! 这样可以让管线先端到端跑通，再做"接入聊天"的语义改造。

use anyhow::Result;
use std::sync::Arc;

use crate::types::AppState;

pub struct DispatchTarget {
    pub user_id: String,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DispatchOutcome {
    pub ok: bool,
    pub message: String,
}

/// 把一段最终转写文本投递给"对应的 AI"。
///
/// MVP 策略：
/// - 当前只记录 trace + 通过 [`crate::speech_translate`] 已有的入口做后续派发
/// - 未来扩展：若 `project_id` 存在 → 走 project_chat；否则 → 走全局聊天 agent
pub async fn dispatch_transcript(
    _state: &Arc<AppState>,
    target: &DispatchTarget,
    transcript: &str,
) -> Result<DispatchOutcome> {
    tracing::info!(
        target: "voice",
        user_id = %target.user_id,
        project_id = ?target.project_id,
        conversation_id = ?target.conversation_id,
        text = %transcript,
        "voice_to_cli: 接收到最终转写文本（占位：暂未触发 CLI 任务）"
    );

    // TODO(后续 PR)：调用现有 project_chat 内部分发函数或 ai_cli::run_with_workspace
    // 这里保留为占位，让骨架编译通过、让两条管线先端到端跑通

    Ok(DispatchOutcome {
        ok: true,
        message: format!("已记录转写文本（{} 字符），后续将接入聊天链路", transcript.chars().count()),
    })
}
