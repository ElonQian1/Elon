use axum::{
    http::{HeaderMap, StatusCode},
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::{
    node_compute_sharing::endpoint_authority::VerifiedDirectTlsConnectionEvidence,
    node_endpoint_transport::{
        direct_tls::DirectTlsPeerAddress, evidence_slot::VerifiedSecureTransportSlot,
    },
    project_auth::bearer_token,
    types::AppState,
};

use super::{rate_limit, response};

const PRESENTED_ENDPOINT_SECRET_HEADER: &str = "x-elon-node-endpoint-secret";
const ISSUE_PATH: &str = "/api/me/node-endpoint-credentials/issue";
const CREDENTIAL_PATH_PREFIX: &str = "/api/me/node-endpoint-credentials/";

#[derive(Clone, Copy)]
pub(super) enum OwnerApiRoute {
    Issue,
    Rotate,
    Recover,
    Revoke,
}

pub(super) struct OwnerApiIngress {
    evidence: VerifiedDirectTlsConnectionEvidence,
    exact_path: String,
    agent_id: Option<String>,
    bearer: String,
    presented_endpoint_secret: Option<String>,
}

impl OwnerApiIngress {
    pub(super) fn agent_id(&self) -> Option<&str> {
        self.agent_id.as_deref()
    }

    pub(super) fn into_request_parts(
        self,
    ) -> (
        VerifiedDirectTlsConnectionEvidence,
        String,
        String,
        Option<String>,
    ) {
        (
            self.evidence,
            self.exact_path,
            self.bearer,
            self.presented_endpoint_secret,
        )
    }
}

pub(super) fn prepare(
    state: &AppState,
    slot: VerifiedSecureTransportSlot,
    peer: DirectTlsPeerAddress,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    route: OwnerApiRoute,
) -> Result<OwnerApiIngress, Response> {
    if let Err(retry_after_seconds) = rate_limit::check_peer(peer) {
        return Err(response::rate_limited(retry_after_seconds));
    }
    if uri.query().is_some() {
        return Err(response::error(
            StatusCode::BAD_REQUEST,
            "NODE_ENDPOINT_QUERY_FORBIDDEN",
        ));
    }
    let agent_id = exact_route_agent_id(route, uri.path()).ok_or_else(|| {
        response::error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_SECURE_TRANSPORT_INVALID",
        )
    })?;
    let bearer = match bearer_token(headers) {
        Some(value) if state.owner_token.as_deref() != Some(value) => value.to_string(),
        _ => {
            return Err(response::error(
                StatusCode::UNAUTHORIZED,
                "NODE_ENDPOINT_BEARER_REQUIRED",
            ))
        }
    };
    let bearer_rate_key = format!("bearer:{}", hex::encode(Sha256::digest(bearer.as_bytes())));
    if let Err(retry_after_seconds) = rate_limit::check_bearer(&bearer_rate_key) {
        return Err(response::rate_limited(retry_after_seconds));
    }
    let presented_endpoint_secret = if matches!(route, OwnerApiRoute::Rotate) {
        match sensitive_header(headers, PRESENTED_ENDPOINT_SECRET_HEADER) {
            Ok(Some(value)) => Some(value),
            _ => {
                return Err(response::error(
                    StatusCode::BAD_REQUEST,
                    "NODE_ENDPOINT_CREDENTIAL_POSSESSION_REQUIRED",
                ))
            }
        }
    } else {
        None
    };
    let evidence = slot.take().map_err(|_| {
        response::error(
            StatusCode::UNAUTHORIZED,
            "NODE_ENDPOINT_SECURE_TRANSPORT_REQUIRED",
        )
    })?;
    Ok(OwnerApiIngress {
        evidence,
        exact_path: uri.path().to_string(),
        agent_id,
        bearer,
        presented_endpoint_secret,
    })
}

fn exact_route_agent_id(route: OwnerApiRoute, exact_path: &str) -> Option<Option<String>> {
    if matches!(route, OwnerApiRoute::Issue) {
        return (exact_path == ISSUE_PATH).then_some(None);
    }
    let suffix = match route {
        OwnerApiRoute::Rotate => "rotate",
        OwnerApiRoute::Recover => "recover",
        OwnerApiRoute::Revoke => "revoke",
        OwnerApiRoute::Issue => unreachable!(),
    };
    let remainder = exact_path.strip_prefix(CREDENTIAL_PATH_PREFIX)?;
    let agent_id = remainder.strip_suffix(&format!("/{suffix}"))?;
    if agent_id.is_empty()
        || agent_id.len() > 160
        || !agent_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    {
        return None;
    }
    Some(Some(agent_id.to_string()))
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
