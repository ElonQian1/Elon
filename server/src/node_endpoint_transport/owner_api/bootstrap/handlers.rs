use std::sync::Arc;

use axum::{
    extract::{Extension, FromRequest, OriginalUri, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    node_endpoint_transport::{
        direct_tls::DirectTlsPeerAddress, evidence_slot::VerifiedSecureTransportSlot,
    },
    project_auth::login_inner,
    store::node_credentials::{LegacyNodeRegistrationOutcome, LegacyNodeRegistrationRequest},
    types::AppState,
};

use super::contracts::{
    BootstrapLoginRequest, BootstrapNodeRegistrationRequest, BootstrapNodeRegistrationResponse,
};
use crate::node_endpoint_transport::owner_api::rate_limit;

const LOGIN_PATH: &str = "/api/auth/login";
const REGISTER_PATH: &str = "/api/me/nodes/register";
const MAX_BEARER_BYTES: usize = 2048;
const BEARER_NOT_CURRENT: &str = "NODE_ENDPOINT_BOOTSTRAP_BEARER_NOT_CURRENT";

pub(in crate::node_endpoint_transport) async fn login(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    if let Err(retry_after_seconds) = rate_limit::check_peer(peer) {
        return rate_limited(retry_after_seconds);
    }
    let _transport = match take_transport(slot, &uri, LOGIN_PATH) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if rejects_ambient_headers(http_request.headers(), false) {
        return error(
            StatusCode::BAD_REQUEST,
            "NODE_ENDPOINT_OWNER_BOOTSTRAP_HEADERS_FORBIDDEN",
        );
    }
    let request = match Json::<BootstrapLoginRequest>::from_request(http_request, &state).await {
        Ok(Json(value)) => match value.into_login_request() {
            Ok(request) => request,
            Err(_) => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "NODE_ENDPOINT_BOOTSTRAP_LOGIN_REQUEST_INVALID",
                )
            }
        },
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "NODE_ENDPOINT_BOOTSTRAP_LOGIN_REQUEST_INVALID",
            )
        }
    };
    match login_inner(&state, request) {
        Ok((token, expires_at, user)) => sensitive_json(
            StatusCode::OK,
            serde_json::json!({
                "token": token,
                "expires_at": expires_at,
                "user": user,
            }),
        ),
        Err(_) => error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_BOOTSTRAP_LOGIN_DENIED",
        ),
    }
}

pub(in crate::node_endpoint_transport) async fn register_node(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    if let Err(retry_after_seconds) = rate_limit::check_peer(peer) {
        return rate_limited(retry_after_seconds);
    }
    let _transport = match take_transport(slot, &uri, REGISTER_PATH) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if rejects_ambient_headers(http_request.headers(), true) {
        return error(
            StatusCode::BAD_REQUEST,
            "NODE_ENDPOINT_OWNER_BOOTSTRAP_HEADERS_FORBIDDEN",
        );
    }
    let bearer = match strict_bearer(http_request.headers()) {
        Some(value) => value.to_string(),
        _ => {
            return error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_BOOTSTRAP_BEARER_REQUIRED",
            )
        }
    };
    let bearer_rate_key = format!("bearer:{}", hex::encode(Sha256::digest(bearer.as_bytes())));
    if let Err(retry_after_seconds) = rate_limit::check_bearer(&bearer_rate_key) {
        return rate_limited(retry_after_seconds);
    }
    if state.owner_token.as_deref() == Some(bearer.as_str()) {
        return error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_BOOTSTRAP_BEARER_REQUIRED",
        );
    }
    let user = match state.store.authenticate_token(&bearer) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_BOOTSTRAP_BEARER_DENIED",
            )
        }
    };
    let request =
        match Json::<BootstrapNodeRegistrationRequest>::from_request(http_request, &state).await {
            Ok(Json(value)) => match value.into_register_node_request() {
                Ok(request) => request,
                Err(_) => {
                    return error(
                        StatusCode::BAD_REQUEST,
                        "NODE_ENDPOINT_BOOTSTRAP_REGISTRATION_REQUEST_INVALID",
                    )
                }
            },
            Err(_) => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "NODE_ENDPOINT_BOOTSTRAP_REGISTRATION_REQUEST_INVALID",
                )
            }
        };

    let new_secret = uuid::Uuid::new_v4().to_string().replace('-', "")
        + &uuid::Uuid::new_v4().to_string().replace('-', "");
    let new_secret_hash = hex::encode(Sha256::digest(new_secret.as_bytes()));
    let existing_agent_id = normalized(request.existing_agent_id.as_deref());
    let existing_secret_hash = normalized(request.existing_secret.as_deref())
        .map(|secret| hex::encode(Sha256::digest(secret.as_bytes())));
    let install_id = normalized(request.install_id.as_deref());
    let device_name = normalized(request.device_name.as_deref());
    let label = normalized(request.label.as_deref());
    let proposed_agent_id = format!(
        "node-{}-{}",
        user.id.chars().take(6).collect::<String>(),
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );
    let registration = LegacyNodeRegistrationRequest::new(
        &user.id,
        &proposed_agent_id,
        &new_secret_hash,
        existing_agent_id,
        existing_secret_hash.as_deref(),
        install_id,
        label,
        device_name,
    )
    .with_current_bearer_token(&bearer);
    let outcome = state
        .agent_manager
        .run_legacy_registration_and_close_process_session(&state, || {
            state
                .store
                .register_or_renew_legacy_node_credential(registration)
        })
        .await;
    match outcome {
        Ok(LegacyNodeRegistrationOutcome::Renewed { agent_id })
        | Ok(LegacyNodeRegistrationOutcome::Created { agent_id }) => sensitive_json(
            StatusCode::OK,
            BootstrapNodeRegistrationResponse::new(agent_id, user.id),
        ),
        Ok(LegacyNodeRegistrationOutcome::EndpointAuthorityRequired { endpoint_authority }) => {
            sensitive_json(
                StatusCode::CONFLICT,
                serde_json::json!({
                    "error": "NODE_ENDPOINT_AUTHORITY_REQUIRED",
                    "endpoint_authority": endpoint_authority,
                }),
            )
        }
        Err(store_error) if store_error.to_string() == BEARER_NOT_CURRENT => error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_BOOTSTRAP_BEARER_DENIED",
        ),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "NODE_ENDPOINT_BOOTSTRAP_REGISTRATION_FAILED",
        ),
    }
}

struct BootstrapTransportPermit {
    _evidence: crate::node_compute_sharing::endpoint_authority::VerifiedDirectTlsConnectionEvidence,
}

fn take_transport(
    slot: VerifiedSecureTransportSlot,
    uri: &axum::http::Uri,
    expected_path: &str,
) -> Result<BootstrapTransportPermit, Response> {
    let evidence = slot.take().map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_SECURE_TRANSPORT_REQUIRED",
        )
    })?;
    if uri.query().is_some() || uri.path() != expected_path {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "NODE_ENDPOINT_BOOTSTRAP_QUERY_OR_PATH_FORBIDDEN",
        ));
    }
    Ok(BootstrapTransportPermit {
        _evidence: evidence,
    })
}

fn rejects_ambient_headers(headers: &HeaderMap, bearer_allowed: bool) -> bool {
    headers.contains_key(header::COOKIE)
        || (!bearer_allowed && headers.contains_key(header::AUTHORIZATION))
        || headers.keys().any(|name| {
            let name = name.as_str();
            name == "forwarded"
                || name.starts_with("x-forwarded-")
                || matches!(name, "proxy-authorization" | "x-real-ip")
        })
}

fn strict_bearer(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next()?.to_str().ok()?.trim();
    if values.next().is_some() {
        return None;
    }
    let token = value.strip_prefix("Bearer ")?.trim();
    (!token.is_empty() && token.len() <= MAX_BEARER_BYTES).then_some(token)
}

fn normalized(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn rate_limited(retry_after_seconds: u64) -> Response {
    let mut headers = sensitive_headers();
    if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
        headers.insert(header::RETRY_AFTER, value);
    }
    (
        StatusCode::TOO_MANY_REQUESTS,
        headers,
        Json(serde_json::json!({
            "error": "NODE_ENDPOINT_OWNER_BOOTSTRAP_RATE_LIMITED"
        })),
    )
        .into_response()
}

fn error(status: StatusCode, code: &'static str) -> Response {
    sensitive_json(status, serde_json::json!({ "error": code }))
}

fn sensitive_json(status: StatusCode, value: impl Serialize) -> Response {
    (status, sensitive_headers(), Json(value)).into_response()
}

fn sensitive_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers
}
