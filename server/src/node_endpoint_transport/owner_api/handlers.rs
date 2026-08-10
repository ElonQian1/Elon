use std::sync::Arc;

use axum::{
    extract::{Extension, FromRequest, OriginalUri, Request, State},
    http::StatusCode,
    response::Response,
    Json,
};

use crate::{
    node_compute_sharing::endpoint_authority::{
        bind_direct_tls_owner_api_transport, NodeEndpointOwnerCredentialMutationRequest,
    },
    types::AppState,
};

use super::{
    contracts::{
        IssueCredentialRequest, RecoverCredentialRequest, RevokeCredentialRequest,
        RotateCredentialRequest,
    },
    ingress::{prepare as prepare_ingress, OwnerApiIngress, OwnerApiRoute},
    response,
};
use crate::node_endpoint_transport::{
    direct_tls::DirectTlsPeerAddress, evidence_slot::VerifiedSecureTransportSlot,
};

const REQUEST_METHOD: &str = "POST";

pub(in crate::node_endpoint_transport) async fn issue(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    let ingress = match prepare_ingress(
        &state,
        slot,
        peer,
        &uri,
        http_request.headers(),
        OwnerApiRoute::Issue,
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match Json::<IssueCredentialRequest>::from_request(http_request, &state).await {
        Ok(Json(value)) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let (request, password) = match body.into_parts() {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, ingress, request, password).await
}

pub(in crate::node_endpoint_transport) async fn rotate(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    let ingress = match prepare_ingress(
        &state,
        slot,
        peer,
        &uri,
        http_request.headers(),
        OwnerApiRoute::Rotate,
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let agent_id = match ingress.agent_id() {
        Some(value) => value.to_string(),
        None => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SECURE_TRANSPORT_INVALID",
            )
        }
    };
    let body = match Json::<RotateCredentialRequest>::from_request(http_request, &state).await {
        Ok(Json(value)) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, ingress, request, password).await
}

pub(in crate::node_endpoint_transport) async fn recover(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    let ingress = match prepare_ingress(
        &state,
        slot,
        peer,
        &uri,
        http_request.headers(),
        OwnerApiRoute::Recover,
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let agent_id = match ingress.agent_id() {
        Some(value) => value.to_string(),
        None => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SECURE_TRANSPORT_INVALID",
            )
        }
    };
    let body = match Json::<RecoverCredentialRequest>::from_request(http_request, &state).await {
        Ok(Json(value)) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, ingress, request, password).await
}

pub(in crate::node_endpoint_transport) async fn revoke(
    State(state): State<Arc<AppState>>,
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
    Extension(peer): Extension<DirectTlsPeerAddress>,
    OriginalUri(uri): OriginalUri,
    http_request: Request,
) -> Response {
    let ingress = match prepare_ingress(
        &state,
        slot,
        peer,
        &uri,
        http_request.headers(),
        OwnerApiRoute::Revoke,
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let agent_id = match ingress.agent_id() {
        Some(value) => value.to_string(),
        None => {
            return response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_SECURE_TRANSPORT_INVALID",
            )
        }
    };
    let body = match Json::<RevokeCredentialRequest>::from_request(http_request, &state).await {
        Ok(Json(value)) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let (request, password) = match body.into_parts(agent_id) {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    execute(state, ingress, request, password).await
}

async fn execute(
    state: Arc<AppState>,
    ingress: OwnerApiIngress,
    request: NodeEndpointOwnerCredentialMutationRequest,
    password: String,
) -> Response {
    let (_, mutation_digest) = match request.canonical_json_and_digest() {
        Ok(value) => value,
        Err(_) => return response::error(StatusCode::BAD_REQUEST, "NODE_ENDPOINT_REQUEST_INVALID"),
    };
    let (evidence, exact_path, bearer, presented_endpoint_secret) = ingress.into_request_parts();
    let (transport, response_permit) = match bind_direct_tls_owner_api_transport(
        evidence,
        REQUEST_METHOD,
        &exact_path,
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
    let commit = match state
        .agent_manager
        .run_endpoint_root_mutation_and_close_process_session(
            &state,
            request,
            |request| {
                state
                    .store
                    .preflight_node_endpoint_owner_credential_mutation(&bearer, &password, request)
            },
            |request| {
                state.store.mutate_node_endpoint_credential_as_owner(
                    &bearer,
                    &password,
                    presented_endpoint_secret.as_deref(),
                    request,
                    transport,
                    response_permit,
                )
            },
        )
        .await
    {
        Ok(value) => value,
        Err(_) => {
            return response::error(
                StatusCode::FORBIDDEN,
                "NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_DENIED",
            )
        }
    };
    let delivery =
        match commit.into_response_delivery(REQUEST_METHOD, &exact_path, &mutation_digest) {
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
