//! Context Pack construction and async generation for group summary posts.

use serde_json::{json, Value};
use std::{collections::HashSet, sync::Arc};
use tracing::{info, warn};

use crate::{
    external_app_context_feedback::spawn_generated_answer_feedback,
    social_ai_agents::call_social_chat_llm_with_fallback_options,
    store::{GroupAiDocument, GroupSummaryCreateInput, GroupSummarySourceMessage},
    types::AppState,
};

pub(crate) fn spawn_group_summary_generation(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    post_id: String,
    context_pack: String,
    external_context: Option<Value>,
    sources: Vec<GroupSummarySourceMessage>,
) {
    tokio::spawn(async move {
        info!(
            "group summary generation queued: group_id={} post_id={}",
            group_id, post_id
        );
        let result = generate_group_summary(&state, &user_id, &context_pack).await;
        let (summary, status, model_used, error) = match result {
            Ok((summary, model)) => (
                ensure_fb2_summary_policy_shape(&summary, &context_pack, external_context.as_ref()),
                "ready",
                Some(model),
                None,
            ),
            Err(error) => {
                let fallback = fallback_summary(&sources, &error.to_string());
                (
                    ensure_fb2_summary_policy_shape(
                        &fallback,
                        &context_pack,
                        external_context.as_ref(),
                    ),
                    "ready_with_fallback",
                    None,
                    Some(error.to_string()),
                )
            }
        };
        if let Err(update_error) = state.store.update_group_summary_post_result(
            &group_id,
            &post_id,
            &summary,
            status,
            model_used.as_deref(),
            error.as_deref(),
        ) {
            warn!(
                "group summary result update failed: group_id={} post_id={} error={}",
                group_id, post_id, update_error
            );
        }
        spawn_generated_answer_feedback(
            Arc::clone(&state),
            user_id,
            group_id.clone(),
            format!("social_group_summary_post:{post_id}"),
            "group_summary_post",
            external_context,
            None,
            summary,
            group_summary_feedback_validation_sources(&post_id, &sources),
        );
    });
}

fn group_summary_feedback_validation_sources(
    post_id: &str,
    sources: &[GroupSummarySourceMessage],
) -> Vec<Value> {
    let mut values = Vec::new();
    let post_id = post_id.trim();
    if !post_id.is_empty() {
        values.push(json!({
            "kind": "summary_post",
            "id": post_id,
            "label": "summary post"
        }));
    }
    for source in sources.iter().take(48) {
        let id = source.id.trim();
        if id.is_empty() {
            continue;
        }
        values.push(json!({
            "kind": "group_message",
            "id": id,
            "message_id": id,
            "label": "summary source message"
        }));
    }
    values
}

pub(crate) fn build_context_pack(
    group_id: &str,
    input: &GroupSummaryCreateInput,
    messages: &[GroupSummarySourceMessage],
    documents: &[GroupAiDocument],
    external_context: Option<Value>,
    task: &str,
) -> serde_json::Value {
    let source_start_at = messages.first().map(|message| message.created_at.as_str());
    let source_end_at = messages.last().map(|message| message.created_at.as_str());
    json!({
        "group_id": group_id,
        "task": task,
        "source_window": {
            "start_at": input.start_at.as_deref().or(source_start_at),
            "end_at": input.end_at.as_deref().or(source_end_at)
        },
        "user_instructions": input.instructions.as_deref(),
        "requested_title": input.title.as_deref(),
        "requested_topic": input.topic.as_deref(),
        "retrieval_strategy": {
            "exact_message_ids": !input.message_ids.is_empty(),
            "time_window": input.start_at.is_some() || input.end_at.is_some(),
            "keyword_search_endpoint": "/api/me/groups/:group_id/messages/search",
            "hybrid_layers": [
                "selected_messages",
                "time_window",
                "recent_messages",
                "keyword_full_text",
                "sender_filter",
                "group_ai_documents",
                "external_app_context",
                "future_vector_embedding"
            ],
            "vector_status": "pending_group_chat_embedding_index"
        },
        "source_message_count": messages.len(),
        "selected_messages": messages.iter().map(|message| json!({
            "id": message.id.as_str(),
            "sender_user_id": message.sender_user_id.as_str(),
            "sender_name": message.sender_name.as_str(),
            "created_at": message.created_at.as_str(),
            "content": message.content.as_str()
        })).collect::<Vec<_>>(),
        "group_ai_docs": documents.iter().map(|doc| json!({
            "path": doc.path.as_str(),
            "title": doc.title.as_str(),
            "content": doc.content.as_str()
        })).collect::<Vec<_>>(),
        "external_app_context": external_context,
        "output_contract": {
            "format": "markdown",
            "required_sections": [
                "数据事实",
                "群友观点",
                "AI推断",
                "风险边界",
                "摘要",
                "已达成结论",
                "待确认问题",
                "行动项",
                "相关发言"
            ],
            "citation_required": true,
            "no_fabrication": true,
            "source_reference_required": ["message_id"],
            "fb2_answer_policy": {
                "when_external_app_context_is_fb2": true,
                "must_distinguish": ["比赛事实", "用户订单", "平台汇总", "群友观点", "AI推断"],
                "risk_boundary": "赛果不确定，不保证命中，不建议重注或梭哈"
            }
        }
    })
}

async fn generate_group_summary(
    state: &Arc<AppState>,
    user_id: &str,
    context_pack: &str,
) -> anyhow::Result<(String, String)> {
    let response =
        match request_group_summary_completion(state, user_id, context_pack, "group_summary_post")
            .await
        {
            Ok(response) => response,
            Err(full_error) => {
                if let Some(compact_pack) = compact_group_summary_context_pack(context_pack) {
                    warn!(
                    "group summary full context generation failed, retrying compact context: {}",
                    full_error
                );
                    request_group_summary_completion(
                        state,
                        user_id,
                        &compact_pack,
                        "group_summary_post_compact",
                    )
                    .await
                    .map_err(|compact_error| {
                        anyhow::anyhow!(
                            "full context failed: {}; compact context failed: {}",
                            full_error,
                            compact_error
                        )
                    })?
                } else {
                    return Err(full_error);
                }
            }
        };
    let summary = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    if summary.is_empty() {
        anyhow::bail!("AI 没有返回总结内容");
    }
    let model = response["model"]
        .as_str()
        .filter(|model| !model.trim().is_empty())
        .unwrap_or("social_ai_fallback")
        .to_string();
    Ok((summary.chars().take(6000).collect(), model))
}

async fn request_group_summary_completion(
    state: &Arc<AppState>,
    user_id: &str,
    context_pack: &str,
    feature: &str,
) -> anyhow::Result<Value> {
    call_social_chat_llm_with_fallback_options(
        state,
        &[
            json!({
                "role": "system",
                "content": "你是群聊总结帖 AI。你只能根据 Context Pack 和群聊 AI 文档总结，不得编造。输出中文 Markdown。若 Context Pack 来自 fb2 或包含赛事/赔率/订单/群观点上下文，必须使用这些小节：数据事实、用户订单、平台汇总、群友观点、AI推断、风险边界、摘要、已达成结论、待确认问题、行动项、相关发言；没有对应材料的小节可以写“未在当前来源中出现”。相关发言必须引用消息 ID；涉及比赛、赔率、票据、推荐、预测或今日比赛讨论时，风险边界必须说明赛果不确定、不保证命中、不建议重注或梭哈。"
            }),
            json!({
                "role": "user",
                "content": format!(
                    "请根据下面 Context Pack 生成一个可以在群聊中置顶查看的总结帖。\n\n<context_pack>\n{}\n</context_pack>",
                    context_pack
                )
            }),
        ],
        user_id,
        feature,
        0.2,
        1600,
    )
    .await
}

fn compact_group_summary_context_pack(context_pack: &str) -> Option<String> {
    const MAX_COMPACT_CHARS: usize = 18_000;
    if context_pack.chars().count() <= MAX_COMPACT_CHARS {
        return None;
    }

    let parsed: Value = serde_json::from_str(context_pack).ok()?;
    let selected_messages = parsed
        .get("selected_messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .rev()
                .take(24)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(compact_summary_message)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let group_docs = parsed
        .get("group_ai_docs")
        .and_then(Value::as_array)
        .map(|docs| {
            docs.iter()
                .take(8)
                .map(compact_summary_doc)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let external_app_context = parsed
        .get("external_app_context")
        .map(|value| compact_json_value(value, 9000));

    let compact = json!({
        "schema": "group_summary.compact_context_pack.v1",
        "compaction_reason": "retry_after_provider_error",
        "original_chars": context_pack.chars().count(),
        "group_id": parsed.get("group_id"),
        "task": parsed.get("task"),
        "source_window": parsed.get("source_window"),
        "user_instructions": parsed.get("user_instructions"),
        "requested_title": parsed.get("requested_title"),
        "requested_topic": parsed.get("requested_topic"),
        "source_message_count": parsed.get("source_message_count"),
        "selected_messages": selected_messages,
        "group_ai_docs": group_docs,
        "external_app_context_compact": external_app_context,
        "output_contract": parsed.get("output_contract"),
        "must_preserve": [
            "message_id",
            "match_id",
            "order_id",
            "context_audit_id",
            "数据事实",
            "用户订单",
            "平台汇总",
            "群友观点",
            "AI推断",
            "风险边界"
        ]
    });

    let compact_text = serde_json::to_string_pretty(&compact).ok()?;
    if compact_text.chars().count() >= context_pack.chars().count() {
        None
    } else {
        Some(compact_text)
    }
}

fn compact_summary_message(message: &Value) -> Value {
    json!({
        "id": message.get("id"),
        "sender_user_id": message.get("sender_user_id"),
        "sender_name": message.get("sender_name"),
        "created_at": message.get("created_at"),
        "content": truncate_value_string(message.get("content"), 240),
    })
}

fn compact_summary_doc(doc: &Value) -> Value {
    json!({
        "path": doc.get("path"),
        "title": doc.get("title"),
        "content": truncate_value_string(doc.get("content"), 600),
    })
}

fn compact_json_value(value: &Value, max_chars: usize) -> Value {
    let text = serde_json::to_string(value).unwrap_or_default();
    if text.chars().count() <= max_chars {
        value.clone()
    } else {
        json!({
            "truncated": true,
            "original_chars": text.chars().count(),
            "excerpt": truncate_chars(&text, max_chars)
        })
    }
}

fn truncate_value_string(value: Option<&Value>, max_chars: usize) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|text| truncate_chars(text, max_chars))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn fallback_summary(messages: &[GroupSummarySourceMessage], error: &str) -> String {
    let mut out = String::new();
    out.push_str("## 摘要\n");
    out.push_str("- AI 生成暂时不可用，系统已根据源消息生成可审计的提取式总结。\n");
    out.push_str("- 请管理员或群成员后续编辑本帖，补充结论和行动项。\n\n");
    out.push_str("## 已达成结论\n");
    out.push_str("- 未形成明确结论，或需要人工确认。\n\n");
    out.push_str("## 待确认问题\n");
    out.push_str("- 需要确认本帖是否覆盖同一议题。\n\n");
    out.push_str("## 行动项\n");
    out.push_str("- 未指定。\n\n");
    out.push_str("## 相关发言\n");
    for message in messages.iter().take(12) {
        let excerpt: String = message.content.chars().take(120).collect();
        out.push_str(&format!(
            "- `{}` {}：{}\n",
            message.id,
            message.sender_name,
            excerpt.replace('\n', " ")
        ));
    }
    out.push_str("\n## 生成状态\n");
    out.push_str(&format!("- AI 调用失败：{}\n", error));
    out
}

pub(crate) fn ensure_fb2_summary_policy_shape(
    summary: &str,
    context_pack: &str,
    external_context: Option<&Value>,
) -> String {
    let summary = summary.trim();
    if summary.is_empty() || !context_pack_has_fb2_external_context(context_pack) {
        return summary.to_string();
    }

    let summary = sanitize_unmatched_fb2_source_ids(summary, context_pack, external_context);

    let has_data = contains_any(&summary, &["数据事实", "比赛事实"]);
    let has_opinion = contains_any(&summary, &["群友观点", "相关发言"]);
    let has_inference = contains_any(&summary, &["AI推断", "AI 推断"]);
    let has_risk = contains_any(&summary, &["风险边界", "不保证", "不能保证"]);

    if has_data && has_opinion && has_inference && has_risk {
        return ensure_fb2_summary_context_audit_source(&summary, context_pack);
    }

    let mut sections = Vec::new();
    if !has_data {
        sections.push(
            "## 数据事实\n- 以下总结仅基于 Context Pack 中的 fb2 比赛、赔率、订单摘要和群聊消息；未在来源中出现的信息不作事实断言。"
                .to_string(),
        );
    }
    if !has_opinion {
        sections.push(
            "## 群友观点\n- 群友观点以「相关发言」中列出的消息 ID 为准；未引用的观点不作为结论。"
                .to_string(),
        );
    }
    sections.push(summary.to_string());
    if !has_inference {
        sections
            .push("## AI推断\n- 以上分析只基于当前 fb2 上下文、群聊内容和已引用来源。".to_string());
    }
    if !has_risk {
        sections.push("## 风险边界\n- 赛果不确定，不保证命中，不建议重注或梭哈。".to_string());
    }

    ensure_fb2_summary_context_audit_source(&sections.join("\n\n"), context_pack)
}

fn context_pack_has_fb2_external_context(context_pack: &str) -> bool {
    contains_any(
        context_pack,
        &[
            "fb2.answer_policy.v1",
            "<fb2_context_pack",
            "\"app_id\": \"fb2\"",
            "\"app_id\":\"fb2\"",
        ],
    )
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn ensure_fb2_summary_context_audit_source(summary: &str, context_pack: &str) -> String {
    let Some(context_audit_id) = extract_context_audit_id(context_pack) else {
        return summary.to_string();
    };
    if summary.contains(&context_audit_id) {
        return summary.to_string();
    }
    format!("{summary}\n\n## 来源审计\n- context_audit_id {context_audit_id}")
}

fn extract_context_audit_id(context_pack: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(context_pack).ok()?;
    find_context_audit_id(&parsed)
}

fn find_context_audit_id(value: &Value) -> Option<String> {
    if let Some(id) = value
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        return Some(id.to_string());
    }
    match value {
        Value::Array(items) => items.iter().find_map(find_context_audit_id),
        Value::Object(map) => map.values().find_map(find_context_audit_id),
        _ => None,
    }
}

fn sanitize_unmatched_fb2_source_ids(
    summary: &str,
    context_pack: &str,
    external_context: Option<&Value>,
) -> String {
    let allowed_ids = fb2_summary_allowed_source_ids(context_pack, external_context);
    let chars = summary.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(summary.len());
    let mut index = 0;

    while index < chars.len() {
        if starts_ext_source_token_at(&chars, index) {
            let (token, next_index) = read_ext_source_token(&chars, index);
            push_allowed_or_redacted_source_token(&mut out, &allowed_ids, &token);
            index = next_index;
            continue;
        }
        if starts_uuid_source_token_at(&chars, index) {
            let token = chars[index..index + 36].iter().collect::<String>();
            push_allowed_or_redacted_source_token(&mut out, &allowed_ids, &token);
            index += 36;
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

fn read_ext_source_token(chars: &[char], mut index: usize) -> (String, usize) {
    let start = index;
    index += 4;
    while index < chars.len() && is_source_token_char(chars[index]) {
        index += 1;
    }
    (chars[start..index].iter().collect::<String>(), index)
}

fn push_allowed_or_redacted_source_token(
    out: &mut String,
    allowed_ids: &HashSet<String>,
    token: &str,
) {
    if allowed_ids.contains(&token.to_ascii_lowercase()) {
        out.push_str(token);
    } else {
        out.push_str("未核验来源编号");
    }
}

fn fb2_summary_allowed_source_ids(
    context_pack: &str,
    external_context: Option<&Value>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    let parsed_context;
    let context = if let Some(context) = external_context {
        Some(context)
    } else {
        parsed_context = serde_json::from_str::<Value>(context_pack).ok();
        parsed_context
            .as_ref()
            .and_then(|pack| pack.get("external_app_context"))
    };
    let Some(context) = context else {
        return out;
    };

    if let Some(context_audit_id) = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        out.insert(context_audit_id.to_ascii_lowercase());
    }
    for source in context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for field in ["id", "message_id", "source_message_id", "context_audit_id"] {
            if let Some(value) = source
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                out.insert(value.to_ascii_lowercase());
            }
        }
    }
    out
}

fn starts_ext_source_token_at(chars: &[char], index: usize) -> bool {
    if index + 4 > chars.len() {
        return false;
    }
    if index > 0 && is_source_token_char(chars[index - 1]) {
        return false;
    }
    chars[index].eq_ignore_ascii_case(&'e')
        && chars[index + 1].eq_ignore_ascii_case(&'x')
        && chars[index + 2].eq_ignore_ascii_case(&'t')
        && chars[index + 3] == '-'
}

fn starts_uuid_source_token_at(chars: &[char], index: usize) -> bool {
    const UUID_LEN: usize = 36;
    if index + UUID_LEN > chars.len() {
        return false;
    }
    if index > 0 && is_source_token_char(chars[index - 1]) {
        return false;
    }
    for offset in 0..UUID_LEN {
        let ch = chars[index + offset];
        let valid = match offset {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        };
        if !valid {
            return false;
        }
    }
    if index + UUID_LEN < chars.len() && is_source_token_char(chars[index + UUID_LEN]) {
        return false;
    }
    true
}

fn is_source_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':')
}


#[cfg(test)]
#[path = "group_summary_context_pack_tests.rs"]
mod tests;
