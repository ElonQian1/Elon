//! One-transaction owner credential mutation producer.
//!
//! The public facade is added only after v217 receipt recording, v218 consumption, and the
//! credential CAS kernels are composed below this module. These source leaves are deliberately
//! unavailable to the legacy HTTP and WebSocket paths.

mod authorization;
mod current_account;
mod current_target;
mod execute;
mod replay;
mod secret;
mod transaction;

pub(crate) use current_account::CurrentOwnerAccountSource;

use anyhow::Result;

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointCredentialBinding, NodeEndpointOwnerCredentialMutationRequest,
    NodeEndpointOwnerReauthenticationConsumptionEnvelope, OwnerApiResponsePermit,
    VerifiedSecureOwnerApiTransport,
};

use super::super::Store;

pub(crate) struct NodeEndpointOwnerCredentialMutationCommit {
    committed: NodeEndpointCredentialBinding,
    consumption: NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    consumption_digest: String,
    replayed: bool,
    result_is_current: bool,
    secret: Option<secret::GeneratedEndpointSecret>,
    response_permit: OwnerApiResponsePermit,
}

pub(crate) struct NodeEndpointOwnerCredentialMutationDelivery {
    committed: NodeEndpointCredentialBinding,
    consumption_id: String,
    consumption_digest: String,
    replayed: bool,
    result_is_current: bool,
    secret: Option<String>,
}

impl NodeEndpointOwnerCredentialMutationCommit {
    pub(crate) fn committed(&self) -> &NodeEndpointCredentialBinding {
        &self.committed
    }

    pub(crate) fn consumption(&self) -> &NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        &self.consumption
    }

    pub(crate) fn consumption_digest(&self) -> &str {
        &self.consumption_digest
    }

    pub(crate) fn replayed(&self) -> bool {
        self.replayed
    }

    pub(crate) fn result_is_current(&self) -> bool {
        self.result_is_current
    }

    pub(crate) fn secret_visible_once(&self) -> bool {
        self.secret.is_some()
    }

    pub(crate) fn into_response_delivery(
        self,
        request_method: &str,
        exact_path: &str,
        canonical_mutation_digest: &str,
    ) -> Result<NodeEndpointOwnerCredentialMutationDelivery> {
        self.response_permit.consume_for_response(
            request_method,
            exact_path,
            canonical_mutation_digest,
        )?;
        Ok(NodeEndpointOwnerCredentialMutationDelivery {
            committed: self.committed,
            consumption_id: self.consumption.consumption_id().to_string(),
            consumption_digest: self.consumption_digest,
            replayed: self.replayed,
            result_is_current: self.result_is_current,
            secret: self
                .secret
                .map(secret::GeneratedEndpointSecret::into_plaintext),
        })
    }
}

impl NodeEndpointOwnerCredentialMutationDelivery {
    pub(crate) fn committed(&self) -> &NodeEndpointCredentialBinding {
        &self.committed
    }

    pub(crate) fn consumption_id(&self) -> &str {
        &self.consumption_id
    }

    pub(crate) fn consumption_digest(&self) -> &str {
        &self.consumption_digest
    }

    pub(crate) fn replayed(&self) -> bool {
        self.replayed
    }

    pub(crate) fn result_is_current(&self) -> bool {
        self.result_is_current
    }

    pub(crate) fn secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }
}

impl Store {
    pub(crate) fn preflight_node_endpoint_owner_credential_mutation(
        &self,
        bearer_token: &str,
        current_password: &str,
        request: &NodeEndpointOwnerCredentialMutationRequest,
    ) -> Result<String> {
        transaction::preflight_owned_target(self, bearer_token, current_password, request)
    }

    pub(crate) fn mutate_node_endpoint_credential_as_owner(
        &self,
        bearer_token: &str,
        current_password: &str,
        presented_endpoint_secret: Option<&str>,
        request: NodeEndpointOwnerCredentialMutationRequest,
        transport: VerifiedSecureOwnerApiTransport,
        response_permit: OwnerApiResponsePermit,
    ) -> Result<NodeEndpointOwnerCredentialMutationCommit> {
        transaction::mutate(
            self,
            bearer_token,
            current_password,
            presented_endpoint_secret,
            request,
            transport,
            response_permit,
        )
    }
}

pub(super) fn commit_result(
    committed: NodeEndpointCredentialBinding,
    consumption: NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    consumption_digest: String,
    replayed: bool,
    result_is_current: bool,
    secret: Option<secret::GeneratedEndpointSecret>,
    response_permit: OwnerApiResponsePermit,
) -> NodeEndpointOwnerCredentialMutationCommit {
    NodeEndpointOwnerCredentialMutationCommit {
        committed,
        consumption,
        consumption_digest,
        replayed,
        result_is_current,
        secret,
        response_permit,
    }
}
