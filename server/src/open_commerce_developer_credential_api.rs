//! Project and platform-admin endpoints for production developer credentials.

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
    open_commerce_developer_credential_model::{
        production_credentials_enabled, IssueDeveloperProductionCredentialRequest,
        RevokeDeveloperProductionCredentialRequest,
    },
    open_commerce_developer_credential_service as service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/production-credentials",
            get(list_credentials),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/production-credentials/:credential_id/revoke",
            post(revoke_credential),
        )
        .route(
            "/api/admin/open-commerce/developer-apps/:app_record_id/production-credentials/issue",
            post(issue_credential),
        )
}

async fn list_credentials(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        service::list_credentials(&state.store, &project_id, &app_record_id, &actor(&caller)).map(
            |credentials| {
                json!({
                    "schema": "open_commerce.developer_production_credentials.v1",
                    "credentials": credentials,
                    "issuance_enabled": production_credentials_enabled(),
                    "funds_moved": false,
                })
            },
        ),
    )
}

async fn issue_credential(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_record_id): Path<String>,
    Json(request): Json<IssueDeveloperProductionCredentialRequest>,
) -> Response {
    let issuer = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::issue_credential(
        &state.store,
        &app_record_id,
        request,
        &issuer,
    ))
}

async fn revoke_credential(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, credential_id)): Path<(String, String, String)>,
    Json(request): Json<RevokeDeveloperProductionCredentialRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::revoke_credential(
        &state.store,
        &project_id,
        &app_record_id,
        &credential_id,
        request,
        &actor(&caller),
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以签发或轮换生产凭据",
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
