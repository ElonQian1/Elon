//! Public first-party federated login contract shared by Win, Android and Web.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

use crate::{
    auth_request_guard::{check_rate_limit, client_key, validate_request_id, AuthRateLimited},
    federated_auth_google::{verify_google_id_token, GoogleIdentityError, GoogleOidcConfig},
    federated_auth_idempotency::{completion_cache, completion_cache_key},
    project_auth::{auth_from_headers, json_error},
    store::IdentityError,
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct ChallengeRequest {
    mode: String,
    platform: String,
    request_id: Option<String>,
    client_instance_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompleteRequest {
    challenge_id: String,
    id_token: String,
    device_name: Option<String>,
    apk_version: Option<String>,
    #[serde(default)]
    remember_device: bool,
    request_id: Option<String>,
    client_instance_id: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/federation/providers", get(providers))
        .route(
            "/api/auth/federation/google/challenges",
            post(create_google_challenge),
        )
        .route(
            "/api/auth/federation/google/complete",
            post(complete_google),
        )
        .route("/api/auth/identities", get(list_identities))
        .route("/api/auth/identities/:identity_id", delete(unlink_identity))
}

async fn providers() -> Response {
    let config = GoogleOidcConfig::from_env();
    Json(serde_json::json!({
        "schema_version": 1,
        "providers": [{
            "id": "google",
            "configured": config.is_some(),
            "login": true,
            "bind": true,
            "platforms": ["windows", "android", "web"],
            "client_id": config.as_ref().map(GoogleOidcConfig::primary_client_id),
            "credential_storage": "provider_managed",
        }],
        "account_linking": {
            "automatic_email_linking": false,
            "requires_existing_session_for_bind": true,
        },
        "request_safety": {
            "request_id_supported": true,
            "completion_replay_window_seconds": 300,
            "rate_limit": "process_local_plus_perimeter_required",
        }
    }))
    .into_response()
}

async fn create_google_challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChallengeRequest>,
) -> Response {
    if GoogleOidcConfig::from_env().is_none() {
        return json_error(StatusCode::SERVICE_UNAVAILABLE, "Google 登录尚未配置");
    }
    if !matches!(request.mode.as_str(), "login" | "bind")
        || !matches!(request.platform.as_str(), "windows" | "android" | "web")
    {
        return json_error(StatusCode::BAD_REQUEST, "无效的登录模式或平台");
    }
    if request
        .request_id
        .as_deref()
        .is_some_and(|value| !validate_request_id(value))
    {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_id",
            "request_id 格式无效",
        );
    }
    let user_id = if request.mode == "bind" {
        match auth_from_headers(&state, &headers) {
            Ok(user) if user.id != "local-owner" => Some(user.id),
            Ok(_) => return json_error(StatusCode::BAD_REQUEST, "本地 owner 不能绑定云端身份"),
            Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
        }
    } else {
        None
    };
    let key = client_key(
        &headers,
        request.client_instance_id.as_deref(),
        "federated-challenge",
    );
    if let Err(error) = check_rate_limit(
        &format!("google_challenge_{}", request.mode),
        user_id.as_deref().unwrap_or(&key),
        10,
        Duration::from_secs(10 * 60),
    ) {
        let _ = state.store.record_identity_audit_event(
            user_id.as_deref(),
            "google",
            &format!("{}_challenge", request.mode),
            "rate_limited",
            request.request_id.as_deref(),
            Some("auth_rate_limited"),
        );
        return rate_limited_response(error);
    }
    match state.store.create_identity_challenge(
        "google",
        &request.mode,
        user_id.as_deref(),
        &request.platform,
    ) {
        Ok(challenge) => Json(challenge).into_response(),
        Err(error) => identity_error_response(error),
    }
}

async fn complete_google(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CompleteRequest>,
) -> Response {
    if request
        .request_id
        .as_deref()
        .is_some_and(|value| !validate_request_id(value))
    {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_id",
            "request_id 格式无效",
        );
    }
    let client_key = client_key(
        &headers,
        request.client_instance_id.as_deref(),
        "federated-complete",
    );
    if let Err(error) = check_rate_limit(
        "google_complete",
        &client_key,
        16,
        Duration::from_secs(10 * 60),
    ) {
        let _ = state.store.record_identity_audit_event(
            None,
            "google",
            "complete",
            "rate_limited",
            request.request_id.as_deref(),
            Some("auth_rate_limited"),
        );
        return rate_limited_response(error);
    }
    let cache_key = request
        .request_id
        .as_deref()
        .map(|request_id| completion_cache_key(&request.challenge_id, request_id, &client_key));
    let mut cache_guard = if cache_key.is_some() {
        Some(completion_cache().lock().await)
    } else {
        None
    };
    if let (Some(key), Some(cache)) = (cache_key.as_deref(), cache_guard.as_mut()) {
        if let Some(cached) = cache.get(key) {
            if cached.mode == "bind" {
                let current_user = match auth_from_headers(&state, &headers) {
                    Ok(user) => user,
                    Err(error) => {
                        return coded_error(
                            StatusCode::UNAUTHORIZED,
                            "authentication_required",
                            error,
                        )
                    }
                };
                if cached.user_id.as_deref() != Some(current_user.id.as_str()) {
                    return coded_error(
                        StatusCode::FORBIDDEN,
                        "challenge_owner_mismatch",
                        "该绑定请求不属于当前账号",
                    );
                }
            }
            let mut response = cached.response;
            response["idempotent_replay"] = serde_json::Value::Bool(true);
            return Json(response).into_response();
        }
    }
    let challenge = match state.store.identity_challenge(&request.challenge_id) {
        Ok(challenge) if challenge.provider == "google" => challenge,
        Ok(_) => return json_error(StatusCode::BAD_REQUEST, "登录挑战与 Provider 不匹配"),
        Err(error) => return identity_error_response(error),
    };
    if challenge.mode == "bind" {
        let current_user = match auth_from_headers(&state, &headers) {
            Ok(user) => user,
            Err(error) => {
                return coded_error(StatusCode::UNAUTHORIZED, "authentication_required", error)
            }
        };
        if challenge.user_id.as_deref() != Some(current_user.id.as_str()) {
            return coded_error(
                StatusCode::FORBIDDEN,
                "challenge_owner_mismatch",
                "该绑定挑战不属于当前账号",
            );
        }
    }
    let verified = match verify_google_id_token(&request.id_token, &challenge.nonce_hash).await {
        Ok(identity) => identity,
        Err(error) => return google_error_response(error),
    };
    let completion = match state
        .store
        .complete_identity_challenge(&request.challenge_id, &verified)
    {
        Ok(completion) => completion,
        Err(error) => return identity_error_response(error),
    };
    let session = if challenge.mode == "login" {
        match state.store.create_session_with_trust(
            &completion.user.id,
            request.device_name.as_deref(),
            request.apk_version.as_deref(),
            request.remember_device,
        ) {
            Ok((token, expires_at)) => Some(serde_json::json!({
                "token": token,
                "expires_at": expires_at,
            })),
            Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
        }
    } else {
        None
    };
    let response = serde_json::json!({
        "mode": challenge.mode.clone(),
        "user": completion.user,
        "identity": completion.identity,
        "created_user": completion.created_user,
        "session": session,
        "idempotent_replay": false,
    });
    if let (Some(key), Some(cache)) = (cache_key, cache_guard.as_mut()) {
        cache.insert(
            key,
            &challenge.mode,
            challenge.user_id.as_deref(),
            response.clone(),
        );
    }
    Json(response).into_response()
}

async fn list_identities(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) if user.id != "local-owner" => user,
        Ok(_) => return Json(serde_json::json!({ "identities": [] })).into_response(),
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    match state.store.list_linked_identities(&user.id) {
        Ok(identities) => Json(serde_json::json!({ "identities": identities })).into_response(),
        Err(error) => identity_error_response(error),
    }
}

async fn unlink_identity(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(identity_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) if user.id != "local-owner" => user,
        Ok(_) => return json_error(StatusCode::BAD_REQUEST, "本地 owner 没有云端身份"),
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    match state.store.unlink_identity(&user.id, &identity_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => identity_error_response(error),
    }
}

fn identity_error_response(error: IdentityError) -> Response {
    let status = match &error {
        IdentityError::InvalidChallenge => StatusCode::BAD_REQUEST,
        IdentityError::IdentityOwnedByAnother | IdentityError::ExistingAccountRequiresBind => {
            StatusCode::CONFLICT
        }
        IdentityError::CannotUnlinkLastLogin => StatusCode::UNPROCESSABLE_ENTITY,
        IdentityError::IdentityNotFound => StatusCode::NOT_FOUND,
        IdentityError::Store(_) | IdentityError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::warn!(error = %error, "联合身份存储操作失败");
        coded_error(status, "identity_service_unavailable", "身份服务暂时不可用")
    } else {
        let code = match &error {
            IdentityError::InvalidChallenge => "invalid_or_consumed_challenge",
            IdentityError::IdentityOwnedByAnother => "identity_owned_by_another_account",
            IdentityError::ExistingAccountRequiresBind => "existing_account_requires_bind",
            IdentityError::CannotUnlinkLastLogin => "cannot_unlink_last_login",
            IdentityError::IdentityNotFound => "identity_not_found",
            IdentityError::Store(_) | IdentityError::Database(_) => unreachable!(),
        };
        coded_error(status, code, error)
    }
}

fn google_error_response(error: GoogleIdentityError) -> Response {
    let status = match error {
        GoogleIdentityError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
        GoogleIdentityError::KeyServiceUnavailable => StatusCode::BAD_GATEWAY,
        _ => StatusCode::UNAUTHORIZED,
    };
    let code = match &error {
        GoogleIdentityError::NotConfigured => "google_oidc_not_configured",
        GoogleIdentityError::KeyServiceUnavailable => "google_jwks_unavailable",
        GoogleIdentityError::MalformedToken => "malformed_google_id_token",
        GoogleIdentityError::UnsupportedAlgorithm => "unsupported_google_token_algorithm",
        GoogleIdentityError::InvalidSignature => "invalid_google_token_signature",
        GoogleIdentityError::InvalidNonce => "invalid_google_nonce",
        GoogleIdentityError::InvalidClaims => "invalid_google_token_claims",
        GoogleIdentityError::UnverifiedEmail => "google_email_not_verified",
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

fn coded_error(status: StatusCode, code: &str, message: impl ToString) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message.to_string(),
            "code": code,
        })),
    )
        .into_response()
}
