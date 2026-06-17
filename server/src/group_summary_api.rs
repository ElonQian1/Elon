//! Group chat AI documents, Context Packs, and summary posts.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    project_auth::{auth_from_headers, json_error},
    social_ai::resolve_social_agent,
    store::{GroupAiDocument, GroupSummaryCreateInput, GroupSummarySourceMessage},
    types::AppState,
};

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateGroupAiDocumentRequest {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct CreateGroupSummaryPostRequest {
    pub title: Option<String>,
    pub topic: Option<String>,
    pub instructions: Option<String>,
    pub message_ids: Option<Vec<String>>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: Option<i64>,
    pub pin: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateGroupSummaryPostRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub pinned: Option<bool>,
}

pub async fn list_group_ai_documents(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_group_ai_documents(&user.id, &group_id) {
        Ok(documents) => Json(json!({ "documents": documents })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn update_group_ai_document(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(req): Json<UpdateGroupAiDocumentRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .update_group_ai_document(&user.id, &group_id, &req.path, &req.content)
    {
        Ok(document) => Json(json!({ "document": document })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn list_group_summary_posts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .list_group_summary_posts(&user.id, &group_id, query.limit.unwrap_or(50))
    {
        Ok(posts) => Json(json!({ "posts": posts })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn create_group_summary_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(req): Json<CreateGroupSummaryPostRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let input = GroupSummaryCreateInput {
        title: clean_optional(req.title),
        topic: clean_optional(req.topic),
        instructions: clean_optional(req.instructions),
        message_ids: req.message_ids.unwrap_or_default(),
        start_at: clean_optional(req.start_at),
        end_at: clean_optional(req.end_at),
        limit: req.limit.unwrap_or(120),
        pin: req.pin.unwrap_or(false),
    };
    let documents = match state.store.list_group_ai_documents(&user.id, &group_id) {
        Ok(documents) => documents,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let messages = match state
        .store
        .group_summary_messages_for_context(&user.id, &group_id, &input)
    {
        Ok(messages) => messages,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let context_pack = match serde_json::to_string_pretty(&build_context_pack(
        &group_id, &input, &messages, &documents,
    )) {
        Ok(pack) => pack,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let detail = match state.store.create_group_summary_post_draft(
        &user.id,
        &group_id,
        &input,
        &messages,
        &context_pack,
    ) {
        Ok(detail) => detail,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    spawn_group_summary_generation(
        state,
        user.id,
        group_id,
        detail.post.id.clone(),
        context_pack,
        messages,
    );
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "post": detail.post,
            "context_pack": detail.context_pack,
            "sources": detail.sources,
        })),
    )
        .into_response()
}

pub async fn get_group_summary_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_id, post_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state
        .store
        .group_summary_post_detail(&user.id, &group_id, &post_id)
    {
        Ok(detail) => Json(json!({
            "post": detail.post,
            "context_pack": detail.context_pack,
            "sources": detail.sources,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn update_group_summary_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_id, post_id)): Path<(String, String)>,
    Json(req): Json<UpdateGroupSummaryPostRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.edit_group_summary_post(
        &user.id,
        &group_id,
        &post_id,
        req.title.as_deref(),
        req.summary.as_deref(),
        req.pinned,
    ) {
        Ok(detail) => Json(json!({
            "post": detail.post,
            "context_pack": detail.context_pack,
            "sources": detail.sources,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn spawn_group_summary_generation(
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
                "content": "你是群聊总结帖 AI。你只能根据 Context Pack 和群聊 AI 文档总结，不得编造。输出中文 Markdown，包含：摘要、已达成结论、待确认问题、行动项、相关发言。相关发言必须引用消息 ID。"
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

fn build_context_pack(
    group_id: &str,
    input: &GroupSummaryCreateInput,
    messages: &[GroupSummarySourceMessage],
    documents: &[GroupAiDocument],
) -> serde_json::Value {
    let source_start_at = messages.first().map(|message| message.created_at.as_str());
    let source_end_at = messages.last().map(|message| message.created_at.as_str());
    json!({
        "group_id": group_id,
        "task": "group_summary_post",
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
            "hybrid_layers": [
                "selected_messages",
                "time_window",
                "group_ai_documents",
                "future_keyword_full_text",
                "future_vector_embedding"
            ],
            "vector_status": "pending_group_chat_index"
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
        "output_contract": {
            "format": "markdown",
            "required_sections": ["摘要", "已达成结论", "待确认问题", "行动项", "相关发言"],
            "citation_required": true,
            "no_fabrication": true
        }
    })
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

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
