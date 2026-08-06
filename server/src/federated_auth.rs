//! Public first-party federated login contract shared by Win, Android and Web.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    federated_auth_google::{verify_google_id_token, GoogleIdentityError, GoogleOidcConfig},
    project_auth::{auth_from_headers, json_error},
    store::IdentityError,
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct ChallengeRequest {
    mode: String,
    platform: String,
}

#[derive(Debug, Deserialize)]
struct CompleteRequest {
    challenge_id: String,
    id_token: String,
    device_name: Option<String>,
    apk_version: Option<String>,
    #[serde(default)]
    remember_device: bool,
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
    let user_id = if request.mode == "bind" {
        match auth_from_headers(&state, &headers) {
            Ok(user) if user.id != "local-owner" => Some(user.id),
            Ok(_) => return json_error(StatusCode::BAD_REQUEST, "本地 owner 不能绑定云端身份"),
            Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
        }
    } else {
        None
    };
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
    let challenge = match state.store.identity_challenge(&request.challenge_id) {
        Ok(challenge) if challenge.provider == "google" => challenge,
        Ok(_) => return json_error(StatusCode::BAD_REQUEST, "登录挑战与 Provider 不匹配"),
        Err(error) => return identity_error_response(error),
    };
    if challenge.mode == "bind" {
        let current_user = match auth_from_headers(&state, &headers) {
            Ok(user) => user,
            Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
        };
        if challenge.user_id.as_deref() != Some(current_user.id.as_str()) {
            return json_error(StatusCode::FORBIDDEN, "该绑定挑战不属于当前账号");
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
    Json(serde_json::json!({
        "mode": challenge.mode,
        "user": completion.user,
        "identity": completion.identity,
        "created_user": completion.created_user,
        "session": session,
    }))
    .into_response()
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
        json_error(status, "身份服务暂时不可用")
    } else {
        json_error(status, error)
    }
}

fn google_error_response(error: GoogleIdentityError) -> Response {
    let status = match error {
        GoogleIdentityError::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
        GoogleIdentityError::KeyServiceUnavailable => StatusCode::BAD_GATEWAY,
        _ => StatusCode::UNAUTHORIZED,
    };
    json_error(status, error)
}
