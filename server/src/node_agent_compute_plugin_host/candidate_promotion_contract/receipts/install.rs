use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan_admission_validation::is_identifier,
    lifecycle::{SLOT_INSTALLED, SLOT_STAGED},
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    super::{
        HASHED_INSTALL_RECEIPT_SCHEMA, INSTALL_RECEIPT_SCHEMA, RECEIPT_CANONICALIZATION,
        RECEIPT_DIGEST_ALGORITHM,
    },
    ComputePluginAuthorityRevisionTransition, ComputePluginInstallGenerationTransition,
};

macro_rules! string_getters {
    ($($name:ident,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

macro_rules! number_getters {
    ($($name:ident,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
            self.$name
        }
    )*};
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginInstallReceipt {
    schema: String,
    install_receipt_id: String,
    promotion_id: String,
    installation_id_digest: String,
    candidate_token_digest: String,
    candidate_generation: i64,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    staging_id: String,
    staging_receipt_digest: String,
    staging_run_digest: String,
    extraction_plan_digest: String,
    extraction_evidence_digest: String,
    staging_seal_payload_digest: String,
    staging_seal_file_digest: String,
    staging_seal_identity_digest: String,
    health_id: String,
    health_receipt_digest: String,
    health_observation_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
    permission_grant_digest: String,
    signed_manifest_envelope_digest: String,
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms_before: i64,
    authority_updated_at_ms_before: i64,
    installed_at_ms: i64,
    install_generation_before: i64,
    install_generation_after: i64,
    slot_phase_before: String,
    slot_phase_after: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginInstallReceipt {
    schema: String,
    receipt: ComputePluginInstallReceipt,
    canonicalization: String,
    digest_algorithm: String,
    receipt_digest: String,
}

impl ComputePluginInstallReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
        install_receipt_id: String,
        promotion_id: String,
        installation_id_digest: String,
        candidate_token_digest: String,
        candidate_generation: i64,
        plugin_id: String,
        slot_ref: String,
        release: ComputePluginReleaseRef,
        staging_id: String,
        staging_receipt_digest: String,
        staging_run_digest: String,
        extraction_plan_digest: String,
        extraction_evidence_digest: String,
        staging_seal_payload_digest: String,
        staging_seal_file_digest: String,
        staging_seal_identity_digest: String,
        health_id: String,
        health_receipt_digest: String,
        health_observation_digest: String,
        owner_plan_id: String,
        owner_plan_digest: String,
        application_inventory_revision: i64,
        permission_grant_digest: String,
        signed_manifest_envelope_digest: String,
        revisions: ComputePluginAuthorityRevisionTransition,
        generations: ComputePluginInstallGenerationTransition,
        installed_at_ms: i64,
    ) -> Result<Self> {
        let receipt = Self {
            schema: INSTALL_RECEIPT_SCHEMA.to_string(),
            install_receipt_id,
            promotion_id,
            installation_id_digest,
            candidate_token_digest,
            candidate_generation,
            plugin_id,
            slot_ref,
            release,
            staging_id,
            staging_receipt_digest,
            staging_run_digest,
            extraction_plan_digest,
            extraction_evidence_digest,
            staging_seal_payload_digest,
            staging_seal_file_digest,
            staging_seal_identity_digest,
            health_id,
            health_receipt_digest,
            health_observation_digest,
            owner_plan_id,
            owner_plan_digest,
            application_inventory_revision,
            permission_grant_digest,
            signed_manifest_envelope_digest,
            authority_state_revision_before: revisions.authority_state_revision_before,
            authority_state_revision_after: revisions.authority_state_revision_after,
            inventory_revision_before: revisions.inventory_revision_before,
            inventory_revision_after: revisions.inventory_revision_after,
            inventory_digest_before: revisions.inventory_digest_before,
            inventory_digest_after: revisions.inventory_digest_after,
            authority_epoch_before: revisions.authority_epoch_before,
            authority_epoch_after: revisions.authority_epoch_after,
            process_owner_epoch: revisions.process_owner_epoch,
            trusted_time_high_water_ms_before: revisions.trusted_time_high_water_ms_before,
            authority_updated_at_ms_before: revisions.authority_updated_at_ms_before,
            installed_at_ms,
            install_generation_before: generations.install_generation_before,
            install_generation_after: generations.install_generation_after,
            slot_phase_before: SLOT_STAGED.to_string(),
            slot_phase_after: SLOT_INSTALLED.to_string(),
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub(super) fn validate(&self) -> Result<()> {
        let digests = [
            self.installation_id_digest.as_str(),
            self.candidate_token_digest.as_str(),
            self.staging_receipt_digest.as_str(),
            self.staging_run_digest.as_str(),
            self.extraction_plan_digest.as_str(),
            self.extraction_evidence_digest.as_str(),
            self.staging_seal_payload_digest.as_str(),
            self.staging_seal_file_digest.as_str(),
            self.staging_seal_identity_digest.as_str(),
            self.health_receipt_digest.as_str(),
            self.health_observation_digest.as_str(),
            self.owner_plan_digest.as_str(),
            self.permission_grant_digest.as_str(),
            self.signed_manifest_envelope_digest.as_str(),
            self.inventory_digest_before.as_str(),
            self.inventory_digest_after.as_str(),
        ];
        if self.schema != INSTALL_RECEIPT_SCHEMA
            || !is_identifier(&self.install_receipt_id)
            || !is_identifier(&self.promotion_id)
            || !is_identifier(&self.staging_id)
            || !is_identifier(&self.health_id)
            || !is_identifier(&self.owner_plan_id)
            || self.plugin_id.trim().is_empty()
            || self.slot_ref.trim().is_empty()
            || self.candidate_generation <= 0
            || self.application_inventory_revision <= 0
            || self.process_owner_epoch <= 0
            || self.installed_at_ms <= self.trusted_time_high_water_ms_before
            || self.installed_at_ms <= self.authority_updated_at_ms_before
            || self.authority_state_revision_after
                != self
                    .authority_state_revision_before
                    .checked_add(1)
                    .unwrap_or(-1)
            || self.inventory_revision_after
                != self.inventory_revision_before.checked_add(1).unwrap_or(-1)
            || self.authority_epoch_after
                != self.authority_epoch_before.checked_add(1).unwrap_or(-1)
            || self.install_generation_after != self.candidate_generation
            || self.install_generation_after <= self.install_generation_before
            || self.slot_phase_before != SLOT_STAGED
            || self.slot_phase_after != SLOT_INSTALLED
            || self.inventory_digest_before == self.inventory_digest_after
            || digests.iter().any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_INSTALL_RECEIPT_INVALID");
        }
        Ok(())
    }

    string_getters! {
        install_receipt_id,
        promotion_id,
        installation_id_digest,
        candidate_token_digest,
        plugin_id,
        slot_ref,
        staging_id,
        staging_receipt_digest,
        staging_run_digest,
        extraction_plan_digest,
        extraction_evidence_digest,
        staging_seal_payload_digest,
        staging_seal_file_digest,
        staging_seal_identity_digest,
        health_id,
        health_receipt_digest,
        health_observation_digest,
        owner_plan_id,
        owner_plan_digest,
        permission_grant_digest,
        signed_manifest_envelope_digest,
        inventory_digest_before,
        inventory_digest_after,
        slot_phase_before,
        slot_phase_after,
    }

    number_getters! {
        candidate_generation,
        application_inventory_revision,
        authority_state_revision_before,
        authority_state_revision_after,
        inventory_revision_before,
        inventory_revision_after,
        authority_epoch_before,
        authority_epoch_after,
        process_owner_epoch,
        trusted_time_high_water_ms_before,
        authority_updated_at_ms_before,
        installed_at_ms,
        install_generation_before,
        install_generation_after,
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }
}

impl HashedComputePluginInstallReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn from_store_receipt(
        receipt: ComputePluginInstallReceipt,
    ) -> Result<Self> {
        receipt.validate()?;
        let receipt_digest = jcs_sha256_hex(&receipt)?;
        Self::from_store_readback(receipt, receipt_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn from_store_readback(
        receipt: ComputePluginInstallReceipt,
        receipt_digest: String,
    ) -> Result<Self> {
        let hashed = Self {
            schema: HASHED_INSTALL_RECEIPT_SCHEMA.to_string(),
            receipt,
            canonicalization: RECEIPT_CANONICALIZATION.to_string(),
            digest_algorithm: RECEIPT_DIGEST_ALGORITHM.to_string(),
            receipt_digest,
        };
        hashed.validate()?;
        Ok(hashed)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.receipt.validate()?;
        if self.schema != HASHED_INSTALL_RECEIPT_SCHEMA
            || self.canonicalization != RECEIPT_CANONICALIZATION
            || self.digest_algorithm != RECEIPT_DIGEST_ALGORITHM
            || !is_sha256(&self.receipt_digest)
            || jcs_sha256_hex(&self.receipt)? != self.receipt_digest
        {
            bail!("COMPUTE_PLUGIN_HASHED_INSTALL_RECEIPT_INVALID");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginInstallReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}
