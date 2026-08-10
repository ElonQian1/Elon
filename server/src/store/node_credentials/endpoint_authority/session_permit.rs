use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::NodeEndpointSessionBinding;

use super::VerifiedCurrentNodeEndpointSession;

/// Store-issued projection of one exact durable endpoint session.
///
/// This value is socket currentness only. It is deliberately not serializable and never grants
/// planning, work-admission, Ready, routing, dispatch, acknowledgement, or compute authority.
#[derive(Clone)]
pub(crate) struct NodeEndpointSessionPermit {
    binding: NodeEndpointSessionBinding,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    capability_count: u64,
    capability_set_digest: String,
    authenticated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    replayed: bool,
}

impl NodeEndpointSessionPermit {
    pub(crate) fn binding(&self) -> &NodeEndpointSessionBinding {
        &self.binding
    }

    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }

    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }

    pub(crate) fn installation_binding_digest(&self) -> &str {
        &self.installation_binding_digest
    }

    pub(crate) fn capability_count(&self) -> u64 {
        self.capability_count
    }

    pub(crate) fn capability_set_digest(&self) -> &str {
        &self.capability_set_digest
    }

    pub(crate) fn authenticated_at(&self) -> DateTime<Utc> {
        self.authenticated_at
    }

    pub(crate) fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub(crate) fn replayed(&self) -> bool {
        self.replayed
    }

    pub(super) fn from_verified(current: &VerifiedCurrentNodeEndpointSession) -> Result<Self> {
        let receipt = current.receipt();
        if receipt.capability_count() != 0 {
            bail!("NODE_ENDPOINT_SESSION_COMPUTE_INERT_CAPABILITY_MISMATCH");
        }
        Ok(Self {
            binding: current.head().binding().clone(),
            owner_user_id: receipt.owner_user_id().to_string(),
            install_id: receipt.install_id().to_string(),
            installation_binding_digest: receipt.installation_binding_digest().to_string(),
            capability_count: receipt.capability_count(),
            capability_set_digest: receipt.capability_set_digest().to_string(),
            authenticated_at: parse_timestamp(receipt.authenticated_at())?,
            expires_at: parse_timestamp(receipt.expires_at())?,
            replayed: current.replayed(),
        })
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
