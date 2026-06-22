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

use crate::{
    group_summary_context_pack::{build_context_pack, spawn_group_summary_generation},
    group_summary_topic_split::split_group_summary_topics,
    project_auth::{auth_from_headers, json_error},
    store::GroupSummaryCreateInput,
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
pub struct AutoSplitGroupSummaryPostRequest {
    pub instructions: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: Option<i64>,
    pub max_topics: Option<usize>,
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
    let topic_hint = summary_topic_hint(&input);
    let external_context = crate::external_app_context::group_context_for_chat(
        &state,
        &user.id,
        &group_id,
        topic_hint.as_deref(),
    )
    .await;
    let feedback_external_context = external_context.clone();
    let context_pack = match serde_json::to_string_pretty(&build_context_pack(
        &group_id,
        &input,
        &messages,
        &documents,
        external_context,
        "group_summary_post",
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
        feedback_external_context,
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

pub async fn auto_split_group_summary_posts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group_id): Path<String>,
    Json(req): Json<AutoSplitGroupSummaryPostRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let documents = match state.store.list_group_ai_documents(&user.id, &group_id) {
        Ok(documents) => documents,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let base_input = GroupSummaryCreateInput {
        title: None,
        topic: None,
        instructions: clean_optional(req.instructions),
        message_ids: Vec::new(),
        start_at: clean_optional(req.start_at),
        end_at: clean_optional(req.end_at),
        limit: req.limit.unwrap_or(120),
        pin: req.pin.unwrap_or(true),
    };
    let messages =
        match state
            .store
            .group_summary_messages_for_context(&user.id, &group_id, &base_input)
        {
            Ok(messages) => messages,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };
    let split = split_group_summary_topics(
        &state,
        &user.id,
        &messages,
        &documents,
        req.max_topics.unwrap_or(4),
    )
    .await;
    let mut details = Vec::new();
    for topic in &split.topics {
        let topic_messages = messages
            .iter()
            .filter(|message| topic.message_ids.iter().any(|id| id == &message.id))
            .cloned()
            .collect::<Vec<_>>();
        if topic_messages.is_empty() {
            continue;
        }
        let input = GroupSummaryCreateInput {
            title: Some(topic.title.clone()),
            topic: Some(topic.topic.clone()),
            instructions: base_input.instructions.clone(),
            message_ids: topic.message_ids.clone(),
            start_at: None,
            end_at: None,
            limit: topic_messages.len() as i64,
            pin: base_input.pin,
        };
        let topic_hint = summary_topic_hint(&input);
        let external_context = crate::external_app_context::group_context_for_chat(
            &state,
            &user.id,
            &group_id,
            topic_hint.as_deref(),
        )
        .await;
        let feedback_external_context = external_context.clone();
        let context_pack = match serde_json::to_string_pretty(&build_context_pack(
            &group_id,
            &input,
            &topic_messages,
            &documents,
            external_context,
            "group_summary_post_auto_split",
        )) {
            Ok(pack) => pack,
            Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        let detail = match state.store.create_group_summary_post_draft(
            &user.id,
            &group_id,
            &input,
            &topic_messages,
            &context_pack,
        ) {
            Ok(detail) => detail,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };
        spawn_group_summary_generation(
            state.clone(),
            user.id.clone(),
            group_id.clone(),
            detail.post.id.clone(),
            context_pack,
            feedback_external_context,
            topic_messages,
        );
        details.push(detail);
    }
    if details.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "没有找到可拆分的群聊议题");
    }
    let posts = details
        .iter()
        .map(|detail| detail.post.clone())
        .collect::<Vec<_>>();
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "posts": posts,
            "split": split,
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

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn summary_topic_hint(input: &GroupSummaryCreateInput) -> Option<String> {
    let mut parts = Vec::new();
    push_unique_hint(&mut parts, input.topic.as_deref());
    push_unique_hint(&mut parts, input.title.as_deref());
    push_unique_hint(&mut parts, input.instructions.as_deref());
    let hint = parts.join("；");
    if hint.is_empty() {
        None
    } else {
        Some(hint.chars().take(500).collect())
    }
}

fn push_unique_hint(parts: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !parts.iter().any(|existing| existing == value) {
        parts.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::summary_topic_hint;
    use crate::{
        group_summary_context_pack::ensure_fb2_summary_policy_shape, store::GroupSummaryCreateInput,
    };

    #[test]
    fn summary_topic_hint_prefers_topic_and_adds_instructions() {
        let input = GroupSummaryCreateInput {
            title: Some("今日比赛复盘".into()),
            topic: Some("竞彩焦点".into()),
            instructions: Some("重点看我的票和群友观点".into()),
            message_ids: Vec::new(),
            start_at: None,
            end_at: None,
            limit: 120,
            pin: false,
        };

        assert_eq!(
            summary_topic_hint(&input).as_deref(),
            Some("竞彩焦点；今日比赛复盘；重点看我的票和群友观点")
        );
    }

    #[test]
    fn summary_topic_hint_deduplicates_empty_values() {
        let input = GroupSummaryCreateInput {
            title: Some("今日比赛".into()),
            topic: Some("今日比赛".into()),
            instructions: Some(" ".into()),
            message_ids: Vec::new(),
            start_at: None,
            end_at: None,
            limit: 120,
            pin: false,
        };

        assert_eq!(summary_topic_hint(&input).as_deref(), Some("今日比赛"));
    }

    #[test]
    fn fb2_summary_policy_shape_adds_missing_boundaries() {
        let summary = "## 摘要\n- 今天主要讨论 A 队。";
        let context_pack =
            r#"{"external_app_context":{"answer_policy":{"schema":"fb2.answer_policy.v1"}}}"#;

        let shaped = ensure_fb2_summary_policy_shape(summary, context_pack);

        assert!(shaped.contains("## 数据事实"));
        assert!(shaped.contains("## AI推断"));
        assert!(shaped.contains("## 风险边界"));
        assert!(shaped.contains("不保证命中"));
        assert!(shaped.contains(summary));
    }

    #[test]
    fn non_fb2_summary_policy_shape_is_unchanged() {
        let summary = "## 摘要\n- 普通群总结。";

        assert_eq!(
            ensure_fb2_summary_policy_shape(summary, r#"{"task":"group_summary_post"}"#),
            summary
        );
    }
}
