use crate::store::social_ai_messages::requests::work::GroupWorkAiOptions;
use crate::{
    project_auth::{auth_from_headers, json_error},
    project_ws_protocol::ProjectAttachmentRef,
    types::AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub(super) struct SendRequest {
    operation_id: String,
    content: String,
    attachments: Option<Vec<ProjectAttachmentRef>>,
}
#[derive(Deserialize)]
pub(super) struct ActionRequest {
    operation_id: String,
    #[serde(default)]
    action: String,
    content: Option<String>,
    work_options: Option<GroupWorkAiOptions>,
    web_provider: Option<String>,
    selected_context:
        Option<crate::store::social_ai_messages::requests::selection::GroupAiSelection>,
}

pub(super) async fn share_context(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, message)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
    };
    match state
        .store
        .group_ai_context_share_draft(&user.id, &group, &message)
    {
        Ok(value) => ([("cache-control", "private, no-store")], Json(value)).into_response(),
        Err(_) => json_error(
            StatusCode::CONFLICT,
            "仅发起人可分享仍有效的精选讨论；原消息可能已变化",
        ),
    }
}

pub(super) async fn work_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if state
        .store
        .friend_group_member_ids(&user.id, &group)
        .is_err()
    {
        return json_error(StatusCode::FORBIDDEN, "无法访问当前群聊");
    }
    let agents = crate::agent_fallback::server_api_agents_in_fallback_order(&state).await;
    Json(serde_json::json!({"schema":1,"models":agents.iter().map(|a|
        serde_json::json!({"id":a.name,"label":a.name,"model":a.model})).collect::<Vec<_>>()}))
    .into_response()
}

pub(super) async fn send(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Json(req): Json<SendRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.send_group_web_ai_message(
        &user.id,
        &group,
        &req.content,
        req.attachments.as_deref(),
        &req.operation_id,
    ) {
        Ok((message, request)) => {
            if let Ok(members) = state.store.friend_group_member_ids(&user.id, &group) {
                crate::friend_events::publish_group_message(&message, members);
            }
            Json(serde_json::json!({"message":message,"request":request})).into_response()
        }
        Err(e) => json_error(StatusCode::CONFLICT, e.to_string()),
    }
}

pub(super) async fn prepare(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, source)): Path<(String, String)>,
    Json(req): Json<ActionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let prepared = match req.selected_context.as_ref() {
        Some(selection) => state.store.prepare_group_ai_selection(
            &user.id,
            &group,
            &source,
            &req.operation_id,
            selection,
        ),
        None => state
            .store
            .prepare_group_web_ai(&user.id, &group, &source, &req.operation_id),
    };
    match prepared {
        Ok(request) => Json(serde_json::json!({"request":request})).into_response(),
        Err(e) => json_error(StatusCode::CONFLICT, e.to_string()),
    }
}

pub(super) async fn action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
    Json(req): Json<ActionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(v) => v,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if req.action == "work" {
        let Some(options) = req.work_options.as_ref() else {
            return json_error(StatusCode::BAD_REQUEST, "缺少工作 AI 设置");
        };
        if let Err(e) = crate::social_ai_agents::resolve_group_work_agent(&state, options).await {
            return json_error(StatusCode::CONFLICT, e.to_string());
        }
    }
    let result = (|| -> anyhow::Result<_> {
        if req.action == "dispatch" {
            return state.store.group_web_ai_provider_action(
                &user.id,
                &group,
                &id,
                &req.operation_id,
                "dispatch",
                req.web_provider.as_deref().unwrap_or("chatgpt_web"),
            );
        }
        if req.action == "work" {
            return state.store.activate_group_work_ai(
                &user.id,
                &group,
                &id,
                &req.operation_id,
                req.work_options.as_ref().expect("validated work options"),
            );
        }
        if req.action == "complete" {
            let current = state.store.group_web_ai_action(
                &user.id,
                &group,
                &id,
                &req.operation_id,
                "status",
            )?;
            anyhow::ensure!(current.engine == "chatgpt_web", "请求已选择其他 AI");
            anyhow::ensure!(
                matches!(
                    current.state.as_str(),
                    "dispatched" | "indeterminate" | "completed"
                ),
                "请求尚未派发"
            );
            let message = state.store.complete_group_ai_reply(
                &user.id,
                &id,
                req.content.as_deref().unwrap_or(""),
            )?;
            let members = state.store.friend_group_member_ids(&user.id, &group)?;
            crate::friend_events::publish_group_message(&message, members);
        }
        state.store.group_web_ai_action(
            &user.id,
            &group,
            &id,
            &req.operation_id,
            if req.action == "complete" {
                "status"
            } else {
                &req.action
            },
        )
    })();
    match result {
        Ok(request) => {
            if matches!(req.action.as_str(), "fallback" | "work") && request.state == "server_ready"
            {
                if let Err(e) = crate::social_ai_message_reply::spawn_group_reply_for_message(
                    state.clone(),
                    user.id.clone(),
                    group,
                    request.trigger_message_id.clone(),
                ) {
                    return json_error(StatusCode::CONFLICT, e.to_string());
                }
            }
            Json(serde_json::json!({"request":request})).into_response()
        }
        Err(e) => json_error(StatusCode::CONFLICT, e.to_string()),
    }
}
