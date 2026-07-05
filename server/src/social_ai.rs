//! 好友/群聊里的 `@EL` 文本助手。
//!
//! 这里只做普通文本问答：不接工具、不修改代码、不触发构建。

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::{
    friend_events, intent_router,
    store::{SocialAiHistoryMessage, SocialAiPendingMention, SOCIAL_AI_USER_ID},
    types::{AgentConfig, AppState, WsMessage},
};

static SOCIAL_AI_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
const DIRECT_SOCIAL_AI_SCENE: &str = "一龙AI私聊";

pub(crate) const DEVELOPMENT_REDIRECT_REPLY: &str =
    "这个需求已经涉及项目开发，我在好友/群聊里不能直接写代码、改项目或打包。请到「项目」页面新建项目，或进入已有项目后在项目聊天里发起开发任务；在那里我可以按完整开发流程帮你实现。";

pub(crate) fn contains_el_mention(content: &str) -> bool {
    content
        .replace('＠', "@")
        .to_ascii_lowercase()
        .contains("@el")
}

pub(crate) fn spawn_friend_reply(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
    trigger_content: String,
) {
    if !contains_el_mention(&trigger_content) {
        return;
    }
    spawn_friend_reply_if_needed(state, user_id, friend_id);
}

pub(crate) fn spawn_friend_reply_if_needed(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
) {
    let pending = match state
        .store
        .latest_unanswered_friend_social_ai_mention(&user_id, &friend_id)
    {
        Ok(pending) => pending,
        Err(error) => {
            warn!("friend @EL pending lookup failed: {}", error);
            return;
        }
    };
    let Some(pending) = pending else {
        return;
    };
    spawn_friend_pending_reply(state, user_id, friend_id, pending);
}

pub(crate) fn spawn_direct_friend_reply(
    state: Arc<AppState>,
    user_id: String,
    trigger_message_id: String,
) {
    let key = format!("direct:{user_id}:{trigger_message_id}");
    if !mark_in_flight(&key) {
        return;
    }
    info!(
        "direct social AI reply queued: user_id={} trigger_message_id={}",
        user_id, trigger_message_id
    );
    tokio::spawn(async move {
        let result = reply_to_direct_friend(state, user_id).await;
        clear_in_flight(&key);
        if let Err(error) = result {
            warn!("direct social AI reply failed: {}", error);
        } else {
            info!(
                "direct social AI reply stored: trigger_message_id={}",
                trigger_message_id
            );
        }
    });
}

pub(crate) fn spawn_direct_friend_voice_reply(
    state: Arc<AppState>,
    user_id: String,
    transcript: &str,
    ai_reply_tx: UnboundedSender<String>,
) -> Result<()> {
    let content = transcript.trim();
    if content.is_empty() {
        let _ = ai_reply_tx.send(WsMessage::error("转写文本为空，已忽略").to_json());
        return Ok(());
    }
    let message = state
        .store
        .send_friend_message(&user_id, SOCIAL_AI_USER_ID, content, None)?;
    let trigger_message_id = message.id.clone();
    friend_events::publish_friend_message(&message);
    info!(
        "direct social AI voice reply queued: user_id={} trigger_message_id={}",
        user_id, trigger_message_id
    );
    tokio::spawn(async move {
        let result = reply_to_direct_friend(state, user_id).await;
        match result {
            Ok(reply) => {
                let _ = ai_reply_tx.send(
                    WsMessage::Done {
                        message: reply,
                        apk_url: None,
                        image_url: None,
                        model_used: None,
                        node_id: None,
                    }
                    .to_json(),
                );
                info!(
                    "direct social AI voice reply stored: trigger_message_id={}",
                    trigger_message_id
                );
            }
            Err(error) => {
                warn!("direct social AI voice reply failed: {}", error);
                let _ = ai_reply_tx
                    .send(WsMessage::error(format!("一龙AI 语音回复失败：{}", error)).to_json());
            }
        }
    });
    Ok(())
}

fn spawn_friend_pending_reply(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
    pending: SocialAiPendingMention,
) {
    if !contains_el_mention(&pending.trigger_content) {
        return;
    }
    let key = format!(
        "friend:{user_id}:{friend_id}:{}",
        pending.trigger_message_id
    );
    if !mark_in_flight(&key) {
        return;
    }
    let trigger_message_id = pending.trigger_message_id;
    info!(
        "friend @EL reply queued: user_id={} friend_id={} trigger_message_id={}",
        user_id, friend_id, trigger_message_id
    );
    tokio::spawn(async move {
        let result = reply_to_friend(state, user_id, friend_id).await;
        clear_in_flight(&key);
        if let Err(error) = result {
            warn!("friend @EL reply failed: {}", error);
        } else {
            info!(
                "friend @EL reply stored: trigger_message_id={}",
                trigger_message_id
            );
        }
    });
}

pub(crate) fn spawn_group_reply(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    trigger_content: String,
) {
    if !contains_el_mention(&trigger_content) {
        return;
    }
    spawn_group_reply_if_needed(state, user_id, group_id);
}

pub(crate) fn spawn_group_reply_if_needed(state: Arc<AppState>, user_id: String, group_id: String) {
    let pending = match state
        .store
        .latest_unanswered_group_social_ai_mention(&user_id, &group_id)
    {
        Ok(pending) => pending,
        Err(error) => {
            warn!("group @EL pending lookup failed: {}", error);
            return;
        }
    };
    let Some(pending) = pending else {
        return;
    };
    spawn_group_pending_reply(state, user_id, group_id, pending);
}

fn spawn_group_pending_reply(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    pending: SocialAiPendingMention,
) {
    if !contains_el_mention(&pending.trigger_content) {
        return;
    }
    let key = format!("group:{group_id}:{}", pending.trigger_message_id);
    if !mark_in_flight(&key) {
        return;
    }
    let trigger_message_id = pending.trigger_message_id;
    info!(
        "group @EL reply queued: user_id={} group_id={} trigger_message_id={}",
        user_id, group_id, trigger_message_id
    );
    tokio::spawn(async move {
        let result = reply_to_group(state, user_id, group_id).await;
        clear_in_flight(&key);
        if let Err(error) = result {
            warn!("group @EL reply failed: {}", error);
        } else {
            info!(
                "group @EL reply stored: trigger_message_id={}",
                trigger_message_id
            );
        }
    });
}

async fn reply_to_friend(state: Arc<AppState>, user_id: String, friend_id: String) -> Result<()> {
    let history = state
        .store
        .list_recent_friend_messages_for_social_ai(&user_id, &friend_id, 18)?;
    // 方案6: 统一使用 classify() 检测开发意图；方案4: 开发意图走桥接卡片
    if let Some(summary) = is_development_intent(&history) {
        let messages = state.store.insert_friend_social_ai_reply(
            &user_id,
            &friend_id,
            DEVELOPMENT_REDIRECT_REPLY,
        )?;
        for message in messages {
            friend_events::publish_friend_bridge_message(&message, &summary);
        }
        return Ok(());
    }
    let reply =
        social_ai_reply_or_fallback(&state, &user_id, "好友聊天", &history, None, None).await;
    let messages = state
        .store
        .insert_friend_social_ai_reply(&user_id, &friend_id, &reply)?;
    for message in messages {
        friend_events::publish_friend_message(&message);
    }
    Ok(())
}

async fn reply_to_direct_friend(state: Arc<AppState>, user_id: String) -> Result<String> {
    let history =
        state
            .store
            .list_recent_friend_messages_for_social_ai(&user_id, SOCIAL_AI_USER_ID, 18)?;
    if let Some(summary) = is_development_intent(&history) {
        let message = state
            .store
            .insert_direct_social_ai_reply(&user_id, DEVELOPMENT_REDIRECT_REPLY)?;
        friend_events::publish_friend_bridge_message(&message, &summary);
        return Ok(DEVELOPMENT_REDIRECT_REPLY.to_string());
    }
    let reply = social_ai_reply_or_fallback(
        &state,
        &user_id,
        DIRECT_SOCIAL_AI_SCENE,
        &history,
        None,
        None,
    )
    .await;
    let message = state
        .store
        .insert_direct_social_ai_reply(&user_id, &reply)?;
    friend_events::publish_friend_message(&message);
    Ok(reply)
}

async fn reply_to_group(state: Arc<AppState>, user_id: String, group_id: String) -> Result<()> {
    let recipient_user_ids = state.store.friend_group_member_ids(&user_id, &group_id)?;
    let history = state
        .store
        .list_recent_group_messages_for_social_ai(&user_id, &group_id, 50)?;
    let mut feedback_context = None;
    let mut feedback_tool_results = None;
    // 方案6: 统一使用 classify() 检测开发意图；方案4: 开发意图走桥接（群聊暂只发文字，无桥接卡片）
    let reply = if is_development_intent(&history).is_some() {
        DEVELOPMENT_REDIRECT_REPLY.to_string()
    } else {
        let topic_hint = latest_request_user_text(&history);
        let external_context = crate::external_app_context::group_context_for_chat(
            &state,
            &user_id,
            &group_id,
            topic_hint.as_deref(),
        )
        .await;
        let external_tool_results = if let Some(context) = external_context.as_ref() {
            crate::external_app_context_tool_runtime::group_tool_results_for_chat(
                &state,
                &user_id,
                &group_id,
                context,
                topic_hint.as_deref(),
            )
            .await
        } else {
            None
        };
        let reply = social_ai_reply_or_fallback(
            &state,
            &user_id,
            "群聊",
            &history,
            external_context.as_ref(),
            external_tool_results.as_ref(),
        )
        .await;
        feedback_context = external_context;
        feedback_tool_results = external_tool_results;
        reply
    };
    let message = state
        .store
        .insert_group_social_ai_reply(&group_id, &reply)?;
    crate::external_app_context_feedback::spawn_generated_answer_feedback(
        Arc::clone(&state),
        user_id,
        group_id.clone(),
        format!("social_group_message:{}", message.id),
        "group_mention",
        feedback_context,
        feedback_tool_results,
        reply.clone(),
        Vec::new(),
    );
    friend_events::publish_group_message(&message, recipient_user_ids);
    Ok(())
}

async fn social_ai_reply_or_fallback(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    match build_reply(
        state,
        user_id,
        scene,
        history,
        external_context,
        external_tool_results,
    )
    .await
    {
        Ok(reply) => reply,
        Err(error) => {
            warn!("{scene} AI 生成失败: {}", error);
            social_ai_fallback_message(
                scene,
                &error.to_string(),
                external_context,
                external_tool_results,
            )
        }
    }
}

fn social_ai_fallback_message(
    scene: &str,
    error: &str,
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    if is_billing_or_quota_error(error) {
        return error.to_string();
    }
    if is_fb2_external_context(external_context) {
        return fb2_social_ai_generation_fallback(external_context, external_tool_results);
    }
    if scene == DIRECT_SOCIAL_AI_SCENE {
        "一龙AI 暂时没能连上 AI。你可以稍后再发一次，或联系管理员检查 AI 代理配置。".into()
    } else {
        "EL 暂时没能连上 AI。你可以稍后再 @EL 一次，或联系管理员检查 AI 代理配置。".into()
    }
}

fn fb2_social_ai_generation_fallback(
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    let source_markers = fb2_fallback_source_markers(external_context);
    let source_line = if source_markers.is_empty() {
        "来源：fb2 Context Pack 已读取，但当前没有可写入回复的 source id。".to_string()
    } else {
        format!("来源：{}", source_markers.join("，"))
    };
    let reply = format!(
        "数据事实：主项目已读取 fb2 比赛、订单和群聊上下文，但当前 AI 模型服务暂时不可用，本次不能生成完整赛事分析。\n\
         群友观点：已读取群友观点或观点记忆；在模型恢复前，只能把它作为讨论线索，不采纳为比赛事实。\n\
         AI推断：请稍后再 @EL 重新分析；现在不对比赛结果、赔率方向或订单风险做确定判断。\n\
         风险边界：赛果不确定，不保证命中，不建议重注或梭哈。\n\
         {source_line}"
    );
    ensure_fb2_opinion_memory_source(&reply, external_context, external_tool_results)
}

fn fb2_fallback_source_markers(external_context: Option<&Value>) -> Vec<String> {
    let Some(context) = external_context else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if let Some(context_audit_id) = clean_json_string(context.get("context_audit_id")) {
        push_unique_source_marker(
            &mut out,
            &mut seen,
            format!("context_audit_id {context_audit_id}"),
        );
    }

    for preferred_kind in [
        "match",
        "odds",
        "user_order",
        "order",
        "ticket",
        "group_message",
        "opinion_memory",
        "platform_order_summary",
    ] {
        push_first_context_source_marker(context, preferred_kind, &mut out, &mut seen);
        if out.len() >= 4 {
            break;
        }
    }
    out
}

fn push_first_context_source_marker(
    context: &Value,
    preferred_kind: &str,
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(source) = context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|source| {
            source
                .get("kind")
                .and_then(Value::as_str)
                .map(|kind| kind.trim() == preferred_kind)
                .unwrap_or(false)
        })
    else {
        return;
    };
    if let Some(marker) = fallback_source_marker(source) {
        push_unique_source_marker(out, seen, marker);
    }
}

fn fallback_source_marker(source: &Value) -> Option<String> {
    let id = clean_json_string(source.get("id"))?;
    let kind = source
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("source");
    let label = match kind {
        "match" => "match_id",
        "odds" => "odds_source_id",
        "user_order" | "order" => "order_id",
        "ticket" => "ticket_id",
        "group_message" => "message_id",
        "opinion_memory" | "group_opinion_memory" => "opinion_memory_id",
        "platform_order_summary" => "platform_order_summary",
        "context_audit" => "context_audit_id",
        _ => "source_id",
    };
    Some(format!("{label} {id}"))
}

fn push_unique_source_marker(out: &mut Vec<String>, seen: &mut HashSet<String>, marker: String) {
    if out.len() >= 4 {
        return;
    }
    if seen.insert(marker.to_lowercase()) {
        out.push(marker);
    }
}

fn is_billing_or_quota_error(error: &str) -> bool {
    error.contains("余额不足")
        || error.contains("计费系统暂时不可用")
        || error.contains("token 用量已达上限")
        || error.contains("用户已被封禁")
}


pub(crate) mod reply_core;
pub(crate) use self::reply_core::*;
