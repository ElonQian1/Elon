//! Authenticated owner/admin HTTP surface for inert V254 activation candidates.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_provider_activation_candidate_service::{
    self as service, ActivationCandidateServiceError, ActivationPreflightQuery,
    ActivationReadActor, CreateActivationCandidateBody, RevokeActivationDelegationBody,
};

const OWNER_BINDINGS: &str = "/api/me/compute/external-pool-provider-bindings";
const ADMIN_BINDINGS: &str = "/api/admin/compute/external-pool-provider-bindings";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{OWNER_BINDINGS}/:provider_binding_id/activation-candidates"),
            post(create),
        )
        .route(
            &format!(
                "{OWNER_BINDINGS}/:provider_binding_id/activation-candidates/:candidate_id/currentness"
            ),
            get(owner_currentness),
        )
        .route(
            &format!(
                "{OWNER_BINDINGS}/:provider_binding_id/activation-candidates/:candidate_id/preflight"
            ),
            get(owner_preflight),
        )
        .route(
            &format!(
                "{OWNER_BINDINGS}/:provider_binding_id/activation-delegations/:delegation_id/revocation"
            ),
            post(revoke),
        )
        .route(
            &format!(
                "{ADMIN_BINDINGS}/:provider_binding_id/activation-candidates/:candidate_id/currentness"
            ),
            get(admin_currentness),
        )
        .route(
            &format!(
                "{ADMIN_BINDINGS}/:provider_binding_id/activation-candidates/:candidate_id/preflight"
            ),
            get(admin_preflight),
        )
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_binding_id): Path<String>,
    payload: Result<Json<CreateActivationCandidateBody>, JsonRejection>,
) -> Response {
    let owner = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::create_for_owner(&state, &owner, &provider_binding_id, body).await)
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_binding_id, delegation_id)): Path<(String, String)>,
    payload: Result<Json<RevokeActivationDelegationBody>, JsonRejection>,
) -> Response {
    let owner = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(
        service::revoke_for_owner(&state, &owner, &provider_binding_id, &delegation_id, body).await,
    )
}

async fn owner_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_binding_id, candidate_id)): Path<(String, String)>,
) -> Response {
    let owner = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    read_response(
        service::currentness(
            &state,
            ActivationReadActor::ProviderOwner(&owner),
            &provider_binding_id,
            &candidate_id,
        )
        .await,
    )
}

async fn owner_preflight(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_binding_id, candidate_id)): Path<(String, String)>,
    Query(query): Query<ActivationPreflightQuery>,
) -> Response {
    let owner = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    read_response(
        service::preflight(
            &state,
            ActivationReadActor::ProviderOwner(&owner),
            &provider_binding_id,
            &candidate_id,
            query,
        )
        .await,
    )
}

async fn admin_currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_binding_id, candidate_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(
        service::currentness(
            &state,
            ActivationReadActor::PlatformAdmin,
            &provider_binding_id,
            &candidate_id,
        )
        .await,
    )
}

async fn admin_preflight(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_binding_id, candidate_id)): Path<(String, String)>,
    Query(query): Query<ActivationPreflightQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(
        service::preflight(
            &state,
            ActivationReadActor::PlatformAdmin,
            &provider_binding_id,
            &candidate_id,
            query,
        )
        .await,
    )
}

fn write_response(result: Result<serde_json::Value, ActivationCandidateServiceError>) -> Response {
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

fn read_response(result: Result<serde_json::Value, ActivationCandidateServiceError>) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn error_response(error: ActivationCandidateServiceError) -> Response {
    let status = match error {
        ActivationCandidateServiceError::NotFound => StatusCode::NOT_FOUND,
        ActivationCandidateServiceError::Forbidden => StatusCode::FORBIDDEN,
        ActivationCandidateServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        ActivationCandidateServiceError::Conflict(_) => StatusCode::CONFLICT,
        ActivationCandidateServiceError::Task(_) | ActivationCandidateServiceError::Storage(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    json_error(status, error)
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can inspect activation preflight authority",
        ));
    }
    Ok(())
}
