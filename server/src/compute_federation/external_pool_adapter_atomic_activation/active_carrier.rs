use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use crate::compute_federation::provider::{
    ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE,
};

use super::{
    canonical_task_protocol_active_carrier_json_and_digest,
    ExternalPoolAdapterAtomicActivationReceipt,
    ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial,
    ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
};

/// Genesis carrier proof. Its constructor accepts a planned transition, never a durable witness.
pub(crate) struct ExternalPoolAdapterTaskProtocolGenesisActiveCarrier {
    material: ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
}

/// Refresh carrier proof. Its constructor accepts a durable V277 witness and a live Provider.
pub(crate) struct ExternalPoolAdapterTaskProtocolRefreshActiveCarrier {
    material: ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
}

impl ExternalPoolAdapterTaskProtocolGenesisActiveCarrier {
    pub(crate) fn new(
        material: ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
        transition: &ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial,
    ) -> Result<Self> {
        if material.provider_binding_id != transition.provider_binding_id
            || material.provider_binding_digest != transition.provider_binding_digest
            || material.activation_root_digest != transition.activation_root_digest
            || material.target_active_provider_id != transition.target_active_provider_id
            || material.target_active_provider_policy_revision
                != transition.target_active_provider_policy_revision
            || material.target_active_provider_digest != transition.target_active_provider_digest
            || material.route_adapter_projection_id != transition.route_adapter_projection_id
        {
            bail!("V277 genesis active carrier does not match planned transition")
        }
        canonical_task_protocol_active_carrier_json_and_digest(&material)?;
        Ok(Self { material })
    }

    pub(crate) fn material(&self) -> &ExternalPoolAdapterTaskProtocolActiveCarrierMaterial {
        &self.material
    }

    pub(crate) fn canonical_json_and_digest(&self) -> Result<(String, String)> {
        canonical_task_protocol_active_carrier_json_and_digest(&self.material)
    }
}

impl ExternalPoolAdapterTaskProtocolRefreshActiveCarrier {
    pub(crate) fn new(
        material: ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
        witness: &ExternalPoolAdapterAtomicActivationReceipt,
        live_provider: &ComputeProvider,
    ) -> Result<Self> {
        let provider_json = serde_json::to_string(live_provider)?;
        let provider_digest = hex::encode(Sha256::digest(provider_json.as_bytes()));
        if live_provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
            || live_provider.status != PROVIDER_STATUS_ACTIVE
            || material.provider_binding_id != witness.activation.identity.provider_binding_id
            || material.provider_binding_digest
                != witness.activation.identity.provider_binding_digest
            || material.activation_root_digest != witness.activation.identity.activation_root_digest
            || material.target_active_provider_id != live_provider.provider_id
            || material.target_active_provider_policy_revision != live_provider.policy_revision
            || material.target_active_provider_digest != provider_digest
            || material.route_adapter_projection_id
                != witness.activation.route_closure.route_adapter_projection_id
            || live_provider
                .adapter
                .as_ref()
                .is_none_or(|adapter| adapter.adapter_id != material.route_adapter_projection_id)
        {
            bail!("V277 refresh active carrier lacks exact witness/live Provider roots")
        }
        canonical_task_protocol_active_carrier_json_and_digest(&material)?;
        Ok(Self { material })
    }

    pub(crate) fn material(&self) -> &ExternalPoolAdapterTaskProtocolActiveCarrierMaterial {
        &self.material
    }

    pub(crate) fn canonical_json_and_digest(&self) -> Result<(String, String)> {
        canonical_task_protocol_active_carrier_json_and_digest(&self.material)
    }
}
