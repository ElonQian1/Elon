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
    open_commerce_merchant_identity_model::CreateMerchantIdentityKeyRequest,
    open_commerce_merchant_identity_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/identity-keys",
            get(list_identity_keys).post(create_identity_key),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/identity-keys/:record_id/revoke",
            axum::routing::post(revoke_identity_key),
        )
}

async fn list_identity_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_merchant_identity_service::list_identity_keys(
            &state.store,
            &project_id,
            &merchant_id,
            &actor,
        )
        .map(|keys| json!({"schema":"open_commerce.merchant_identity_keys.v1","keys":keys}))
    })
}

async fn create_identity_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Json(request): Json<CreateMerchantIdentityKeyRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_merchant_identity_service::create_identity_key(
            &state.store,
            &project_id,
            &merchant_id,
            &actor,
            request,
        )
    })
}

async fn revoke_identity_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id, record_id)): Path<(String, String, String)>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_merchant_identity_service::revoke_identity_key(
            &state.store,
            &project_id,
            &merchant_id,
            &record_id,
            &actor,
        )
    })
}

fn with_caller<T: serde::Serialize>(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    operation: impl FnOnce(OpenCommerceActor<'_>) -> anyhow::Result<T>,
) -> Response {
    let user = match auth_from_headers(state, headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let access = match project_access(state, &user.id, project_id) {
        Ok(access) => access,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    match operation(OpenCommerceActor {
        user_id: &user.id,
        app_id: "pc-web",
        project_role: Some(&access.role),
    }) {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
