use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_relationship_model::CreateConsumerRelationshipRequest,
    open_commerce_relationship_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-relationships",
            get(list_consumer_relationships).post(create_consumer_relationship),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-relationships/:relationship_id/revoke",
            post(revoke_consumer_relationship),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-relationships",
            get(list_merchant_relationships),
        )
}

async fn list_consumer_relationships(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_relationship_service::list_consumer_relationships(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(|relationships| {
            json!({"schema":"open_commerce.consumer_relationships.v1","relationships":relationships})
        }),
    )
}

async fn create_consumer_relationship(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerRelationshipRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_relationship_service::create_relationship(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn revoke_consumer_relationship(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, relationship_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_relationship_service::revoke_relationship(
        &state.store,
        &project_id,
        &relationship_id,
        &actor(&caller),
    ))
}

async fn list_merchant_relationships(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_relationship_service::list_merchant_relationships(
            &state.store,
            &project_id,
            &merchant_id,
            &actor(&caller),
            100,
        )
        .map(|relationships| {
            json!({"schema":"open_commerce.merchant_relationships.v1","relationships":relationships})
        }),
    )
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
