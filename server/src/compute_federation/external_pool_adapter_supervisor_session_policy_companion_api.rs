//! Authenticated owner/admin HTTP surface for inert V259 policy companions.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_supervisor_session_policy_companion_service::{
    self as service, CreateSupervisorSessionPolicyCompanionBody,
    RevokeSupervisorSessionPolicyCompanionBody, SupervisorSessionPolicyCompanionActor,
    SupervisorSessionPolicyCompanionServiceError,
};

const OWNER_BINDINGS: &str = "/api/me/compute/external-pool-provider-bindings";
const ADMIN_BINDINGS: &str = "/api/admin/compute/external-pool-provider-bindings";
const TARGET_PATH: &str = "/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{OWNER_BINDINGS}{TARGET_PATH}/supervisor-session-policy"),
            get(owner_policy),
        )
        .route(
            &format!("{OWNER_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions"),
            post(owner_create),
        )
        .route(
            &format!("{OWNER_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions/:companion_id/currentness"),
            get(owner_currentness),
        )
        .route(
            &format!("{OWNER_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions/:companion_id/revocation"),
            post(owner_revoke),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{TARGET_PATH}/supervisor-session-policy"),
            get(admin_policy),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions"),
            post(admin_create),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions/:companion_id/currentness"),
            get(admin_currentness),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{TARGET_PATH}/supervisor-session-policy-companions/:companion_id/revocation"),
            post(admin_revoke),
        )
}

type TargetPath = (String, String, String, String);
type CompanionPath = (String, String, String, String, String);

async fn owner_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<TargetPath>,
) -> Response {
    dispatch_policy(
        &state,
        owner(&state, &headers),
        [&binding, &candidate, &profile, &target],
    )
}

async fn owner_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<TargetPath>,
    payload: Result<Json<CreateSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    dispatch_create(
        &state,
        owner(&state, &headers),
        [&binding, &candidate, &profile, &target],
        payload,
    )
    .await
}

async fn owner_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion)): Path<CompanionPath>,
) -> Response {
    dispatch_currentness(
        &state,
        owner(&state, &headers),
        [&binding, &candidate, &profile, &target],
        &companion,
    )
    .await
}

async fn owner_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion)): Path<CompanionPath>,
    payload: Result<Json<RevokeSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    dispatch_revoke(
        &state,
        owner(&state, &headers),
        [&binding, &candidate, &profile, &target],
        &companion,
        payload,
    )
}

async fn admin_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<TargetPath>,
) -> Response {
    dispatch_policy(
        &state,
        admin(&state, &headers),
        [&binding, &candidate, &profile, &target],
    )
}

async fn admin_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<TargetPath>,
    payload: Result<Json<CreateSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    dispatch_create(
        &state,
        admin(&state, &headers),
        [&binding, &candidate, &profile, &target],
        payload,
    )
    .await
}

async fn admin_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion)): Path<CompanionPath>,
) -> Response {
    dispatch_currentness(
        &state,
        admin(&state, &headers),
        [&binding, &candidate, &profile, &target],
        &companion,
    )
    .await
}

async fn admin_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target, companion)): Path<CompanionPath>,
    payload: Result<Json<RevokeSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    dispatch_revoke(
        &state,
        admin(&state, &headers),
        [&binding, &candidate, &profile, &target],
        &companion,
        payload,
    )
}

fn dispatch_policy(
    state: &AppState,
    actor: Result<SupervisorSessionPolicyCompanionActor, Response>,
    path: [&str; 4],
) -> Response {
    match actor {
        Ok(actor) => read_response(service::policy_summary(state, actor, path)),
        Err(response) => response,
    }
}

async fn dispatch_create(
    state: &AppState,
    actor: Result<SupervisorSessionPolicyCompanionActor, Response>,
    path: [&str; 4],
    payload: Result<Json<CreateSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    let actor = match actor {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::create(state, actor, path, body).await)
}

async fn dispatch_currentness(
    state: &AppState,
    actor: Result<SupervisorSessionPolicyCompanionActor, Response>,
    path: [&str; 4],
    companion: &str,
) -> Response {
    match actor {
        Ok(actor) => read_response(service::currentness(state, actor, path, companion).await),
        Err(response) => response,
    }
}

fn dispatch_revoke(
    state: &AppState,
    actor: Result<SupervisorSessionPolicyCompanionActor, Response>,
    path: [&str; 4],
    companion: &str,
    payload: Result<Json<RevokeSupervisorSessionPolicyCompanionBody>, JsonRejection>,
) -> Response {
    let actor = match actor {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::revoke(state, actor, path, companion, body))
}

fn owner(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SupervisorSessionPolicyCompanionActor, Response> {
    auth_from_headers(state, headers)
        .map(|user| SupervisorSessionPolicyCompanionActor::ProviderOwner(user.id))
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SupervisorSessionPolicyCompanionActor, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can manage external-pool supervisor/session policy companions",
        ));
    }
    Ok(SupervisorSessionPolicyCompanionActor::PlatformAdmin(
        user.id,
    ))
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn write_response(
    result: Result<serde_json::Value, SupervisorSessionPolicyCompanionServiceError>,
) -> Response {
    match result {
        Ok(output) => {
            let status = if output
                .get("replayed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(output)).into_response()
        }
        Err(error) => error_response(error),
    }
}

fn read_response(
    result: Result<serde_json::Value, SupervisorSessionPolicyCompanionServiceError>,
) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: SupervisorSessionPolicyCompanionServiceError) -> Response {
    let status = match error {
        SupervisorSessionPolicyCompanionServiceError::NotFound => StatusCode::NOT_FOUND,
        SupervisorSessionPolicyCompanionServiceError::Forbidden => StatusCode::FORBIDDEN,
        SupervisorSessionPolicyCompanionServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        SupervisorSessionPolicyCompanionServiceError::Conflict(_) => StatusCode::CONFLICT,
        SupervisorSessionPolicyCompanionServiceError::Task(_)
        | SupervisorSessionPolicyCompanionServiceError::Storage(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    json_error(status, error)
}
