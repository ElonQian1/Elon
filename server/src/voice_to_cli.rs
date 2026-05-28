//! 方案 B：转写完成后把文本投递到现有项目聊天 AI 链路。
//!
//! 入参：user_id / project_id / conversation_id / transcript / ai_reply_tx
//! 出参：DispatchOutcome（立即返回，AI 回复通过 ai_reply_tx 异步流回）

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    project_auth::project_access,
    project_chat::run_project_agent_with_scheduler,
    types::AppState,
};

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

/// 把最终转写文本投递给项目聊天的 AI 链路。
///
/// - 验证 user_id + project_id 有权限
/// - 确保 conversation 存在（用传入的 conversation_id，或默认会话）
/// - 调用 `run_project_agent_with_scheduler` 异步启动 AI 任务
/// - AI 产生的 WsMessage JSON 通过 `ai_reply_tx` 流回，调用方负责消费
pub async fn dispatch_transcript(
    state: &Arc<AppState>,
    target: &DispatchTarget,
    transcript: &str,
    ai_reply_tx: UnboundedSender<String>,
) -> Result<DispatchOutcome> {
    let project_id = target
        .project_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("voice dispatch: 缺少 project_id"))?;

    let project = project_access(state.as_ref(), &target.user_id, project_id)
        .map_err(|e| anyhow::anyhow!("无权访问项目 {project_id}: {e}"))?;

    let conversation_id = state.store.ensure_conversation(
        &project.id,
        &target.user_id,
        target.conversation_id.as_deref(),
        None,
    )?;

    let message = transcript.trim().to_string();
    if message.is_empty() {
        return Ok(DispatchOutcome {
            ok: false,
            message: "转写文本为空，已忽略".into(),
        });
    }

    let char_count = message.chars().count();
    let download_base = format!(
        "{}/api/projects/{}/download",
        state.public_url, project.id
    );

    tracing::info!(
        target: "voice",
        user_id = %target.user_id,
        project_id,
        conversation_id,
        chars = char_count,
        "voice_to_cli: 投递转写文本到项目聊天 AI"
    );

    run_project_agent_with_scheduler(
        state.clone(),
        target.user_id.clone(),
        project,
        download_base,
        conversation_id,
        message,
        None, // agent_name
        None, // attachments
        None, // trace_id
        ai_reply_tx,
    )
    .await;

    Ok(DispatchOutcome {
        ok: true,
        message: format!("语音指令已投递（{char_count} 字）"),
    })
}
