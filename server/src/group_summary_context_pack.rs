//! Context Pack construction and async generation for group summary posts.

use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    social_ai::resolve_social_agent,
    store::{GroupAiDocument, GroupSummaryCreateInput, GroupSummarySourceMessage},
    types::AppState,
};

pub(crate) fn spawn_group_summary_generation(
    state: Arc<AppState>,
    user_id: String,
    group_id: String,
    post_id: String,
    context_pack: String,
    sources: Vec<GroupSummarySourceMessage>,
) {
    tokio::spawn(async move {
        info!(
            "group summary generation queued: group_id={} post_id={}",
            group_id, post_id
        );
        let result = generate_group_summary(&state, &user_id, &context_pack).await;
        let (summary, status, model_used, error) = match result {
            Ok((summary, model)) => (summary, "ready", Some(model), None),
            Err(error) => {
                let fallback = fallback_summary(&sources, &error.to_string());
                (
                    fallback,
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
    });
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
            "required_sections": ["摘要", "已达成结论", "待确认问题", "行动项", "相关发言"],
            "citation_required": true,
            "no_fabrication": true
        }
    })
}

async fn generate_group_summary(
    state: &Arc<AppState>,
    user_id: &str,
    context_pack: &str,
) -> anyhow::Result<(String, String)> {
    let agent = resolve_social_agent(state).await?;
    let response = call_chat_llm_with_options(
        state,
        &agent,
        &[
            json!({
                "role": "system",
                "content": "你是群聊总结帖 AI。你只能根据 Context Pack 和群聊 AI 文档总结，不得编造。输出中文 Markdown，包含：摘要、已达成结论、待确认问题、行动项、相关发言。相关发言必须引用消息 ID。若 Context Pack 包含外部赛事/赔率上下文，只能作为讨论背景，必须说明不保证结果，不诱导投注。"
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
        "group_summary_post",
        0.2,
        1600,
    )
    .await?;
    let summary = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    if summary.is_empty() {
        anyhow::bail!("AI 没有返回总结内容");
    }
    Ok((summary.chars().take(6000).collect(), agent.model))
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
