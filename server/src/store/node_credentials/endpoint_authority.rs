//! Dormant Store kernels for durable node endpoint credentials and authenticated sessions.
//!
//! No HTTP, WebSocket, protobuf, legacy credential, or in-memory connection path calls this
//! module. Returned session values are durable facts only; they are not socket, routing,
//! command, lease, or compute authority.

use anyhow::Result;

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedFreshNodeEndpointCredentialIssuance, AuthorizedNodeEndpointCredentialRecovery,
    AuthorizedNodeEndpointCredentialRevocation, AuthorizedNodeEndpointCredentialRotation,
    AuthorizedNodeEndpointOwnerReauthentication, NodeEndpointCredentialBinding,
    NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
    NodeEndpointOwnerReauthenticationEnvelope, NodeEndpointSessionAuthenticationReceiptEnvelope,
    NodeEndpointSessionHeadSnapshot, NodeEndpointSessionOpenRequest,
    PresentedNodeEndpointCredentialSecret, VerifiedSecureNodeEndpointTransport,
};

use super::super::Store;

mod credentials;
mod legacy_currentness;
mod owner_credential_mutation;
mod owner_reauthentication;
mod secret;
mod sessions;

pub(super) use legacy_currentness::{
    current_node_endpoint_root_by_agent_on, current_node_endpoint_root_by_owner_install_on,
};
pub(crate) use owner_credential_mutation::{
    CurrentOwnerAccountSource, NodeEndpointOwnerCredentialMutationCommit,
    NodeEndpointOwnerCredentialMutationDelivery,
};

pub(in crate::store) struct NodeEndpointOwnerReauthenticationReceipt {
    envelope: NodeEndpointOwnerReauthenticationEnvelope,
    receipt_digest: String,
    replayed: bool,
}

impl NodeEndpointOwnerReauthenticationReceipt {
    pub(in crate::store) fn envelope(&self) -> &NodeEndpointOwnerReauthenticationEnvelope {
        &self.envelope
    }

    pub(in crate::store) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }

    pub(in crate::store) fn replayed(&self) -> bool {
        self.replayed
    }
}

pub(in crate::store) struct NodeEndpointCredentialMutationReceipt {
    current: NodeEndpointCredentialBinding,
    issued_version: Option<NodeEndpointCredentialVersionEnvelope>,
    revoked_version: Option<NodeEndpointCredentialRevocationEnvelope>,
    replayed: bool,
}

impl NodeEndpointCredentialMutationReceipt {
    pub(in crate::store) fn current(&self) -> &NodeEndpointCredentialBinding {
        &self.current
    }

    pub(in crate::store) fn issued_version(
        &self,
    ) -> Option<&NodeEndpointCredentialVersionEnvelope> {
        self.issued_version.as_ref()
    }

    pub(in crate::store) fn revoked_version(
        &self,
    ) -> Option<&NodeEndpointCredentialRevocationEnvelope> {
        self.revoked_version.as_ref()
    }

    pub(in crate::store) fn replayed(&self) -> bool {
        self.replayed
    }
}

/// Transactionally verified durable currentness. Consumers must still use a Store CAS kernel;
/// this type never authorizes an online socket or a compute action.
pub(in crate::store) struct VerifiedCurrentNodeEndpointSession {
    receipt: NodeEndpointSessionAuthenticationReceiptEnvelope,
    head: NodeEndpointSessionHeadSnapshot,
    replayed: bool,
}

impl VerifiedCurrentNodeEndpointSession {
    pub(in crate::store) fn receipt(&self) -> &NodeEndpointSessionAuthenticationReceiptEnvelope {
        &self.receipt
    }

    pub(in crate::store) fn head(&self) -> &NodeEndpointSessionHeadSnapshot {
        &self.head
    }

    pub(in crate::store) fn replayed(&self) -> bool {
        self.replayed
    }
}

impl Store {
    pub(in crate::store) fn record_node_endpoint_owner_reauthentication(
        &self,
        authorized: &AuthorizedNodeEndpointOwnerReauthentication,
    ) -> Result<NodeEndpointOwnerReauthenticationReceipt> {
        owner_reauthentication::record(self, authorized)
    }

    pub(in crate::store) fn issue_fresh_node_endpoint_credential(
        &self,
        authorized: &AuthorizedFreshNodeEndpointCredentialIssuance,
    ) -> Result<NodeEndpointCredentialMutationReceipt> {
        credentials::issue_fresh(self, authorized)
    }

    pub(in crate::store) fn rotate_node_endpoint_credential(
        &self,
        authorized: &AuthorizedNodeEndpointCredentialRotation,
        presented: &PresentedNodeEndpointCredentialSecret,
    ) -> Result<NodeEndpointCredentialMutationReceipt> {
        credentials::rotate(self, authorized, presented)
    }

    pub(in crate::store) fn recover_node_endpoint_credential(
        &self,
        authorized: &AuthorizedNodeEndpointCredentialRecovery,
    ) -> Result<NodeEndpointCredentialMutationReceipt> {
        credentials::recover(self, authorized)
    }

    pub(in crate::store) fn revoke_node_endpoint_credential(
        &self,
        authorized: &AuthorizedNodeEndpointCredentialRevocation,
    ) -> Result<NodeEndpointCredentialMutationReceipt> {
        credentials::revoke(self, authorized)
    }

    pub(in crate::store) fn authenticate_node_endpoint_session(
        &self,
        request: &NodeEndpointSessionOpenRequest,
        transport: &VerifiedSecureNodeEndpointTransport,
    ) -> Result<VerifiedCurrentNodeEndpointSession> {
        sessions::authenticate(self, request, transport)
    }

    pub(in crate::store) fn close_node_endpoint_session(
        &self,
        current: &VerifiedCurrentNodeEndpointSession,
    ) -> Result<NodeEndpointSessionHeadSnapshot> {
        sessions::close(self, current)
    }

    pub(in crate::store) fn inspect_node_endpoint_session_currentness(
        &self,
        binding: &crate::node_compute_sharing::endpoint_authority::NodeEndpointSessionBinding,
    ) -> Result<VerifiedCurrentNodeEndpointSession> {
        sessions::inspect_currentness(self, binding)
    }

    pub(in crate::store) fn restart_node_endpoint_sessions(
        &self,
    ) -> Result<Vec<NodeEndpointSessionHeadSnapshot>> {
        sessions::restart(self)
    }

    pub(in crate::store) fn recover_node_endpoint_session_heads(
        &self,
    ) -> Result<Vec<NodeEndpointSessionHeadSnapshot>> {
        sessions::recover_heads(self)
    }
}

pub(super) fn owner_reauthentication_receipt(
    envelope: NodeEndpointOwnerReauthenticationEnvelope,
    receipt_digest: String,
    replayed: bool,
) -> NodeEndpointOwnerReauthenticationReceipt {
    NodeEndpointOwnerReauthenticationReceipt {
        envelope,
        receipt_digest,
        replayed,
    }
}

pub(super) fn credential_receipt(
    current: NodeEndpointCredentialBinding,
    issued_version: Option<NodeEndpointCredentialVersionEnvelope>,
    revoked_version: Option<NodeEndpointCredentialRevocationEnvelope>,
    replayed: bool,
) -> NodeEndpointCredentialMutationReceipt {
    NodeEndpointCredentialMutationReceipt {
        current,
        issued_version,
        revoked_version,
        replayed,
    }
}

pub(super) fn verified_session(
    receipt: NodeEndpointSessionAuthenticationReceiptEnvelope,
    head: NodeEndpointSessionHeadSnapshot,
    replayed: bool,
) -> VerifiedCurrentNodeEndpointSession {
    VerifiedCurrentNodeEndpointSession {
        receipt,
        head,
        replayed,
    }
}
