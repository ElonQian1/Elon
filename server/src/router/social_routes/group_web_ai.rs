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
    match state
        .store
        .prepare_group_web_ai(&user.id, &group, &source, &req.operation_id)
    {
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
    let result = (|| -> anyhow::Result<_> {
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
            if req.action == "fallback" && request.state == "server_ready" {
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
