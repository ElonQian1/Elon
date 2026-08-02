use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    open_commerce_business_handoff_model::RecordBusinessHandoffReceiptRequest,
    open_commerce_business_handoff_service,
    open_commerce_model::normalize_app_id,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct ReceiptQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

struct ProjectCaller {
    user_id: String,
    role: String,
    app_id: String,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/business-handoff-receipts",
            post(record_receipt),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-handoff-receipts",
            get(list_receipts),
        )
}

async fn record_receipt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<RecordBusinessHandoffReceiptRequest>,
) -> Response {
    let caller = match authorize(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_business_handoff_service::record_receipt(
        &state.store,
        &project_id,
        &OpenCommerceActor {
            user_id: &caller.user_id,
            app_id: &caller.app_id,
            project_role: Some(&caller.role),
        },
        request,
    ))
}

async fn list_receipts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Query(query): Query<ReceiptQuery>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_business_handoff_service::list_receipts(
        &state.store,
        &project_id,
        &merchant_id,
        query.limit,
    ))
}

fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("pc-web");
    let app_id =
        normalize_app_id(app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok(ProjectCaller {
        user_id: user.id,
        role: access.role,
        app_id,
    })
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if message.contains("不能用于不同结果") || message.contains("并发冲突")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

fn default_limit() -> usize {
    50
}
