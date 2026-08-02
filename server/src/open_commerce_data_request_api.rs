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
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, DecideConsumerDataRequest,
    },
    open_commerce_data_request_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-requests",
            get(list_consumer_requests).post(create_erasure_request),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-requests/:request_id/withdraw",
            post(withdraw_request),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-requests",
            get(list_merchant_requests),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-requests/:request_id/decision",
            post(decide_request),
        )
}

async fn list_consumer_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_data_request_service::list_consumer_requests(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(|requests| {
            json!({"schema":"open_commerce.consumer_data_requests.v1","requests":requests})
        }),
    )
}

async fn create_erasure_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerDataErasureRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_data_request_service::create_erasure_request(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn withdraw_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_data_request_service::withdraw_request(
        &state.store,
        &project_id,
        &request_id,
        &actor(&caller),
    ))
}

async fn list_merchant_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_data_request_service::list_merchant_requests(
            &state.store,
            &project_id,
            &merchant_id,
            &actor(&caller),
            100,
        )
        .map(|requests| {
            json!({"schema":"open_commerce.merchant_data_requests.v1","requests":requests})
        }),
    )
}

async fn decide_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id, request_id)): Path<(String, String, String)>,
    Json(decision): Json<DecideConsumerDataRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_data_request_service::decide_request(
        &state.store,
        &project_id,
        &merchant_id,
        &request_id,
        &actor(&caller),
        decision,
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
