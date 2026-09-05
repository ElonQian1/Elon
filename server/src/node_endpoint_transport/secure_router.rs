use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, Extension},
    http::StatusCode,
    routing::{get, post},
    Router,
};

use crate::types::AppState;

use super::{
    direct_tls::DirectTlsPeerAddress, endpoint_session, evidence_slot::VerifiedSecureTransportSlot,
    owner_api,
};

pub(super) fn build(
    slot: VerifiedSecureTransportSlot,
    state: Arc<AppState>,
    owner_credential_api_enabled: bool,
    owner_bootstrap_api_enabled: bool,
    endpoint_session_api_enabled: bool,
    peer_address: DirectTlsPeerAddress,
) -> Router {
    let router = if endpoint_session_api_enabled {
        Router::new().route("/agent/ws", get(endpoint_session::session_ws))
    } else {
        Router::new().route("/agent/ws", get(reject_disabled_endpoint_session))
    };
    let router = if owner_credential_api_enabled {
        router
            .route(
                "/api/me/node-endpoint-credentials/issue",
                post(owner_api::issue),
            )
            .route(
                "/api/me/node-endpoint-credentials/:agent_id/rotate",
                post(owner_api::rotate),
            )
            .route(
                "/api/me/node-endpoint-credentials/:agent_id/recover",
                post(owner_api::recover),
            )
            .route(
                "/api/me/node-endpoint-credentials/:agent_id/revoke",
                post(owner_api::revoke),
            )
    } else {
        router
    };
    let router = if owner_credential_api_enabled && owner_bootstrap_api_enabled {
        router
            .route("/api/auth/login", post(owner_api::bootstrap_login))
            .route(
                "/api/me/nodes/register",
                post(owner_api::bootstrap_register_node),
            )
    } else {
        router
    };
    router
        .merge(super::asset_access::routes(&state.public_url))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(Extension(peer_address))
        .layer(Extension(slot))
        .with_state(state)
}

async fn reject_disabled_endpoint_session(
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
) -> (StatusCode, &'static str) {
    // Consume and immediately drop the single-use proof. A configured TLS listener must not make
    // endpoint authentication reachable until its independent session gate is explicitly enabled.
    let _ = slot.take();
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "NODE_ENDPOINT_SESSION_DISABLED",
    )
}
