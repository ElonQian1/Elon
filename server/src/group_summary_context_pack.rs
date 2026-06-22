//! Context Pack construction and async generation for group summary posts.

use serde_json::{json, Value};
use std::sync::Arc;
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
                ensure_fb2_summary_policy_shape(&summary, &context_pack),
                "ready",
                Some(model),
                None,
            ),
            Err(error) => {
                let fallback = fallback_summary(&sources, &error.to_string());
                (
                    ensure_fb2_summary_policy_shape(&fallback, &context_pack),
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
            group_summary_feedback_citation_sources(&post_id, &sources),
        );
    });
}

fn group_summary_feedback_citation_sources(
    _post_id: &str,
    _sources: &[GroupSummarySourceMessage],
) -> Vec<Value> {
    // Summary post identity is already carried by main_request_id. Do not add
    // main-project local gmsg_* rows as fb2 cited_sources: fb2 matches feedback
    // citations against its Context Pack audit registry, so arbitrary visible
    // chat IDs pollute quality metrics as unmatched sources.
    Vec::new()
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

pub(crate) fn ensure_fb2_summary_policy_shape(summary: &str, context_pack: &str) -> String {
    let summary = summary.trim();
    if summary.is_empty() || !context_pack_has_fb2_external_context(context_pack) {
        return summary.to_string();
    }

    let has_data = contains_any(summary, &["数据事实", "比赛事实"]);
    let has_opinion = contains_any(summary, &["群友观点", "相关发言"]);
    let has_inference = contains_any(summary, &["AI推断", "AI 推断"]);
    let has_risk = contains_any(summary, &["风险边界", "不保证", "不能保证"]);

    if has_data && has_opinion && has_inference && has_risk {
        return summary.to_string();
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

    sections.join("\n\n")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn source_message(id: &str, sender_name: &str) -> GroupSummarySourceMessage {
        GroupSummarySourceMessage {
            id: id.to_string(),
            group_id: "official".to_string(),
            sender_user_id: format!("user-{sender_name}"),
            sender_name: sender_name.to_string(),
            content: "今天比赛怎么看".to_string(),
            created_at: "2026-06-22T09:20:00Z".to_string(),
        }
    }

    #[test]
    fn summary_feedback_extra_citations_exclude_main_group_messages() {
        let citations = group_summary_feedback_citation_sources(
            "gsp-summary-1",
            &[
                source_message("gmsg-1", "用户A"),
                source_message("gmsg-2", "用户B"),
            ],
        );

        assert!(citations.is_empty());
    }

    #[test]
    fn compact_summary_context_skips_small_context() {
        let context = serde_json::json!({
            "group_id": "ext_fb2_official",
            "selected_messages": [{
                "id": "gmsg-1",
                "sender_name": "用户A",
                "content": "今天比赛怎么看"
            }]
        })
        .to_string();

        assert!(compact_group_summary_context_pack(&context).is_none());
    }

    #[test]
    fn compact_summary_context_preserves_source_ids_and_truncates_large_payload() {
        let long_text = "今天比赛怎么看，赔率有什么变化。".repeat(1200);
        let context = serde_json::json!({
            "group_id": "ext_fb2_official",
            "task": "summary_post",
            "source_message_count": 40,
            "selected_messages": [{
                "id": "gmsg-source-1",
                "sender_user_id": "usr-1",
                "sender_name": "用户A",
                "created_at": "2026-06-22T11:42:00Z",
                "content": long_text
            }],
            "external_app_context": {
                "app_id": "fb2",
                "context_audit_id": "audit-1",
                "payload": "赔率与订单摘要".repeat(2200)
            },
            "output_contract": {
                "required_sections": ["数据事实", "AI推断", "风险边界"]
            }
        })
        .to_string();

        let compact = compact_group_summary_context_pack(&context).expect("compact context");

        assert!(compact.len() < context.len());
        assert!(compact.contains("group_summary.compact_context_pack.v1"));
        assert!(compact.contains("gmsg-source-1"));
        assert!(compact.contains("audit-1"));
        assert!(compact.contains("message_id"));
        assert!(compact.contains("风险边界"));
        assert!(compact.contains("truncated"));
    }
}
