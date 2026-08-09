use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_INSTALLED, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::{
    super::{
        HASHED_PROMOTION_RECEIPT_SCHEMA, PROMOTION_RECEIPT_SCHEMA, RECEIPT_CANONICALIZATION,
        RECEIPT_DIGEST_ALGORITHM,
    },
    ComputePluginActivationGenerationTransition, ComputePluginAuthorityRevisionTransition,
    ComputePluginPreviousActiveSlot,
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
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPromotionReceipt {
    schema: String,
    promotion_receipt_id: String,
    install_receipt_id: String,
    install_receipt_digest: String,
    installation_id_digest: String,
    candidate_token_digest: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    health_id: String,
    health_receipt_digest: String,
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
    promoted_at_ms: i64,
    install_generation_after: i64,
    activation_generation_before: i64,
    activation_generation_after: i64,
    previous_active_slot_ref: Option<String>,
    previous_active_release: Option<ComputePluginReleaseRef>,
    previous_active_install_receipt_digest: Option<String>,
    previous_active_promotion_receipt_digest: Option<String>,
    active_slot_ref_after: String,
    active_release_after: ComputePluginReleaseRef,
    slot_phase_after: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginPromotionReceipt {
    schema: String,
    receipt: ComputePluginPromotionReceipt,
    canonicalization: String,
    digest_algorithm: String,
    receipt_digest: String,
}

impl ComputePluginPromotionReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
        promotion_receipt_id: String,
        install_receipt_id: String,
        install_receipt_digest: String,
        installation_id_digest: String,
        candidate_token_digest: String,
        plugin_id: String,
        slot_ref: String,
        release: ComputePluginReleaseRef,
        health_id: String,
        health_receipt_digest: String,
        owner_plan_id: String,
        owner_plan_digest: String,
        application_inventory_revision: i64,
        permission_grant_digest: String,
        signed_manifest_envelope_digest: String,
        revisions: ComputePluginAuthorityRevisionTransition,
        install_generation_after: i64,
        activation: ComputePluginActivationGenerationTransition,
        previous_active: Option<ComputePluginPreviousActiveSlot>,
        promoted_at_ms: i64,
    ) -> Result<Self> {
        let (
            previous_active_slot_ref,
            previous_active_release,
            previous_active_install_receipt_digest,
            previous_active_promotion_receipt_digest,
        ) = match previous_active {
            Some(previous) => (
                Some(previous.slot_ref),
                Some(previous.release),
                Some(previous.install_receipt_digest),
                Some(previous.promotion_receipt_digest),
            ),
            None => (None, None, None, None),
        };
        let active_slot_ref_after = slot_ref.clone();
        let active_release_after = release.clone();
        let receipt = Self {
            schema: PROMOTION_RECEIPT_SCHEMA.to_string(),
            promotion_receipt_id,
            install_receipt_id,
            install_receipt_digest,
            installation_id_digest,
            candidate_token_digest,
            plugin_id,
            slot_ref,
            release,
            health_id,
            health_receipt_digest,
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
            promoted_at_ms,
            install_generation_after,
            activation_generation_before: activation.activation_generation_before,
            activation_generation_after: activation.activation_generation_after,
            previous_active_slot_ref,
            previous_active_release,
            previous_active_install_receipt_digest,
            previous_active_promotion_receipt_digest,
            active_slot_ref_after,
            active_release_after,
            slot_phase_after: SLOT_INSTALLED.to_string(),
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub(super) fn validate(&self) -> Result<()> {
        let previous_group = [
            self.previous_active_slot_ref.is_some(),
            self.previous_active_release.is_some(),
            self.previous_active_install_receipt_digest.is_some(),
            self.previous_active_promotion_receipt_digest.is_some(),
        ];
        let previous_digests_valid = self
            .previous_active_install_receipt_digest
            .as_deref()
            .is_none_or(is_sha256)
            && self
                .previous_active_promotion_receipt_digest
                .as_deref()
                .is_none_or(is_sha256);
        let digests = [
            self.install_receipt_digest.as_str(),
            self.installation_id_digest.as_str(),
            self.candidate_token_digest.as_str(),
            self.health_receipt_digest.as_str(),
            self.owner_plan_digest.as_str(),
            self.permission_grant_digest.as_str(),
            self.signed_manifest_envelope_digest.as_str(),
            self.inventory_digest_before.as_str(),
            self.inventory_digest_after.as_str(),
        ];
        if self.schema != PROMOTION_RECEIPT_SCHEMA
            || !is_identifier(&self.promotion_receipt_id)
            || !is_identifier(&self.install_receipt_id)
            || !is_identifier(&self.health_id)
            || !is_identifier(&self.owner_plan_id)
            || self.plugin_id.trim().is_empty()
            || self.slot_ref.trim().is_empty()
            || self.process_owner_epoch <= 0
            || self.application_inventory_revision <= 0
            || self.promoted_at_ms <= self.trusted_time_high_water_ms_before
            || self.promoted_at_ms <= self.authority_updated_at_ms_before
            || self.install_generation_after <= 0
            || self.authority_state_revision_after
                != self
                    .authority_state_revision_before
                    .checked_add(1)
                    .unwrap_or(-1)
            || self.inventory_revision_after
                != self.inventory_revision_before.checked_add(1).unwrap_or(-1)
            || self.authority_epoch_after
                != self.authority_epoch_before.checked_add(1).unwrap_or(-1)
            || self.activation_generation_after
                != self
                    .activation_generation_before
                    .checked_add(1)
                    .unwrap_or(-1)
            || previous_group
                .iter()
                .any(|present| *present != previous_group[0])
            || !previous_digests_valid
            || self.active_slot_ref_after != self.slot_ref
            || self.active_release_after != self.release
            || self.slot_phase_after != SLOT_INSTALLED
            || self.inventory_digest_before == self.inventory_digest_after
            || digests.iter().any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_PROMOTION_RECEIPT_INVALID");
        }
        Ok(())
    }

    string_getters! {
        promotion_receipt_id,
        install_receipt_id,
        install_receipt_digest,
        installation_id_digest,
        candidate_token_digest,
        plugin_id,
        slot_ref,
        health_id,
        health_receipt_digest,
        owner_plan_id,
        owner_plan_digest,
        permission_grant_digest,
        signed_manifest_envelope_digest,
        inventory_digest_before,
        inventory_digest_after,
        active_slot_ref_after,
        slot_phase_after,
    }

    number_getters! {
        authority_state_revision_before,
        authority_state_revision_after,
        inventory_revision_before,
        inventory_revision_after,
        authority_epoch_before,
        authority_epoch_after,
        process_owner_epoch,
        application_inventory_revision,
        trusted_time_high_water_ms_before,
        authority_updated_at_ms_before,
        promoted_at_ms,
        install_generation_after,
        activation_generation_before,
        activation_generation_after,
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_active_slot_ref(
        &self,
    ) -> Option<&str> {
        self.previous_active_slot_ref.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_active_release(
        &self,
    ) -> Option<&ComputePluginReleaseRef> {
        self.previous_active_release.as_ref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_active_install_receipt_digest(
        &self,
    ) -> Option<&str> {
        self.previous_active_install_receipt_digest.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_active_promotion_receipt_digest(
        &self,
    ) -> Option<&str> {
        self.previous_active_promotion_receipt_digest.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn active_release_after(
        &self,
    ) -> &ComputePluginReleaseRef {
        &self.active_release_after
    }
}

impl HashedComputePluginPromotionReceipt {
    pub(in crate::node_agent_compute_plugin_host) fn from_store_receipt(
        receipt: ComputePluginPromotionReceipt,
    ) -> Result<Self> {
        receipt.validate()?;
        let receipt_digest = jcs_sha256_hex(&receipt)?;
        Self::from_store_readback(receipt, receipt_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn from_store_readback(
        receipt: ComputePluginPromotionReceipt,
        receipt_digest: String,
    ) -> Result<Self> {
        let hashed = Self {
            schema: HASHED_PROMOTION_RECEIPT_SCHEMA.to_string(),
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
        if self.schema != HASHED_PROMOTION_RECEIPT_SCHEMA
            || self.canonicalization != RECEIPT_CANONICALIZATION
            || self.digest_algorithm != RECEIPT_DIGEST_ALGORITHM
            || !is_sha256(&self.receipt_digest)
            || jcs_sha256_hex(&self.receipt)? != self.receipt_digest
        {
            bail!("COMPUTE_PLUGIN_HASHED_PROMOTION_RECEIPT_INVALID");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &ComputePluginPromotionReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}
