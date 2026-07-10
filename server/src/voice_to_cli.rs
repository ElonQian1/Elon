// server/src/voice_to_cli.rs
//! 方案 B：转写完成后把文本投递到项目聊天或一龙AI 私聊。
//!
//! 入参：user_id / voice_target / project_id / conversation_id / transcript / ai_reply_tx
//! 出参：DispatchOutcome（立即返回，AI 回复通过 ai_reply_tx 异步流回）

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    external_app_registry::external_group_by_group_id,
    project_auth::project_access,
    project_chat::run_project_agent_with_scheduler,
    project_execution_mode::ProjectExecutionMode,
    types::AppState,
    voice_protocol::{
        VOICE_TARGET_EXTERNAL_GROUP, VOICE_TARGET_SOCIAL_AI_DIRECT, VOICE_TARGET_TRANSCRIBE_ONLY,
    },
};

pub struct DispatchTarget {
    pub user_id: String,
    pub voice_target: Option<String>,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub group_id: Option<String>,
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
    let message = transcript.trim();
    if message.is_empty() {
        return Ok(DispatchOutcome {
            ok: false,
            message: "转写文本为空，已忽略".into(),
        });
    }

    match target.voice_target.as_deref() {
        Some(VOICE_TARGET_TRANSCRIBE_ONLY) => {
            return Ok(DispatchOutcome {
                ok: true,
                message: format!("转写完成，未自动发送（{} 字）", message.chars().count()),
            });
        }
        Some(VOICE_TARGET_EXTERNAL_GROUP) => {
            return dispatch_to_external_group(state, target, message).await;
        }
        Some(VOICE_TARGET_SOCIAL_AI_DIRECT) => {
            return dispatch_to_direct_social_ai(state, target, message, ai_reply_tx).await;
        }
        _ => {}
    }

    dispatch_to_project_chat(state, target, message, ai_reply_tx).await
}

async fn dispatch_to_project_chat(
    state: &Arc<AppState>,
    target: &DispatchTarget,
    message: &str,
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

    let message = message.to_string();
    let char_count = message.chars().count();
    let download_base = format!("{}/api/projects/{}/download", state.public_url, project.id);

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
        None, // project_icon_data_url
        None, // agent_name
        None, // attachments
        ProjectExecutionMode::Execute,
        None,  // pc_runtime_route
        false, // direct_pc_cli
        None,  // project_preflight_note
        None,  // trace_id
        ai_reply_tx,
    )
    .await;

    Ok(DispatchOutcome {
        ok: true,
        message: format!("语音指令已投递（{char_count} 字）"),
    })
}

async fn dispatch_to_external_group(
    state: &Arc<AppState>,
    target: &DispatchTarget,
    transcript: &str,
) -> Result<DispatchOutcome> {
    let group_id = target
        .group_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("voice dispatch: external_group 缺少 group_id"))?;
    let Some((_app, _group)) = external_group_by_group_id(group_id) else {
        anyhow::bail!("voice dispatch: {group_id} 不是已注册外部应用群聊");
    };

    let message =
        state
            .store
            .send_friend_group_message(&target.user_id, group_id, transcript, None)?;
    if let Ok(recipient_user_ids) = state
        .store
        .friend_group_member_ids(&target.user_id, group_id)
    {
        crate::friend_events::publish_group_message(&message, recipient_user_ids);
    }
    crate::social_ai::spawn_group_reply(
        state.clone(),
        target.user_id.clone(),
        group_id.to_string(),
        message.content.clone(),
    );

    Ok(DispatchOutcome {
        ok: true,
        message: format!("语音消息已发送到群聊（{} 字）", transcript.chars().count()),
    })
}

async fn dispatch_to_direct_social_ai(
    state: &Arc<AppState>,
    target: &DispatchTarget,
    message: &str,
    ai_reply_tx: UnboundedSender<String>,
) -> Result<DispatchOutcome> {
    tracing::info!(
        target: "voice",
        user_id = %target.user_id,
        chars = message.chars().count(),
        "voice_to_cli: 投递转写文本到一龙AI私聊"
    );

    crate::social_ai::spawn_direct_friend_voice_reply(
        state.clone(),
        target.user_id.clone(),
        message,
        ai_reply_tx,
    )?;
    Ok(DispatchOutcome {
        ok: true,
        message: "一龙AI 正在回复".into(),
    })
}
