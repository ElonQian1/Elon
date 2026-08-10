use anyhow::{bail, Result};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    PreparedNodeEndpointOwnerReauthenticationConsumption,
};

mod read;
mod write;

pub(in crate::store::node_credentials::endpoint_authority) use read::{
    by_consumption_id_on, by_owner_mutation_request_on,
};
pub(in crate::store::node_credentials::endpoint_authority) use write::insert_on;

/// Canonically validated Store readback. The type is intentionally linear: callers may inspect or
/// consume it, but cannot clone a replay result into a second authority path.
pub(in crate::store::node_credentials::endpoint_authority) struct StoredOwnerReauthenticationConsumption
{
    envelope: NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    consumption_json: String,
    consumption_digest: String,
    canonicalization: String,
    digest_algorithm: String,
}

impl StoredOwnerReauthenticationConsumption {
    pub(in crate::store::node_credentials::endpoint_authority) fn envelope(
        &self,
    ) -> &NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        &self.envelope
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn consumption_json(&self) -> &str {
        &self.consumption_json
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn consumption_digest(
        &self,
    ) -> &str {
        &self.consumption_digest
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn consumed_at(&self) -> &str {
        self.envelope.consumed_at()
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn recorded_at(&self) -> &str {
        self.envelope.recorded_at()
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn ensure_exact(
        &self,
        prepared: &PreparedNodeEndpointOwnerReauthenticationConsumption,
    ) -> Result<()> {
        if &self.envelope != prepared.envelope()
            || self.consumption_json != prepared.consumption_json()
            || self.consumption_digest != prepared.consumption_digest()
            || self.canonicalization != prepared.canonicalization()
            || self.digest_algorithm != prepared.digest_algorithm()
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_REPLAY_MISMATCH");
        }
        Ok(())
    }

    pub(in crate::store::node_credentials::endpoint_authority) fn into_envelope(
        self,
    ) -> NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        self.envelope
    }
}
