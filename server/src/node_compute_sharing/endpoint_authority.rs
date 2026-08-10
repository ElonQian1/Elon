//! Dormant, sealed authority contracts for versioned node endpoint credentials and sessions.
//!
//! This module deliberately has no producer wired to the legacy HTTP/WebSocket path. A legacy
//! credential row, an in-memory session UUID, or a `ws://` connection cannot construct any sealed
//! input declared here.

mod canonical;
mod credential;
mod owner_credential_mutation;
mod owner_reauthentication;
mod owner_reauthentication_consumption;
mod session;
mod types;

pub(crate) use credential::{
    authorize_owner_credential_mutation, AuthorizedFreshNodeEndpointCredentialIssuance,
    AuthorizedNodeEndpointCredentialMutation, AuthorizedNodeEndpointCredentialRecovery,
    AuthorizedNodeEndpointCredentialRevocation, AuthorizedNodeEndpointCredentialRotation,
    NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
    PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
    PresentedNodeEndpointCredentialSecret,
};
pub(crate) use owner_credential_mutation::{
    ExpectedNodeEndpointCredential, NodeEndpointOwnerCredentialMutationRequest,
};
pub(crate) use owner_reauthentication::{
    authorize_password_owner_reauthentication, bind_direct_tls_owner_api_transport,
    derive_owner_account_auth_state_digest, derive_owner_account_session_binding_digest,
    derive_owner_google_factor_binding_digest, derive_owner_password_factor_binding_digest,
    AuthorizedNodeEndpointOwnerReauthentication, NodeEndpointOwnerReauthenticationEnvelope,
    OwnerApiResponsePermit, PreparedNodeEndpointOwnerReauthentication,
    VerifiedCurrentAccountSession, VerifiedRecentOwnerReauthentication,
    VerifiedSecureOwnerApiTransport,
};
pub(crate) use owner_reauthentication_consumption::{
    prepare_owner_reauthentication_consumption, NodeEndpointCredentialMutationResultBinding,
    NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    PreparedNodeEndpointOwnerReauthenticationConsumption,
};
pub(crate) use session::{
    bind_direct_tls_node_endpoint_transport, canonical_direct_tls_verifier_digest,
    canonical_node_endpoint_capability_set, seal_direct_tls_connection,
    NodeEndpointSecureTransportBinding, NodeEndpointSessionAuthenticationReceiptEnvelope,
    NodeEndpointSessionOpenRequest, NodeEndpointSessionProfile,
    PreparedNodeEndpointSessionAuthentication, VerifiedDirectTlsConnectionEvidence,
    VerifiedSecureNodeEndpointTransport,
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
