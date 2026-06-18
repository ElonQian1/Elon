//! HTTP contract for external app integrations such as fb2.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    external_app_registry::{
        external_app_by_id, group_seeds, public_external_app_config, service_token_env_names,
        ExternalAppDefinition,
    },
    project_auth::{auth_from_headers, json_error},
    store::{ExternalAccountSessionInput, ExternalAccountUpsert},
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExternalAccountSyncRequest {
    pub external_user_id: String,
    pub account: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExternalAccountSessionRequest {
    pub external_user_id: String,
    pub account: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExternalAccountLookupRequest {
    pub account: String,
}

#[derive(Debug, Deserialize)]
pub struct ExternalAuthorizeRequest {
    pub scopes: Option<Vec<String>>,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExternalAuthorizeExchangeRequest {
    pub code: String,
}

pub async fn get_external_app(Path(app_id): Path<String>) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    Json(json!({ "app": public_external_app_config(app) })).into_response()
}

pub async fn lookup_external_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAccountLookupRequest>,
) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    if let Err(response) = require_external_app_service_token(app.id, &headers) {
        return response;
    }
    match state.store.external_account_origin_hint(&req.account) {
        Ok(Some(account)) if account.app_id == app.id => Json(json!({
            "registered": true,
            "app": public_external_app_config(app),
            "account": account,
        }))
        .into_response(),
        Ok(_) => Json(json!({
            "registered": false,
            "app": public_external_app_config(app),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn sync_external_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAccountSyncRequest>,
) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    if let Err(response) = require_external_app_service_token(app.id, &headers) {
        return response;
    }
    let input = ExternalAccountUpsert {
        external_user_id: req.external_user_id,
        account: req.account,
        display_name: req.display_name,
        avatar_url: req.avatar_url,
        status: req.status,
    };
    match state.store.upsert_external_app_account(app.id, input) {
        Ok(account) => Json(json!({
            "app": public_external_app_config(app),
            "account": account,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn create_external_account_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAccountSessionRequest>,
) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    if let Err(response) = require_external_app_service_token(app.id, &headers) {
        return response;
    }
    let input = ExternalAccountSessionInput {
        external_user_id: req.external_user_id,
        account: req.account,
        display_name: req.display_name,
        avatar_url: req.avatar_url,
        device_name: req.device_name,
        apk_version: req.apk_version,
    };
    match state
        .store
        .create_external_app_session(app.id, &group_seeds(app), input)
    {
        Ok(session) => Json(json!({
            "app": public_external_app_config(app),
            "token": session.token,
            "expires_at": session.expires_at,
            "user": session.user,
            "account": session.account,
            "default_groups": session.default_groups,
            "trial_credit": session.trial_credit,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn authorize_external_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAuthorizeRequest>,
) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.create_external_app_authorization_code(
        app.id,
        &user.id,
        req.scopes.unwrap_or_default(),
        req.redirect_uri.as_deref(),
    ) {
        Ok(code) => Json(json!({
            "app": public_external_app_config(app),
            "authorization": code,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn exchange_external_app_authorization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Json(req): Json<ExternalAuthorizeExchangeRequest>,
) -> Response {
    let app = match resolve_external_app(&app_id) {
        Ok(app) => app,
        Err(response) => return response,
    };
    if let Err(response) = require_external_app_service_token(app.id, &headers) {
        return response;
    }
    match state
        .store
        .exchange_external_app_authorization_code(app.id, &req.code)
    {
        Ok(exchange) => Json(json!({
            "app": public_external_app_config(app),
            "authorization": exchange,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn resolve_external_app(app_id: &str) -> Result<&'static ExternalAppDefinition, Response> {
    external_app_by_id(app_id).ok_or_else(|| {
        json_error(
            StatusCode::NOT_FOUND,
            format!("未知外部应用：{}", app_id.trim()),
        )
    })
}

fn require_external_app_service_token(app_id: &str, headers: &HeaderMap) -> Result<(), Response> {
    let supplied = external_app_service_token_from_headers(headers).ok_or_else(|| {
        json_error(
            StatusCode::UNAUTHORIZED,
            "缺少外部应用服务令牌，请使用 Authorization: Bearer 或 X-Elon-External-App-Token",
        )
    })?;
    let expected = expected_external_app_service_token(app_id).ok_or_else(|| {
        let names = service_token_env_names(app_id);
        json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "未配置外部应用服务令牌，请设置 {} 或 {}",
                names[0], names[1]
            ),
        )
    })?;
    if supplied == expected {
        Ok(())
    } else {
        Err(json_error(StatusCode::UNAUTHORIZED, "外部应用服务令牌无效"))
    }
}

fn external_app_service_token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-elon-external-app-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().strip_prefix("Bearer "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn expected_external_app_service_token(app_id: &str) -> Option<String> {
    service_token_env_names(app_id)
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
