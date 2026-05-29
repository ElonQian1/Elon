//! 好友/群聊里的 `@EL` 文本助手。
//!
//! 这里只做普通文本问答：不接工具、不修改代码、不触发构建。

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
    store::{SocialAiHistoryMessage, SocialAiPendingMention},
    types::{AgentConfig, AppState},
};

static SOCIAL_AI_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

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
        let messages = state
            .store
            .insert_friend_social_ai_reply(&user_id, &friend_id, DEVELOPMENT_REDIRECT_REPLY)?;
        for message in messages {
            friend_events::publish_friend_bridge_message(&message, &summary);
        }
        return Ok(());
    }
    let reply = social_ai_reply_or_fallback(&state, &user_id, "好友聊天", &history).await;
    let messages = state
        .store
        .insert_friend_social_ai_reply(&user_id, &friend_id, &reply)?;
    for message in messages {
        friend_events::publish_friend_message(&message);
    }
    Ok(())
}

async fn reply_to_group(state: Arc<AppState>, user_id: String, group_id: String) -> Result<()> {
    let recipient_user_ids = state.store.friend_group_member_ids(&user_id, &group_id)?;
    let history = state
        .store
        .list_recent_group_messages_for_social_ai(&user_id, &group_id, 18)?;
    // 方案6: 统一使用 classify() 检测开发意图；方案4: 开发意图走桥接（群聊暂只发文字，无桥接卡片）
    let reply = if is_development_intent(&history).is_some() {
        DEVELOPMENT_REDIRECT_REPLY.to_string()
    } else {
        social_ai_reply_or_fallback(&state, &user_id, "群聊", &history).await
    };
    let message = state
        .store
        .insert_group_social_ai_reply(&group_id, &reply)?;
    friend_events::publish_group_message(&message, recipient_user_ids);
    Ok(())
}

async fn social_ai_reply_or_fallback(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
) -> String {
    match build_reply(state, user_id, scene, history).await {
        Ok(reply) => reply,
        Err(error) => {
            warn!("{scene} @EL 生成失败: {}", error);
            "EL 暂时没能连上 AI。你可以稍后再 @EL 一次，或联系管理员检查 AI 代理配置。".into()
        }
    }
}

async fn build_reply(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
) -> Result<String> {
    if history.is_empty() {
        return Ok("我在。你可以把想问的问题发出来，再带上 @EL。".into());
    }
    // 注意：开发意图已在 reply_to_friend/group 层拦截；此处不需重复判断。

    let prompt_text = format!(
        "聊天场景：{scene}\n\n最近聊天（从旧到新）：\n{}\n\n请回答最后一次 @EL 触发的问题。",
        format_history(history)
    );

    match resolve_social_agent(state).await {
        Ok(agent) => {
            let response = call_chat_llm(
                state,
                &agent,
                &[
                    json!({ "role": "system", "content": social_ai_prompt() }),
                    json!({ "role": "user", "content": prompt_text }),
                ],
                user_id,
                "social_ai",
            )
            .await?;
            let reply = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("我在，但刚才没组织好回复。你可以换个说法再 @EL 一次。")
                .trim();
            Ok(if reply.is_empty() {
                "我在，但刚才没组织好回复。你可以换个说法再 @EL 一次。".into()
            } else {
                reply.chars().take(1400).collect()
            })
        }
        Err(api_err) if state.ai_cli.enabled => {
            info!("social AI 无 API 代理，回退到本地 CLI: {}", api_err);
            build_reply_with_cli(state, user_id, &prompt_text).await
        }
        Err(api_err) => Err(api_err),
    }
}

/// 使用本地 AI CLI（无工具、无项目工作区）生成社交聊天回复
async fn build_reply_with_cli(
    state: &Arc<AppState>,
    user_id: &str,
    prompt: &str,
) -> Result<String> {
    use crate::intent_router::CapabilityRoute;
    use tokio::sync::mpsc;

    let temp_dir = std::env::temp_dir().join(format!("elon_social_{}", user_id));
    std::fs::create_dir_all(&temp_dir)?;

    let full_prompt = format!("{}\n\n{}", social_ai_prompt(), prompt);
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let run_result = crate::ai_cli::run_with_workspace(
        user_id,
        &temp_dir,
        "",
        &full_prompt,
        None,
        None,
        CapabilityRoute::ChatAgent,
        false,
        None,
        None,
        state,
        &tx,
    )
    .await;

    drop(tx); // 关闭 sender，让 rx 可以正常耗尽

    let mut final_reply: Option<String> = None;
    while let Some(msg_json) = rx.recv().await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg_json) {
            if val["type"].as_str() == Some("done") {
                final_reply = val["message"].as_str().map(|s| s.to_string());
            }
        }
    }

    run_result?; // CLI 报错则传播
    final_reply
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("本地 AI 未返回完整回复"))
}

pub(crate) async fn resolve_social_agent(state: &Arc<AppState>) -> Result<AgentConfig> {
    state
        .agents_config
        .read()
        .await
        .get_agent(None)
        .cloned()
        .ok_or_else(|| anyhow!("未配置可用 AI 代理，请先在后台配置 API 代理"))
}

pub(crate) fn social_ai_prompt() -> &'static str {
    r#"你是「EL」，一龙好友聊天和群聊里的文本 AI 助手。

你只做普通文本解答：可以解释、总结、建议、安慰、帮忙梳理想法，但不能写代码、不能修改项目、不能运行命令、不能构建或发布。

如果用户的问题涉及开发工作（例如做 App、改代码、修 bug、打包、部署、发布、项目功能实现），不要给代码方案，也不要假装已经开始做；请明确提醒用户去「项目」页面新建项目，或进入已有项目后在项目聊天里发起开发任务。

根据最近聊天历史回答最后一次 @EL 触发的问题。如果最后一句只是"@EL"或召唤你，请结合它前面的最后一个真实问题来回答。回复中文，简洁自然，只输出要发到聊天框里的文本。

注意：用户的部分消息来自手机语音识别，可能含有同音字替换或音近字错误（例如"你好码"其实是"你好吗"）。请优先推断最合理的语义，忽略明显的识别错误，直接给出正确理解下的回复，无需向用户解释纠错过程。"#
}

pub(crate) fn format_history(history: &[SocialAiHistoryMessage]) -> String {
    history
        .iter()
        .filter_map(|message| {
            let content = message.content.trim();
            if content.is_empty() {
                None
            } else {
                Some(format!("{}：{content}", message.speaker))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 若检测到开发意图（confidence ≥ 70 且 needs_code_change），返回触发文本摘要；否则 None。
/// 使用 intent_router::classify 而非旧的独立关键词函数，确保两路分类逻辑一致。
fn is_development_intent(history: &[SocialAiHistoryMessage]) -> Option<String> {
    let target = latest_request_user_text(history)?;
    let decision = intent_router::classify(&target);
    if decision.needs_code_change && decision.confidence >= 70 {
        Some(target.chars().take(80).collect())
    } else {
        None
    }
}

fn latest_request_user_text(history: &[SocialAiHistoryMessage]) -> Option<String> {
    history
        .iter()
        .rev()
        .filter(|message| message.from_request_user)
        .find_map(|message| {
            let content = strip_el_mention(&message.content);
            if content.is_empty() {
                None
            } else {
                Some(content)
            }
        })
}

fn strip_el_mention(content: &str) -> String {
    content
        .replace('＠', "@")
        .replace("@EL", "")
        .replace("@El", "")
        .replace("@eL", "")
        .replace("@el", "")
        .trim()
        .to_string()
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
    let mutex = SOCIAL_AI_IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}

#[cfg(test)]
mod tests {
    use super::{contains_el_mention, latest_request_user_text};
    use crate::store::SocialAiHistoryMessage;

    #[test]
    fn detects_half_and_full_width_mentions() {
        assert!(contains_el_mention("@EL 帮我看看"));
        assert!(contains_el_mention("＠el 这是什么意思"));
        assert!(!contains_el_mention("普通聊天"));
    }

    #[test]
    fn mention_only_uses_previous_user_question() {
        let history = vec![
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "这句话是什么意思？".into(),
                from_request_user: true,
            },
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "@EL".into(),
                from_request_user: true,
            },
        ];
        assert_eq!(
            latest_request_user_text(&history).as_deref(),
            Some("这句话是什么意思？")
        );
    }
}
