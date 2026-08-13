//! Authenticated owner/admin HTTP surface for inert V258 upstream transport targets.

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

use super::external_pool_adapter_upstream_transport_target_service::{
    self as service, CreateUpstreamTransportTargetBody, RevokeUpstreamTransportTargetBody,
    UpstreamTransportTargetActor, UpstreamTransportTargetServiceError,
};

const OWNER_BINDINGS: &str = "/api/me/compute/external-pool-provider-bindings";
const ADMIN_BINDINGS: &str = "/api/admin/compute/external-pool-provider-bindings";
const PROFILE_PATH: &str =
    "/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{OWNER_BINDINGS}{PROFILE_PATH}/upstream-transport-policy"),
            get(owner_policy),
        )
        .route(
            &format!("{OWNER_BINDINGS}{PROFILE_PATH}/upstream-transport-targets"),
            post(owner_create),
        )
        .route(
            &format!(
                "{OWNER_BINDINGS}{PROFILE_PATH}/upstream-transport-targets/:target_id/currentness"
            ),
            get(owner_currentness),
        )
        .route(
            &format!(
                "{OWNER_BINDINGS}{PROFILE_PATH}/upstream-transport-targets/:target_id/revocation"
            ),
            post(owner_revoke),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{PROFILE_PATH}/upstream-transport-policy"),
            get(admin_policy),
        )
        .route(
            &format!("{ADMIN_BINDINGS}{PROFILE_PATH}/upstream-transport-targets"),
            post(admin_create),
        )
        .route(
            &format!(
                "{ADMIN_BINDINGS}{PROFILE_PATH}/upstream-transport-targets/:target_id/currentness"
            ),
            get(admin_currentness),
        )
        .route(
            &format!(
                "{ADMIN_BINDINGS}{PROFILE_PATH}/upstream-transport-targets/:target_id/revocation"
            ),
            post(admin_revoke),
        )
}

async fn owner_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile)): Path<(String, String, String)>,
) -> Response {
    let actor = match owner(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    read_response(service::policy_summary(
        &state, actor, &binding, &candidate, &profile,
    ))
}

async fn owner_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile)): Path<(String, String, String)>,
    payload: Result<Json<CreateUpstreamTransportTargetBody>, JsonRejection>,
) -> Response {
    let actor = match owner(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::create(&state, actor, &binding, &candidate, &profile, body).await)
}

async fn owner_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<(String, String, String, String)>,
) -> Response {
    let actor = match owner(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    read_response(
        service::currentness(&state, actor, &binding, &candidate, &profile, &target).await,
    )
}

async fn owner_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<(String, String, String, String)>,
    payload: Result<Json<RevokeUpstreamTransportTargetBody>, JsonRejection>,
) -> Response {
    let actor = match owner(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::revoke(
        &state, actor, &binding, &candidate, &profile, &target, body,
    ))
}

async fn admin_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile)): Path<(String, String, String)>,
) -> Response {
    let actor = match admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    read_response(service::policy_summary(
        &state, actor, &binding, &candidate, &profile,
    ))
}

async fn admin_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile)): Path<(String, String, String)>,
    payload: Result<Json<CreateUpstreamTransportTargetBody>, JsonRejection>,
) -> Response {
    let actor = match admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::create(&state, actor, &binding, &candidate, &profile, body).await)
}

async fn admin_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<(String, String, String, String)>,
) -> Response {
    let actor = match admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    read_response(
        service::currentness(&state, actor, &binding, &candidate, &profile, &target).await,
    )
}

async fn admin_revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((binding, candidate, profile, target)): Path<(String, String, String, String)>,
    payload: Result<Json<RevokeUpstreamTransportTargetBody>, JsonRejection>,
) -> Response {
    let actor = match admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    write_response(service::revoke(
        &state, actor, &binding, &candidate, &profile, &target, body,
    ))
}

fn owner(state: &AppState, headers: &HeaderMap) -> Result<UpstreamTransportTargetActor, Response> {
    authenticated_user(state, headers).map(UpstreamTransportTargetActor::ProviderOwner)
}

fn admin(state: &AppState, headers: &HeaderMap) -> Result<UpstreamTransportTargetActor, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can manage external-pool upstream transport targets",
        ));
    }
    Ok(UpstreamTransportTargetActor::PlatformAdmin(user.id))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn write_response(
    result: Result<serde_json::Value, UpstreamTransportTargetServiceError>,
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
    result: Result<serde_json::Value, UpstreamTransportTargetServiceError>,
) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: UpstreamTransportTargetServiceError) -> Response {
    let status = match error {
        UpstreamTransportTargetServiceError::NotFound => StatusCode::NOT_FOUND,
        UpstreamTransportTargetServiceError::Forbidden => StatusCode::FORBIDDEN,
        UpstreamTransportTargetServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        UpstreamTransportTargetServiceError::Conflict(_) => StatusCode::CONFLICT,
        UpstreamTransportTargetServiceError::Task(_)
        | UpstreamTransportTargetServiceError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    json_error(status, error)
}
