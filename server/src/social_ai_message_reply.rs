//! Long-press selected-message AI replies for friend chats and groups.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, OnceLock},
};
use tracing::{info, warn};

use crate::{
    friend_events, intent_router,
    social_ai::{format_history, social_ai_prompt, DEVELOPMENT_REDIRECT_REPLY},
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
        let result = reply_to_selected_friend_message(
            state,
            user_id,
            friend_id,
            message_id.clone(),
            selected,
        )
        .await;
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
        let result =
            reply_to_selected_group_message(state, user_id, group_id, message_id.clone(), selected)
                .await;
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


#[path = "social_ai_message_reply_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
