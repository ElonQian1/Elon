use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    store::CreateUiTunerContextArtifact,
    types::AppState,
};

const MAX_CONTEXT_ARTIFACT_BYTES: usize = 256 * 1024;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/modules/ui-tuner/workspace",
            get(get_workspace),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/context-artifacts",
            post(create_context_artifact),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/workspace/legacy-import",
            post(import_legacy_workspace),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/conversations/:conversation_id/fork",
            post(fork_conversation),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/memories/:memory_id",
            patch(review_memory),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWorkspaceBody {
    #[serde(default)]
    stable_summary: String,
    #[serde(default)]
    accepted_decisions: Vec<String>,
    #[serde(default)]
    preferred_standards: Vec<String>,
    #[serde(default)]
    open_questions: Vec<String>,
}

async fn import_legacy_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(body): Json<LegacyWorkspaceBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    if body.stable_summary.chars().count() > 4_000 {
        return json_error(StatusCode::BAD_REQUEST, "旧版 stableSummary 超过 4000 字");
    }
    let accepted_decisions = bounded_memory_values(body.accepted_decisions);
    let preferred_standards = bounded_memory_values(body.preferred_standards);
    let open_questions = bounded_memory_values(body.open_questions);
    match state.store.import_legacy_ui_tuner_memory(
        &project_id,
        &user.id,
        &body.stable_summary,
        &accepted_decisions,
        &preferred_standards,
        &open_questions,
    ) {
        Ok(bundle) => Json(bundle).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn get_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    match state.store.ensure_ui_tuner_workspace(&project_id, &user.id) {
        Ok(bundle) => Json(bundle).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateContextArtifactBody {
    conversation_id: String,
    user_intent: String,
    payload: serde_json::Value,
    selected_element_name: Option<String>,
    resource_id: Option<String>,
    source_file: Option<String>,
}

async fn create_context_artifact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(body): Json<CreateContextArtifactBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    let conversation_id = body.conversation_id.trim();
    let user_intent = body.user_intent.trim();
    if conversation_id.is_empty() || user_intent.is_empty() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "conversationId 和 userIntent 不能为空",
        );
    }
    if body.payload.get("kind").and_then(|value| value.as_str())
        != Some("elon_ui_tuner_codex_context")
    {
        return json_error(StatusCode::BAD_REQUEST, "Context Artifact kind 不正确");
    }
    let payload_json = match serde_json::to_string_pretty(&body.payload) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if payload_json.len() > MAX_CONTEXT_ARTIFACT_BYTES {
        return json_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Context Artifact 超过 256 KiB",
        );
    }
    let payload_sha256 = format!("{:x}", Sha256::digest(payload_json.as_bytes()));
    if let Err(error) = state.store.ensure_ui_tuner_workspace(&project_id, &user.id) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    let input = CreateUiTunerContextArtifact {
        conversation_id,
        schema_version: "elon.ui_tuner.context.v1",
        payload_json: &payload_json,
        payload_sha256: &payload_sha256,
        selected_element_name: body.selected_element_name.as_deref(),
        resource_id: body.resource_id.as_deref(),
        source_file: body.source_file.as_deref(),
        user_intent,
    };
    match state
        .store
        .create_ui_tuner_context_artifact(&project_id, &user.id, input)
    {
        Ok(artifact) => (StatusCode::CREATED, Json(artifact)).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForkConversationBody {
    title: Option<String>,
    selected_element_name: Option<String>,
}

async fn fork_conversation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, conversation_id)): Path<(String, String)>,
    Json(body): Json<ForkConversationBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    let bundle = match state.store.ensure_ui_tuner_workspace(&project_id, &user.id) {
        Ok(bundle) => bundle,
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    if !bundle
        .sessions
        .iter()
        .any(|item| item.conversation_id == conversation_id)
    {
        return json_error(StatusCode::NOT_FOUND, "源 ui-tuner 会话不存在");
    }
    let title = body
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("微调画布 · 分叉会话")
        .chars()
        .take(80)
        .collect::<String>();
    let fork_point =
        match state
            .store
            .latest_ui_tuner_fork_point(&project_id, &user.id, &conversation_id)
        {
            Ok(point) => point,
            Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
    let (new_conversation_id, source_message_id, checkpoint_id) =
        if let Some((checkpoint_id, message_id)) = fork_point {
            match state.store.fork_conversation_at_message(
                &project_id,
                &user.id,
                &conversation_id,
                &message_id,
                None,
                Some(&title),
            ) {
                Ok(result) => (result.conversation_id, Some(message_id), checkpoint_id),
                Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
            }
        } else {
            (state.store.new_ui_tuner_conversation_id(), None, None)
        };
    match state.store.register_ui_tuner_fork(
        &project_id,
        &user.id,
        &new_conversation_id,
        &title,
        &conversation_id,
        source_message_id.as_deref(),
        checkpoint_id.as_deref(),
        body.selected_element_name.as_deref(),
    ) {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewMemoryBody {
    decision: String,
    scope_type: Option<String>,
}

async fn review_memory(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, memory_id)): Path<(String, String)>,
    Json(body): Json<ReviewMemoryBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    match state.store.review_ui_tuner_memory(
        &project_id,
        &user.id,
        &memory_id,
        body.decision.trim(),
        body.scope_type.as_deref().unwrap_or("user"),
    ) {
        Ok(memory) => Json(memory).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn authorized_developer(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<crate::store::PublicUser, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let project = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    if !can_edit(&project.role) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "当前角色不能修改项目或模块记忆",
        ));
    }
    Ok(user)
}

fn bounded_memory_values(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().chars().take(500).collect::<String>())
        .filter(|value| !value.is_empty())
        .take(24)
        .collect()
}
