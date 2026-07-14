use super::*;

pub(super) async fn reply_to_selected_friend_message(
    state: Arc<AppState>,
    user_id: String,
    friend_id: String,
    selected_message_id: String,
    selected: SocialAiHistoryMessage,
) -> Result<()> {
    let history = state
        .store
        .list_recent_friend_messages_for_social_ai(&user_id, &friend_id, 18)?;
    let reply = selected_reply_or_fallback(
        &state,
        &user_id,
        "好友聊天",
        &history,
        &selected,
        &selected_message_id,
        None,
        None,
    )
    .await;
    let messages = state
        .store
        .insert_friend_social_ai_reply(&user_id, &friend_id, &reply)?;
    for message in messages {
        friend_events::publish_friend_message(&message);
    }
    Ok(())
}

pub(super) async fn reply_to_selected_group_message(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    selected_message_id: String,
    selected: SocialAiHistoryMessage,
) -> Result<()> {
    let recipient_user_ids = state.store.friend_group_member_ids(&user_id, &group_id)?;
    let history = state
        .store
        .list_recent_group_messages_for_social_ai(&user_id, &group_id, 50)?;
    let topic_hint = selected_message_topic_hint(&selected);
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
    let reply = selected_reply_or_fallback(
        &state,
        &user_id,
        "群聊",
        &history,
        &selected,
        &selected_message_id,
        external_context.as_ref(),
        external_tool_results.as_ref(),
    )
    .await;
    let message = state
        .store
        .insert_group_social_ai_reply(&group_id, &reply)?;
    crate::external_app_context_feedback::spawn_generated_answer_feedback(
        Arc::clone(&state),
        user_id,
        group_id.clone(),
        format!("social_group_selected_message:{}", message.id),
        "selected_message_ai_reply",
        external_context,
        external_tool_results,
        reply.clone(),
        vec![selected_message_citation_source(&selected_message_id)],
    );
    friend_events::publish_group_message(&message, recipient_user_ids);
    Ok(())
}

pub(super) async fn selected_reply_or_fallback(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    selected: &SocialAiHistoryMessage,
    selected_message_id: &str,
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> String {
    match build_selected_reply(
        state,
        user_id,
        scene,
        history,
        selected,
        selected_message_id,
        external_context,
        external_tool_results,
    )
    .await
    {
        Ok(reply) => reply,
        Err(error) => {
            warn!("{scene} selected-message AI generation failed: {}", error);
            selected_message_generation_fallback(selected, selected_message_id, external_context)
        }
    }
}

pub(super) async fn build_selected_reply(
    state: &Arc<AppState>,
    user_id: &str,
    scene: &str,
    history: &[SocialAiHistoryMessage],
    selected: &SocialAiHistoryMessage,
    selected_message_id: &str,
    external_context: Option<&Value>,
    external_tool_results: Option<&Value>,
) -> Result<String> {
    let selected_content = selected.content.trim();
    if selected_content.is_empty() {
        return Err(anyhow!("selected message is empty"));
    }
    if intent_router::looks_like_development_request(selected_content) {
        return Ok(DEVELOPMENT_REDIRECT_REPLY.into());
    }

    let external_context_block =
        crate::social_ai::format_external_context(external_context, external_tool_results);
    let external_context_section = if external_context_block.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n{}", external_context_block)
    };
    let response = crate::social_ai_agents::call_social_chat_llm_with_fallback(
        state,
        &[
            json!({
                "role": "system",
                "content": format!(
                    "{}\n\n本次触发来自用户长按历史消息后点击「AI回复」，不是 @EL 文本触发。请优先根据被选择的消息作答，必要时只把最近聊天作为上下文参考。回答末尾必须用一行短句标注来源，至少包含 selected_message_id={selected_message_id}；如果使用了 fb2 外部数据，也要同时列出对应 match_id/order_id/context_audit_id 等来源。若被选择消息包含“肯定赢盘、稳赢、稳赚、包赢、重注、梭哈”等表述，必须明确指出这是过度确定或诱导投注，只能在「数据事实」「AI推断」「风险边界」标签下说明。",
                    social_ai_prompt()
                )
            }),
            json!({
                "role": "user",
                "content": format!(
                    "聊天场景：{scene}\nselected_message_id：{selected_message_id}\n\n最近聊天（从旧到新）：\n{}{}\n\n用户长按选择的消息：\n{}：{}\n\n请直接回复这条被选择的消息，输出要发到聊天框里的中文文本。",
                    format_history(history),
                    external_context_section,
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
    let reply: String = if reply.is_empty() {
        "我在，但刚才没组织好回复。你可以换条消息再点一次「AI回复」。".to_string()
    } else {
        reply.chars().take(1400).collect()
    };
    let reply = ensure_selected_message_reply_shape(
        &reply,
        selected_message_id,
        selected_content,
        external_context,
    );
    Ok(reply)
}

pub(super) fn selected_message_topic_hint(selected: &SocialAiHistoryMessage) -> Option<String> {
    let mut text = selected.content.replace('＠', "@");
    for mention in ["@EL", "@El", "@eL", "@el"] {
        text = text.replace(mention, "");
    }
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.chars().take(500).collect())
    }
}

pub(super) fn selected_message_citation_source(message_id: &str) -> Value {
    json!({
        "kind": "selected_message",
        "id": message_id,
        "label": "被长按的群聊消息"
    })
}

pub(super) fn ensure_selected_message_source(reply: &str, selected_message_id: &str) -> String {
    let reply = reply.trim();
    let selected_message_id = selected_message_id.trim();
    if reply.is_empty() || selected_message_id.is_empty() {
        return reply.to_string();
    }
    if reply
        .to_lowercase()
        .contains(&selected_message_id.to_lowercase())
    {
        return reply.to_string();
    }
    format!("{reply}\n来源补充：selected_message_id {selected_message_id}")
}

pub(super) fn ensure_current_context_audit_source(
    reply: &str,
    external_context: Option<&Value>,
) -> String {
    let Some(context_audit_id) = external_context
        .and_then(|context| context.get("context_audit_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return reply.to_string();
    };

    let reply = replace_context_audit_id_values(reply, context_audit_id);
    if reply
        .to_lowercase()
        .contains(&context_audit_id.to_lowercase())
    {
        return reply;
    }
    format!("{reply}\n来源补充：context_audit_id {context_audit_id}")
}

pub(super) fn replace_context_audit_id_values(
    reply: &str,
    current_context_audit_id: &str,
) -> String {
    let marker = "context_audit_id";
    let lower = reply.to_lowercase();
    let mut out = String::with_capacity(reply.len() + current_context_audit_id.len());
    let mut cursor = 0;

    while let Some(relative_start) = lower[cursor..].find(marker) {
        let marker_start = cursor + relative_start;
        let marker_end = marker_start + marker.len();
        out.push_str(&reply[cursor..marker_end]);

        let mut separator_end = marker_end;
        for (offset, ch) in reply[marker_end..].char_indices() {
            if ch.is_whitespace() || matches!(ch, ':' | '：' | '=') {
                separator_end = marker_end + offset + ch.len_utf8();
            } else {
                break;
            }
        }
        out.push_str(&reply[marker_end..separator_end]);

        let mut token_end = separator_end;
        for (offset, ch) in reply[separator_end..].char_indices() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                token_end = separator_end + offset + ch.len_utf8();
            } else {
                break;
            }
        }

        if token_end > separator_end {
            out.push_str(current_context_audit_id);
            cursor = token_end;
        } else {
            cursor = separator_end;
        }
    }

    out.push_str(&reply[cursor..]);
    out
}

pub(super) fn selected_message_generation_fallback(
    selected: &SocialAiHistoryMessage,
    selected_message_id: &str,
    external_context: Option<&Value>,
) -> String {
    let selected_excerpt = truncate_chars(selected.content.trim(), 120);
    let review_terms = selected_message_review_terms(&selected.content);
    let reply = if review_terms.is_empty() {
        format!(
            "数据事实：AI 模型暂时不可用，本次只能复核被选择消息文本：{selected_excerpt}\n\
             AI推断：这条消息需要谨慎看待，等待模型恢复后可再次点击「AI回复」获取更完整分析。"
        )
    } else {
        format!(
            "数据事实：AI 模型暂时不可用，本次只能复核被选择消息文本：{selected_excerpt}\n\
             AI推断：被选择消息里的“{}”表述过于绝对，不合理，不应作为投注结论。",
            review_terms.join("、")
        )
    };
    ensure_selected_message_reply_shape(
        &reply,
        selected_message_id,
        &selected.content,
        external_context,
    )
}

pub(super) fn ensure_selected_message_reply_shape(
    reply: &str,
    selected_message_id: &str,
    selected_content: &str,
    external_context: Option<&Value>,
) -> String {
    let mut reply = reply.trim().to_string();
    if reply.is_empty() {
        reply = "AI推断：这条消息需要谨慎看待。".to_string();
    }

    reply = ensure_selected_message_source(&reply, selected_message_id);
    reply = ensure_current_context_audit_source(&reply, external_context);
    reply = crate::social_ai::ensure_fb2_grounded_answer_shape(&reply, external_context);

    let review_terms = selected_message_review_terms(selected_content);
    if !review_terms.is_empty() {
        let terms = review_terms.join("、");
        if review_terms.iter().any(|term| !reply.contains(term)) {
            reply.push_str(&format!(
                "\n原文复核：已针对被选择消息中的“{terms}”说法作风险判断。"
            ));
        }
        if !contains_any(&reply, &["不合理", "不应", "过于绝对"]) {
            reply.push_str(&format!(
                "\nAI推断：被选择消息里的“{terms}”表述过于绝对，不合理，不应作为投注结论。"
            ));
        }
    }

    if (!review_terms.is_empty() || external_context.is_some())
        && !contains_any(
            &reply,
            &[
                "风险边界",
                "不保证",
                "不能保证",
                "无法保证",
                "仅供参考",
                "有风险",
                "风险",
            ],
        )
    {
        reply.push_str("\n风险边界：赛果不确定，不保证命中，不建议重注或梭哈。");
    }

    reply
}

pub(super) fn selected_message_review_terms(selected_content: &str) -> Vec<&'static str> {
    let mut terms = Vec::new();
    for term in ["肯定赢盘", "稳赢", "稳赚", "包赢", "保证", "重注", "梭哈"] {
        if selected_content.contains(term) {
            terms.push(term);
        }
    }
    terms
}

pub(super) fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

pub(super) fn mark_in_flight(key: &str) -> bool {
    with_in_flight(|items| items.insert(key.to_string()))
}

pub(super) fn clear_in_flight(key: &str) {
    with_in_flight(|items| {
        items.remove(key);
    });
}

pub(super) fn with_in_flight<T>(operation: impl FnOnce(&mut HashSet<String>) -> T) -> T {
    let mutex = SELECTED_REPLY_IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation(&mut guard)
}

#[cfg(test)]
#[path = "social_ai_message_reply_tests.rs"]
mod tests;
