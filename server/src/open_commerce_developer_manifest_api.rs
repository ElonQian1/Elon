//! Project and platform-admin HTTP endpoints for developer-App manifest review.

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
    open_commerce_developer_domain_service, open_commerce_developer_manifest_service as service,
    open_commerce_developer_model::{
        IssueDeveloperAppDomainChallengeRequest, ReviewDeveloperAppManifestRequest,
        SubmitDeveloperAppManifestRequest, UpdateDeveloperAppManifestRequest,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/manifest",
            post(update_manifest),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/manifest/submit",
            post(submit_manifest),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/manifest/domain-challenge",
            post(issue_domain_challenge),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/manifest/domain-challenge/verify",
            post(verify_domain_challenge),
        )
        .route(
            "/api/admin/open-commerce/developer-app-manifests",
            get(list_submitted_manifests),
        )
        .route(
            "/api/admin/open-commerce/developer-app-manifests/:app_record_id/review",
            post(review_manifest),
        )
        .merge(crate::open_commerce_developer_admission_api::routes())
}

async fn issue_domain_challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
    Json(request): Json<IssueDeveloperAppDomainChallengeRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_developer_domain_service::issue_challenge(
        &state.store,
        &project_id,
        &app_record_id,
        request.expected_manifest_revision,
        &actor(&caller),
    ))
}

async fn verify_domain_challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_developer_domain_service::verify_domain(
            &state.store,
            &project_id,
            &app_record_id,
            &actor(&caller),
        )
        .await,
    )
}

async fn update_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
    Json(request): Json<UpdateDeveloperAppManifestRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::update_manifest(
        &state.store,
        &project_id,
        &app_record_id,
        request,
        &actor(&caller),
    ))
}

async fn submit_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
    Json(request): Json<SubmitDeveloperAppManifestRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::submit_manifest(
        &state.store,
        &project_id,
        &app_record_id,
        request.expected_manifest_revision,
        &actor(&caller),
    ))
}

async fn list_submitted_manifests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    service_response(
        state
            .store
            .list_submitted_open_commerce_developer_app_manifests(100)
            .map(|apps| {
                json!({
                    "schema":"open_commerce.developer_app_manifest_review_queue.v1",
                    "apps":apps
                })
            }),
    )
}

async fn review_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_record_id): Path<String>,
    Json(request): Json<ReviewDeveloperAppManifestRequest>,
) -> Response {
    let reviewer = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::review_manifest(
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
            "只有平台管理员可以审核 App 资料",
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
