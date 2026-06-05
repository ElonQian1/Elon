use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

use crate::{
    ai_cli::codex_thread_uri,
    project_auth::{auth_from_headers, json_error, project_access},
    project_mobile::ensure_mobile_project,
    store::{ProjectAccess, PublicUser},
    types::AppState,
};

pub async fn conversation_identity_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath((project_id, conversation_id)): AxumPath<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    conversation_identity_response(state, user, project, conversation_id, None).await
}

pub async fn conversation_identity_user_project(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id, conversation_id)): AxumPath<(String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    conversation_identity_response(
        state,
        user,
        project,
        conversation_id,
        query.get("conversation_title").map(String::as_str),
    )
    .await
}

async fn conversation_identity_response(
    state: Arc<AppState>,
    user: PublicUser,
    project: ProjectAccess,
    conversation_id: String,
    conversation_title: Option<&str>,
) -> Response {
    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user.id,
        Some(&conversation_id),
        conversation_title,
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let native_thread_id = match state.store.latest_native_agent_session_for_conversation(
        &project.id,
        &user.id,
        &conversation_id,
        "codex",
    ) {
        Ok(value) => value,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let task_thread_id =
        match state
            .store
            .latest_task_codex_thread_id(&project.id, &user.id, &conversation_id)
        {
            Ok(value) => value,
            Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
    let codex_thread_id = native_thread_id
        .as_deref()
        .or(task_thread_id.as_deref())
        .map(ToOwned::to_owned);
    let codex_thread_uri = codex_thread_id.as_deref().map(codex_thread_uri);

    Json(json!({
        "status": "ok",
        "project_id": project.id,
        "project_title": project.name,
        "conversation_id": conversation_id,
        "conversation_title": conversation_title,
        "codex_thread_id": codex_thread_id,
        "codex_thread_uri": codex_thread_uri,
        "native_thread_id": native_thread_id,
        "task_codex_thread_id": task_thread_id,
    }))
    .into_response()
}
