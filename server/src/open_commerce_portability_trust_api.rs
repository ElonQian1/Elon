use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_portability_trust_model::CreateConsumerPortabilityTrustKeyRequest,
    open_commerce_portability_trust_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-trust-keys",
            get(list_trust_keys).post(create_trust_key),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-trust-keys/:record_id/revoke",
            axum::routing::post(revoke_trust_key),
        )
}

async fn list_trust_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_portability_trust_service::list_trust_keys(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(
            |keys| json!({"schema":"open_commerce.consumer_portability_trust_keys.v1","keys":keys}),
        ),
    )
}

async fn create_trust_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerPortabilityTrustKeyRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_trust_service::create_trust_key(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn revoke_trust_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_trust_service::revoke_trust_key(
        &state.store,
        &project_id,
        &record_id,
        &actor(&caller),
    ))
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, access.role))
}

fn actor<'a>(caller: &'a (String, String)) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id: &caller.0,
        app_id: "pc-web",
        project_role: Some(&caller.1),
    }
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
