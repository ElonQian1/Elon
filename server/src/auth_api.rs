//! 认证 API（登录 / 注册 / 当前用户）
//!
//! 路由（在 router.rs 注册）：
//!   POST /api/auth/login    → 密码登录，返回 JWT token
//!   POST /api/auth/register → 注册新账号
//!   POST /api/auth/trust-current-device → 将当前网页登录态升级为可信 PC 会话
//!   GET  /api/me            → 获取当前登录用户信息

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::{
    external_app_registry::{external_app_by_id, public_external_app_config},
    project_auth::{
        auth_from_headers, bearer_token, json_error, login_inner, register_inner, LoginRequest,
        RegisterRequest,
    },
    types::AppState,
};

pub async fn login(State(state): State<Arc<AppState>>, Json(req): Json<LoginRequest>) -> Response {
    match login_inner(&state, req) {
        Ok((token, expires_at, user)) => Json(serde_json::json!({
            "token": token,
            "expires_at": expires_at,
            "user": user,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    match state.store.external_account_origin_hint(&req.account) {
        Ok(Some(origin)) => {
            let (error, app) = external_app_by_id(&origin.app_id)
                .map(|app| (app.login_hint.to_string(), public_external_app_config(app)))
                .unwrap_or_else(|| {
                    (
                        "账号已在外部项目注册，请使用该项目账号登录".to_string(),
                        serde_json::json!({ "id": origin.app_id }),
                    )
                });
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": error,
                    "code": "external_account_registered",
                    "account_origin": {
                        "app_id": origin.app_id,
                        "account": origin.account,
                        "display_name": origin.display_name,
                        "avatar_url": origin.avatar_url,
                    },
                    "app": app,
                })),
            )
                .into_response();
        }
        Ok(None) => {}
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
    match register_inner(&state, req) {
        Ok((token, expires_at, user)) => Json(serde_json::json!({
            "token": token,
            "expires_at": expires_at,
            "user": user,
        }))
        .into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("UNIQUE constraint failed") {
                json_error(StatusCode::BAD_REQUEST, "账号已被注册")
            } else {
                json_error(StatusCode::BAD_REQUEST, message)
            }
        }
    }
}

pub async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match auth_from_headers(&state, &headers) {
        Ok(user) => Json(serde_json::json!({ "user": user })).into_response(),
        Err(e) => json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}

pub async fn trust_current_device(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(e) = auth_from_headers(&state, &headers) {
        return json_error(StatusCode::UNAUTHORIZED, e.to_string());
    }
    let Some(token) = bearer_token(&headers) else {
        return json_error(StatusCode::UNAUTHORIZED, "缺少 Authorization token");
    };
    if state.owner_token.as_deref() == Some(token) {
        return Json(serde_json::json!({
            "trusted": true,
            "owner_token": true,
            "expires_at": null,
        }))
        .into_response();
    }
    match state.store.trust_session(token, Some("PC Web")) {
        Ok(expires_at) => Json(serde_json::json!({
            "trusted": true,
            "owner_token": false,
            "expires_at": expires_at,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}
