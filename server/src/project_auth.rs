use axum::{
    Json,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::{
    store::{ProjectAccess, PublicUser},
    types::AppState,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub nickname: Option<String>,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
}

pub fn login_inner(
    state: &AppState,
    req: LoginRequest,
) -> anyhow::Result<(String, String, PublicUser)> {
    let user = state
        .store
        .authenticate_password(&req.account, &req.password)?;
    let (token, expires_at) = state
        .store
        .create_session(&user.id, req.device_name.as_deref(), req.apk_version.as_deref())?;
    Ok((token, expires_at, user))
}

pub fn register_inner(
    state: &AppState,
    req: RegisterRequest,
) -> anyhow::Result<(String, String, PublicUser)> {
    let user = state.store.create_user(
        &req.account,
        &req.password,
        req.nickname.as_deref(),
        Some("user"),
    )?;
    let (token, expires_at) = state
        .store
        .create_session(&user.id, req.device_name.as_deref(), req.apk_version.as_deref())?;
    Ok((token, expires_at, user))
}

pub fn auth_from_headers(state: &AppState, headers: &HeaderMap) -> anyhow::Result<PublicUser> {
    let token = bearer_token(headers).ok_or_else(|| anyhow::anyhow!("缺少 Authorization token"))?;
    state.store.authenticate_token(token)
}

pub fn auth_from_headers_or_query(
    state: &AppState,
    headers: &HeaderMap,
    query: &HashMap<String, String>,
) -> anyhow::Result<PublicUser> {
    if let Some(token) = bearer_token(headers) {
        return state.store.authenticate_token(token);
    }
    let token = query
        .get("token")
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("缺少下载 token"))?;
    state.store.authenticate_token(token)
}

pub fn project_access(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> anyhow::Result<ProjectAccess> {
    state.store.get_project_access(user_id, project_id)
}

pub fn can_edit(role: &str) -> bool {
    matches!(role, "owner" | "editor")
}

pub fn json_error(status: StatusCode, message: impl ToString) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message.to_string()
        })),
    )
        .into_response()
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
