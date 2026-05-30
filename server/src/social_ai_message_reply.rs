//! Long-press selected-message AI replies for friend chats and groups.

use anyhow::{anyhow, Result};
use serde_json::json;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};
use tracing::{info, warn};

use crate::{
    agent_llm_call::call_chat_llm,
    friend_events, intent_router,
    social_ai::{
        format_history, resolve_social_agent, social_ai_prompt, DEVELOPMENT_REDIRECT_REPLY,
    },
    store::SocialAiHistoryMessage,
    types::AppState,
};

static SELECTED_REPLY_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

pub(crate) fn spawn_friend_reply_for_message(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
    message_id: String,
) -> Result<()> {
    let selected = state
        .store
        .friend_message_for_social_ai_selection(&user_id, &friend_id, &message_id)?
        .ok_or_else(|| anyhow!("消息不存在或不属于当前好友聊天"))?;
    if selected.content.trim().is_empty() {
        return Err(anyhow!("这条消息没有可供 AI 回复的文本内容"));
    }
    let key = format!("friend-selected:{user_id}:{friend_id}:{message_id}");
    if !mark_in_flight(&key) {
        return Ok(());
    }
    info!(
        "friend selected-message AI reply queued: user_id={} friend_id={} message_id={}",
        user_id, friend_id, message_id
    );
    tokio::spawn(async move {
        let result = reply_to_selected_friend_message(state, user_id, friend_id, selected).await;
        clear_in_flight(&key);
        if let Err(error) = result {
            warn!("friend selected-message AI reply failed: {}", error);
        } else {
            info!(
                "friend selected-message AI reply stored: message_id={}",
                message_id
            );
        }
    });
    Ok(())
}

pub(crate) fn spawn_group_reply_for_message(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    message_id: String,
) -> Result<()> {
    let selected = state
        .store
        .group_message_for_social_ai_selection(&user_id, &group_id, &message_id)?
        .ok_or_else(|| anyhow!("消息不存在或不属于当前群聊"))?;
    if selected.content.trim().is_empty() {
        return Err(anyhow!("这条消息没有可供 AI 回复的文本内容"));
    }
    let key = format!("group-selected:{group_id}:{message_id}");
    if !mark_in_flight(&key) {
        return Ok(());
    }
    info!(
        "group selected-message AI reply queued: user_id={} group_id={} message_id={}",
        user_id, group_id, message_id
    );
    tokio::spawn(async move {
        let result = reply_to_selected_group_message(state, user_id, group_id, selected).await;
        clear_in_flight(&key);
        if let Err(error) = result {
            warn!("group selected-message AI reply failed: {}", error);
        } else {
            info!(
                "group selected-message AI reply stored: message_id={}",
                message_id
            );
        }
    });
    Ok(())
}

async fn reply_to_selected_friend_message(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
    selected: SocialAiHistoryMessage,
) -> Result<()> {
    let history = state
        .store
        .list_recent_friend_messages_for_social_ai(&user_id, &friend_id, 18)?;
    let reply = selected_reply_or_fallback(&state, &user_id, "好友聊天", &history, &selected).await;
    let messages = state
        .store
        .insert_friend_social_ai_reply(&user_id, &friend_id, &reply)?;
    for message in messages {
        friend_events::publish_friend_message(&message);
    }
    Ok(())
}

async fn reply_to_selected_group_message(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    selected: SocialAiHistoryMessage,
) -> Result<()> {
    let recipient_user_ids = state.store.friend_group_member_ids(&user_id, &group_id)?;
    let history = state
        .store
        .list_recent_group_messages_for_social_ai(&user_id, &group_id, 50)?;
    let reply = selected_reply_or_fallback(&state, &user_id, "群聊", &history, &selected).await;
    let message = state
        .store
        .insert_group_social_ai_reply(&group_id, &reply)?;
    friend_events::publish_group_message(&message, recipient_user_ids);
    Ok(())
}

async fn selected_reply_or_fallback(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    selected: &SocialAiHistoryMessage,
) -> String {
    match build_selected_reply(state, user_id, scene, history, selected).await {
        Ok(reply) => reply,
        Err(error) => {
            warn!("{scene} selected-message AI generation failed: {}", error);
            "EL 暂时没能连上 AI。你可以稍后再点一次「AI回复」，或联系管理员检查 AI 代理配置。"
                .into()
        }
    }
}

async fn build_selected_reply(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    selected: &SocialAiHistoryMessage,
) -> Result<String> {
    let selected_content = selected.content.trim();
    if selected_content.is_empty() {
        return Err(anyhow!("selected message is empty"));
    }
    if intent_router::looks_like_development_request(selected_content) {
        return Ok(DEVELOPMENT_REDIRECT_REPLY.into());
    }

    let agent = resolve_social_agent(state).await?;
    let response = call_chat_llm(
        state,
        &agent,
        &[
            json!({
                "role": "system",
                "content": social_ai_prompt()
            }),
            json!({
                "role": "system",
                "content": "本次触发来自用户长按历史消息后点击「AI回复」，不是 @EL 文本触发。请优先根据被选择的消息作答，必要时只把最近聊天作为上下文参考。"
            }),
            json!({
                "role": "user",
                "content": format!(
                    "聊天场景：{scene}\n\n最近聊天（从旧到新）：\n{}\n\n用户长按选择的消息：\n{}：{}\n\n请直接回复这条被选择的消息，输出要发到聊天框里的中文文本。",
                    format_history(history),
                    selected.speaker,
                    selected_content
                )
            }),
        ],
        user_id,
        "social_ai_selected",
    )
    .await?;
    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("我在，但刚才没组织好回复。你可以换条消息再点一次「AI回复」。")
        .trim();
    Ok(if reply.is_empty() {
        "我在，但刚才没组织好回复。你可以换条消息再点一次「AI回复」。".into()
    } else {
        reply.chars().take(1400).collect()
    })
}

fn mark_in_flight(key: &str) -> bool {
    with_in_flight(|items| items.insert(key.to_string()))
}

fn clear_in_flight(key: &str) {
    with_in_flight(|items| {
        items.remove(key);
    });
}

fn with_in_flight<T>(operation: impl FnOnce(&mut HashSet<String>) -> T) -> T {
    let mutex = SELECTED_REPLY_IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}
