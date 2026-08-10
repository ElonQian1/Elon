//! Dormant, sealed authority contracts for versioned node endpoint credentials and sessions.
//!
//! This module deliberately has no producer wired to the legacy HTTP/WebSocket path. A legacy
//! credential row, an in-memory session UUID, or a `ws://` connection cannot construct any sealed
//! input declared here.

mod canonical;
mod credential;
mod owner_reauthentication;
mod session;
mod types;

pub(crate) use credential::{
    AuthorizedFreshNodeEndpointCredentialIssuance, AuthorizedNodeEndpointCredentialRecovery,
    AuthorizedNodeEndpointCredentialRevocation, AuthorizedNodeEndpointCredentialRotation,
    NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
    PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
    PresentedNodeEndpointCredentialSecret,
};
pub(crate) use owner_reauthentication::{
    derive_owner_account_auth_state_digest, derive_owner_account_session_binding_digest,
    derive_owner_google_factor_binding_digest, derive_owner_password_factor_binding_digest,
    AuthorizedNodeEndpointOwnerReauthentication, NodeEndpointOwnerReauthenticationEnvelope,
    PreparedNodeEndpointOwnerReauthentication, VerifiedCurrentAccountSession,
    VerifiedRecentOwnerReauthentication, VerifiedSecureOwnerApiTransport,
};
pub(crate) use session::{
    canonical_direct_tls_verifier_digest, canonical_node_endpoint_capability_set,
    seal_direct_tls_connection, NodeEndpointSecureTransportBinding,
    NodeEndpointSessionAuthenticationReceiptEnvelope, NodeEndpointSessionOpenRequest,
    PreparedNodeEndpointSessionAuthentication, VerifiedSecureNodeEndpointTransport,
};
pub(crate) use types::{
    NodeEndpointCredentialBinding, NodeEndpointOwnerAuthorizationBasis, NodeEndpointSessionBinding,
    NodeEndpointSessionHeadSnapshot,
};

pub(crate) fn derive_node_endpoint_installation_binding_digest(
    agent_id: &str,
    owner_user_id: &str,
    install_id: &str,
) -> anyhow::Result<String> {
    canonical::installation_binding_digest(agent_id, owner_user_id, install_id)
}

pub(crate) fn derive_node_endpoint_secret_verifier_digest(secret_hash: &[u8; 32]) -> String {
    canonical::secret_verifier_digest(secret_hash)
}
