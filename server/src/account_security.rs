//! First-party account security API shared by Win, Android, and Mobile Web.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

use crate::{
    auth_request_guard::{check_rate_limit, client_key, validate_request_id, AuthRateLimited},
    project_auth::{auth_from_headers, bearer_token},
    store::AccountSecurityError,
    types::AppState,
};

#[path = "openai_chatkit_api.rs"]
pub(crate) mod openai_chatkit_api;

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    current_password: Option<String>,
    new_password: String,
    request_id: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, Deserialize)]
struct RotateRecoveryCodesRequest {
    current_password: Option<String>,
    request_id: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, Deserialize)]
struct RecoverPasswordRequest {
    account: String,
    recovery_code: String,
    new_password: String,
    request_id: String,
    client_instance_id: Option<String>,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, Deserialize)]
struct StartExternalRecoveryRequest {
    account: String,
    client_instance_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfirmRequest {
    #[serde(default)]
    confirm: bool,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/security", get(security_status))
        .route("/api/auth/password", put(change_password))
        .route(
            "/api/auth/password/recovery/start",
            post(start_external_recovery),
        )
        .route("/api/auth/password/recover", post(recover_password))
        .route(
            "/api/auth/recovery-codes/rotate",
            post(rotate_recovery_codes),
        )
        .route("/api/auth/sessions", get(list_sessions))
        .route("/api/auth/sessions/:session_id", delete(revoke_session))
        .route(
            "/api/auth/sessions/revoke-others",
            post(revoke_other_sessions),
        )
        .route("/api/auth/logout", post(logout))
}

async fn security_status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.account_security_snapshot(&user_id, &token) {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn list_sessions(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.list_account_sessions(&user_id, &token) {
        Ok(sessions) => Json(serde_json::json!({
            "schema_version": 1,
            "sessions": sessions,
        }))
        .into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !request.confirm || !validate_request_id(&request.request_id) {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_security_request",
            "必须明确确认并提供有效 request_id",
        );
    }
    match state.store.change_account_password(
        &user_id,
        &token,
        request.current_password.as_deref(),
        &request.new_password,
        &request.request_id,
    ) {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "password_enabled": true,
            "result": result,
        }))
        .into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn rotate_recovery_codes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RotateRecoveryCodesRequest>,
) -> Response {
    let (user_id, _) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !request.confirm || !validate_request_id(&request.request_id) {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_security_request",
            "必须明确确认并提供有效 request_id",
        );
    }
    match state.store.rotate_account_recovery_codes(
        &user_id,
        request.current_password.as_deref(),
        &request.request_id,
    ) {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "one_time_display": !result.replayed,
            "result": result,
        }))
        .into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn recover_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RecoverPasswordRequest>,
) -> Response {
    if !request.confirm || !validate_request_id(&request.request_id) {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_security_request",
            "必须明确确认并提供有效 request_id",
        );
    }
    let key = client_key(
        &headers,
        request.client_instance_id.as_deref(),
        "password-recovery",
    );
    if let Err(error) = check_rate_limit("password_recover", &key, 8, Duration::from_secs(15 * 60))
    {
        return rate_limited_response(error);
    }
    match state.store.recover_account_password(
        &request.account,
        &request.recovery_code,
        &request.new_password,
        &request.request_id,
    ) {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "login_required": true,
            "result": result,
        }))
        .into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn start_external_recovery(
    headers: HeaderMap,
    Json(request): Json<StartExternalRecoveryRequest>,
) -> Response {
    let key = client_key(
        &headers,
        request.client_instance_id.as_deref(),
        "external-password-recovery",
    );
    if let Err(error) = check_rate_limit(
        "password_recovery_start",
        &key,
        5,
        Duration::from_secs(15 * 60),
    ) {
        return rate_limited_response(error);
    }
    let _account_is_intentionally_not_looked_up = request.account.trim();
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "delivery_configured": false,
            "delivery_state": "reserved_not_configured",
            "code": "external_recovery_delivery_unavailable",
            "message": "当前尚未配置邮件或短信恢复服务；请使用离线恢复码，或通过已绑定身份登录。",
        })),
    )
        .into_response()
}

async fn revoke_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .revoke_account_session(&user_id, &token, &session_id)
    {
        Ok(result) => Json(serde_json::json!({ "ok": true, "result": result })).into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn revoke_other_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ConfirmRequest>,
) -> Response {
    if !request.confirm {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "confirmation_required",
            "必须明确确认退出其他设备",
        );
    }
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.revoke_other_account_sessions(&user_id, &token) {
        Ok(revoked) => Json(serde_json::json!({
            "ok": true,
            "revoked_session_count": revoked,
        }))
        .into_response(),
        Err(error) => security_error_response(error),
    }
}

async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let current = match state.store.list_account_sessions(&user_id, &token) {
        Ok(sessions) => sessions.into_iter().find(|session| session.current),
        Err(error) => return security_error_response(error),
    };
    let Some(current) = current else {
        return coded_error(
            StatusCode::UNAUTHORIZED,
            "session_not_found",
            "当前登录会话已经失效",
        );
    };
    match state
        .store
        .revoke_account_session(&user_id, &token, &current.id)
    {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(error) => security_error_response(error),
    }
}

pub(crate) fn authenticated_account(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| coded_error(StatusCode::UNAUTHORIZED, "authentication_required", error))?;
    if user.id == "local-owner" {
        return Err(coded_error(
            StatusCode::BAD_REQUEST,
            "local_owner_not_supported",
            "本地 owner 不使用云端账号安全设置",
        ));
    }
    let token = bearer_token(headers)
        .ok_or_else(|| {
            coded_error(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "缺少 Authorization token",
            )
        })?
        .to_string();
    Ok((user.id, token))
}

fn security_error_response(error: AccountSecurityError) -> Response {
    let (status, code) = match &error {
        AccountSecurityError::InvalidCurrentPassword => {
            (StatusCode::UNAUTHORIZED, "invalid_current_password")
        }
        AccountSecurityError::InvalidRecoveryCode => {
            (StatusCode::BAD_REQUEST, "invalid_recovery_code")
        }
        AccountSecurityError::SessionNotFound => (StatusCode::NOT_FOUND, "session_not_found"),
        AccountSecurityError::InvalidInput(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
        AccountSecurityError::Store(_) | AccountSecurityError::Database(_) => {
            tracing::warn!(error = %error, "账号安全存储操作失败");
            return coded_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "account_security_unavailable",
                "账号安全服务暂时不可用",
            );
        }
    };
    coded_error(status, code, error)
}

fn rate_limited_response(error: AuthRateLimited) -> Response {
    let mut response = coded_error(
        StatusCode::TOO_MANY_REQUESTS,
        "auth_rate_limited",
        error.to_string(),
    );
    if let Ok(value) = HeaderValue::from_str(&error.retry_after_seconds.to_string()) {
        response.headers_mut().insert("retry-after", value);
    }
    response
}

pub(crate) fn coded_error(status: StatusCode, code: &str, message: impl ToString) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message.to_string(),
            "code": code,
        })),
    )
        .into_response()
}
