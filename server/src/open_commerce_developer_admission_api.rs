//! Project and platform-admin HTTP endpoints for developer-App admission review.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_developer_admission_model::{
        ReviewDeveloperAppAdmissionRequest, SubmitDeveloperAppAdmissionRequest,
    },
    open_commerce_developer_admission_service as service,
    open_commerce_developer_credential_model::production_credentials_enabled,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/admission",
            get(current_admission),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/admission/submit",
            post(submit_admission),
        )
        .route(
            "/api/admin/open-commerce/developer-app-admissions",
            get(list_reviewable_admissions),
        )
        .route(
            "/api/admin/open-commerce/developer-app-admissions/:app_record_id/review",
            post(review_admission),
        )
}

async fn current_admission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        service::current_admission(&state.store, &project_id, &app_record_id, &actor(&caller)).map(
            |admission| {
                let production_credential_issued = admission
                    .as_ref()
                    .is_some_and(|value| value.production_credential_issued);
                json!({
                    "schema": "open_commerce.developer_app_admission_state.v1",
                    "admission": admission,
                    "production_credential_issued": production_credential_issued,
                    "network_access_enabled": production_credential_issued && production_credentials_enabled(),
                })
            },
        ),
    )
}

async fn submit_admission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
    Json(request): Json<SubmitDeveloperAppAdmissionRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::submit_admission(
        &state.store,
        &project_id,
        &app_record_id,
        request,
        &actor(&caller),
    ))
}

async fn list_reviewable_admissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    service_response(
        service::list_reviewable_admissions(&state.store, 100).map(|items| {
            json!({
                "schema": "open_commerce.developer_app_admission_review_queue.v1",
                "items": items,
                "production_credentials_enabled": production_credentials_enabled(),
            })
        }),
    )
}

async fn review_admission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_record_id): Path<String>,
    Json(request): Json<ReviewDeveloperAppAdmissionRequest>,
) -> Response {
    let reviewer = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::review_admission(
        &state.store,
        &app_record_id,
        request,
        &reviewer,
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以审核或暂停 App 准入",
        ));
    }
    Ok(user.id)
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

fn service_response<T: Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
