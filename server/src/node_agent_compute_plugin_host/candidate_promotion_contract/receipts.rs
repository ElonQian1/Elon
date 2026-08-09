use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    local_authority::{
        ComputePluginCandidatePromotionAuthorityFacts,
        ComputePluginPostRevalidationPromotionAuthoritySession,
    },
};

mod install;
mod promotion;

pub(in crate::node_agent_compute_plugin_host) use install::{
    ComputePluginInstallReceipt, HashedComputePluginInstallReceipt,
};
pub(in crate::node_agent_compute_plugin_host) use promotion::{
    ComputePluginPromotionReceipt, HashedComputePluginPromotionReceipt,
};

macro_rules! transition_getters {
    ($($name:ident: $ty:ty,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> $ty {
            self.$name
        }
    )*};
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginAuthorityRevisionTransition {
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginInstallGenerationTransition {
    install_generation_before: i64,
    install_generation_after: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginActivationGenerationTransition {
    activation_generation_before: i64,
    activation_generation_after: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPreviousActiveSlot {
    slot_ref: String,
    release: ComputePluginReleaseRef,
    install_receipt_digest: String,
    promotion_receipt_digest: String,
}

/// The exact immutable Store result pair. Neither envelope is cloneable, so the result remains
/// attached to the installed-slot custody that owns the pinned candidate handles.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionReceiptPair {
    install: HashedComputePluginInstallReceipt,
    promotion: HashedComputePluginPromotionReceipt,
}

impl ComputePluginAuthorityRevisionTransition {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
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
    ) -> Self {
        Self {
            authority_state_revision_before,
            authority_state_revision_after,
            inventory_revision_before,
            inventory_revision_after,
            inventory_digest_before,
            inventory_digest_after,
            authority_epoch_before,
            authority_epoch_after,
            process_owner_epoch,
            trusted_time_high_water_ms_before,
            authority_updated_at_ms_before,
        }
    }

    transition_getters! {
        authority_state_revision_before: i64,
        authority_state_revision_after: i64,
        inventory_revision_before: i64,
        inventory_revision_after: i64,
        authority_epoch_before: i64,
        authority_epoch_after: i64,
        process_owner_epoch: i64,
        trusted_time_high_water_ms_before: i64,
        authority_updated_at_ms_before: i64,
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_before(&self) -> &str {
        &self.inventory_digest_before
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest_after(&self) -> &str {
        &self.inventory_digest_after
    }
}

impl ComputePluginInstallGenerationTransition {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        install_generation_before: i64,
        install_generation_after: i64,
    ) -> Self {
        Self {
            install_generation_before,
            install_generation_after,
        }
    }

    transition_getters! {
        install_generation_before: i64,
        install_generation_after: i64,
    }
}

impl ComputePluginActivationGenerationTransition {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        activation_generation_before: i64,
        activation_generation_after: i64,
    ) -> Self {
        Self {
            activation_generation_before,
            activation_generation_after,
        }
    }

    transition_getters! {
        activation_generation_before: i64,
        activation_generation_after: i64,
    }
}

impl ComputePluginPreviousActiveSlot {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        slot_ref: String,
        release: ComputePluginReleaseRef,
        install_receipt_digest: String,
        promotion_receipt_digest: String,
    ) -> Self {
        Self {
            slot_ref,
            release,
            install_receipt_digest,
            promotion_receipt_digest,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn slot_ref(&self) -> &str {
        &self.slot_ref
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn install_receipt_digest(&self) -> &str {
        &self.install_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn promotion_receipt_digest(&self) -> &str {
        &self.promotion_receipt_digest
    }
}

impl CandidatePromotionReceiptPair {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        install: HashedComputePluginInstallReceipt,
        promotion: HashedComputePluginPromotionReceipt,
    ) -> Result<Self> {
        let pair = Self { install, promotion };
        pair.validate()?;
        Ok(pair)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate(&self) -> Result<()> {
        self.install.validate()?;
        self.promotion.validate()?;
        let install_body = self.install.receipt();
        let promotion_body = self.promotion.receipt();
        if promotion_body.install_receipt_id() != install_body.install_receipt_id()
            || promotion_body.promotion_receipt_id() != install_body.promotion_id()
            || promotion_body.install_receipt_digest() != self.install.receipt_digest()
            || promotion_body.installation_id_digest() != install_body.installation_id_digest()
            || promotion_body.candidate_token_digest() != install_body.candidate_token_digest()
            || promotion_body.plugin_id() != install_body.plugin_id()
            || promotion_body.slot_ref() != install_body.slot_ref()
            || promotion_body.release() != install_body.release()
            || promotion_body.health_id() != install_body.health_id()
            || promotion_body.health_receipt_digest() != install_body.health_receipt_digest()
            || promotion_body.signed_manifest_envelope_digest()
                != install_body.signed_manifest_envelope_digest()
            || promotion_body.owner_plan_id() != install_body.owner_plan_id()
            || promotion_body.owner_plan_digest() != install_body.owner_plan_digest()
            || promotion_body.application_inventory_revision()
                != install_body.application_inventory_revision()
            || promotion_body.permission_grant_digest() != install_body.permission_grant_digest()
            || promotion_body.install_generation_after() != install_body.install_generation_after()
            || promotion_body.authority_state_revision_before()
                != install_body.authority_state_revision_before()
            || promotion_body.authority_state_revision_after()
                != install_body.authority_state_revision_after()
            || promotion_body.inventory_revision_before()
                != install_body.inventory_revision_before()
            || promotion_body.inventory_revision_after() != install_body.inventory_revision_after()
            || promotion_body.inventory_digest_before() != install_body.inventory_digest_before()
            || promotion_body.inventory_digest_after() != install_body.inventory_digest_after()
            || promotion_body.authority_epoch_before() != install_body.authority_epoch_before()
            || promotion_body.authority_epoch_after() != install_body.authority_epoch_after()
            || promotion_body.process_owner_epoch() != install_body.process_owner_epoch()
            || promotion_body.trusted_time_high_water_ms_before()
                != install_body.trusted_time_high_water_ms_before()
            || promotion_body.authority_updated_at_ms_before()
                != install_body.authority_updated_at_ms_before()
            || promotion_body.promoted_at_ms() != install_body.installed_at_ms()
        {
            bail!("COMPUTE_PLUGIN_PROMOTION_RECEIPT_PAIR_CHANGED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn install(
        &self,
    ) -> &HashedComputePluginInstallReceipt {
        &self.install
    }

    pub(in crate::node_agent_compute_plugin_host) fn promotion(
        &self,
    ) -> &HashedComputePluginPromotionReceipt {
        &self.promotion
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        HashedComputePluginInstallReceipt,
        HashedComputePluginPromotionReceipt,
    ) {
        (self.install, self.promotion)
    }
}

pub(super) fn build_candidate_promotion_receipts(
    session: &ComputePluginPostRevalidationPromotionAuthoritySession<'_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
    install_id: &str,
    promotion_id: &str,
) -> Result<CandidatePromotionReceiptPair> {
    let install = ComputePluginInstallReceipt::new(
        install_id.to_string(),
        promotion_id.to_string(),
        session.installation_id_digest().to_string(),
        facts.candidate_token_digest().to_string(),
        facts.candidate_generation(),
        facts.plugin_id().to_string(),
        facts.slot_ref().to_string(),
        facts.release().clone(),
        facts.staging_id().to_string(),
        facts.staging_receipt_digest().to_string(),
        facts.staging_run_digest().to_string(),
        facts.extraction_plan_digest().to_string(),
        facts.extraction_evidence_digest().to_string(),
        facts.staging_seal_payload_digest().to_string(),
        facts.staging_seal_file_digest().to_string(),
        facts.staging_seal_identity_digest().to_string(),
        facts.health_id().to_string(),
        facts.health_receipt_digest().to_string(),
        facts.health_observation_digest().to_string(),
        facts.owner_plan_id().to_string(),
        facts.owner_plan_digest().to_string(),
        facts.application_inventory_revision(),
        facts.permission_grant_digest().to_string(),
        facts.signed_manifest_envelope_digest().to_string(),
        revision_transition(facts),
        ComputePluginInstallGenerationTransition::new(
            facts.install_generation_before(),
            facts.install_generation_after(),
        ),
        facts.promoted_at_ms(),
    )?;
    let install = HashedComputePluginInstallReceipt::from_store_receipt(install)?;
    let previous_active = facts.previous_active().map(|previous| {
        ComputePluginPreviousActiveSlot::new(
            previous.slot_ref().to_string(),
            previous.release().clone(),
            previous.install_receipt_digest().to_string(),
            previous.promotion_receipt_digest().to_string(),
        )
    });
    let promotion = ComputePluginPromotionReceipt::new(
        promotion_id.to_string(),
        install_id.to_string(),
        install.receipt_digest().to_string(),
        session.installation_id_digest().to_string(),
        facts.candidate_token_digest().to_string(),
        facts.plugin_id().to_string(),
        facts.slot_ref().to_string(),
        facts.release().clone(),
        facts.health_id().to_string(),
        facts.health_receipt_digest().to_string(),
        facts.owner_plan_id().to_string(),
        facts.owner_plan_digest().to_string(),
        facts.application_inventory_revision(),
        facts.permission_grant_digest().to_string(),
        facts.signed_manifest_envelope_digest().to_string(),
        revision_transition(facts),
        facts.install_generation_after(),
        ComputePluginActivationGenerationTransition::new(
            facts.activation_generation_before(),
            facts.activation_generation_after(),
        ),
        previous_active,
        facts.promoted_at_ms(),
    )?;
    let promotion = HashedComputePluginPromotionReceipt::from_store_receipt(promotion)?;
    CandidatePromotionReceiptPair::new(install, promotion)
}

fn revision_transition(
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
) -> ComputePluginAuthorityRevisionTransition {
    ComputePluginAuthorityRevisionTransition::new(
        facts.authority_state_revision_before(),
        facts.authority_state_revision_after(),
        facts.inventory_revision_before(),
        facts.inventory_revision_after(),
        facts.inventory_digest_before().to_string(),
        facts.inventory_digest_after().to_string(),
        facts.authority_epoch_before(),
        facts.authority_epoch_after(),
        facts.process_owner_epoch(),
        facts.trusted_time_high_water_ms_before(),
        facts.authority_updated_at_ms_before(),
    )
}
