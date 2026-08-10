use std::sync::Arc;

use axum::{
    extract::{Extension, OriginalUri, Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use sha2::{Digest, Sha256};

use crate::{
    node_compute_sharing::endpoint_authority::{
        bind_direct_tls_owner_api_transport, NodeEndpointOwnerCredentialMutationRequest,
    },
    project_auth::bearer_token,
    types::AppState,
};

use super::{
    contracts::{
        IssueCredentialRequest, RecoverCredentialRequest, RevokeCredentialRequest,
        RotateCredentialRequest,
    },
    rate_limit, response,
};
use crate::node_endpoint_transport::{
    direct_tls::DirectTlsPeerAddress, evidence_slot::VerifiedSecureTransportSlot,
};

const REQUEST_METHOD: &str = "POST";
const PRESENTED_ENDPOINT_SECRET_HEADER: &str = "x-elon-node-endpoint-secret";
pub(in crate::node_endpoint_transport) async fn issue(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(body): Json<IssueCredentialRequest>,
) -> Response {
    let (request, password) = match body.into_parts() {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, slot, peer, uri, headers, request, password, None).await
}

pub(in crate::node_endpoint_transport) async fn rotate(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    Path(agent_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RotateCredentialRequest>,
) -> Response {
    let presented_secret = match sensitive_header(&headers, PRESENTED_ENDPOINT_SECRET_HEADER) {
        Ok(Some(value)) => value,
        _ => {
            return response::error(
                StatusCode::BAD_REQUEST,
                "NODE_ENDPOINT_CREDENTIAL_POSSESSION_REQUIRED",
            )
        }
    };
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(
        state,
        slot,
        peer,
        uri,
        headers,
        request,
        password,
        Some(presented_secret),
    )
    .await
}

pub(in crate::node_endpoint_transport) async fn recover(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    Path(agent_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RecoverCredentialRequest>,
) -> Response {
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, slot, peer, uri, headers, request, password, None).await
}

pub(in crate::node_endpoint_transport) async fn revoke(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    Path(agent_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RevokeCredentialRequest>,
) -> Response {
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, slot, peer, uri, headers, request, password, None).await
}

async fn execute(
    state: Arc<AppState>,
    slot: VerifiedSecureTransportSlot,
    peer: DirectTlsPeerAddress,
    uri: axum::http::Uri,
    headers: HeaderMap,
    request: NodeEndpointOwnerCredentialMutationRequest,
    password: String,
    presented_endpoint_secret: Option<String>,
) -> Response {
    if uri.query().is_some() {
        return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_QUERY_FORBIDDEN");
    }
    if let Err(retry_after_seconds) = rate_limit::check_peer(peer) {
        return response::rate_limited(retry_after_seconds);
    }
    let exact_path = uri.path();
    let bearer = match bearer_token(&headers) {
        Some(value) if state.owner_token.as_deref() != Some(value) => value,
        _ => return response::error(StatusCode::UNAUTHORIZED, "NODE_ENDPOINT_BEARER_REQUIRED"),
    };
    let bearer_rate_key = format!("bearer:{}", hex::encode(Sha256::digest(bearer.as_bytes())));
    if let Err(retry_after_seconds) = rate_limit::check_bearer(&bearer_rate_key) {
        return response::rate_limited(retry_after_seconds);
    }
    let (_, mutation_digest) = match request.canonical_json_and_digest() {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let evidence = match slot.take() {
        Ok(value) => value,
        Err(_) => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SECURE_TRANSPORT_REQUIRED",
            )
        }
    };
    let (transport, response_permit) = match bind_direct_tls_owner_api_transport(
        evidence,
        REQUEST_METHOD,
        exact_path,
        &mutation_digest,
    ) {
        Ok(value) => value,
        Err(_) => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SECURE_TRANSPORT_INVALID",
            )
        }
    };
    let requested_secret = request.returns_secret();
    let commit = match state.store.mutate_node_endpoint_credential_as_owner(
        bearer,
        &password,
        presented_endpoint_secret.as_deref(),
        request,
        transport,
        response_permit,
    ) {
        Ok(value) => value,
        Err(_) => {
            return response::error(
                StatusCode::FORBIDDEN,
                "NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_DENIED",
            )
        }
    };
    let delivery = match commit.into_response_delivery(REQUEST_METHOD, exact_path, &mutation_digest)
    {
        Ok(value) => value,
        Err(_) => {
            return response::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "NODE_ENDPOINT_RESPONSE_BINDING_FAILED",
            )
        }
    };
    response::success(delivery, requested_secret)
}

fn sensitive_header(headers: &HeaderMap, name: &'static str) -> anyhow::Result<Option<String>> {
    headers
        .get(name)
        .map(|value| {
            let value = value.to_str()?.trim();
            if value.is_empty() || value.len() > 512 {
                anyhow::bail!("NODE_ENDPOINT_SENSITIVE_HEADER_INVALID");
            }
            Ok(value.to_string())
        })
        .transpose()
}
