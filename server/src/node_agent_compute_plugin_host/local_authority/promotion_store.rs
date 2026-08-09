use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::{
        ComputePluginPreviousActiveSlot, RevalidatedCandidatePromotion,
        ValidatedCandidatePromotionStorePermit,
    },
    fetch_contract::ComputePluginFetchCancellationGuard,
    identity::ComputePluginReleaseRef,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod binding;
mod meta;
mod projection;
mod readback;
mod recovery;
mod write;

pub(in crate::node_agent_compute_plugin_host) use recovery::ComputePluginCandidatePromotionRecoveryAuthoritySession;
pub(in crate::node_agent_compute_plugin_host) use crate::node_agent_compute_plugin_host::candidate_promotion_contract::{
    ComputePluginCandidatePromotionRecoveryOutcome, HashedComputePluginInstallReceipt,
    HashedComputePluginPromotionReceipt,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostRevalidationPromotionAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidatePromotionAuthorityFacts {
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
    candidate_token_digest: String,
    plugin_id: String,
    slot_ref: String,
    candidate_generation: i64,
    release: ComputePluginReleaseRef,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
    permission_grant_digest: String,
    signed_manifest_envelope_digest: String,
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
    install_generation_before: i64,
    install_generation_after: i64,
    activation_generation_before: i64,
    activation_generation_after: i64,
    previous_active: Option<ComputePluginPreviousActiveSlot>,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_promotion_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: &ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginPostRevalidationPromotionAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_SESSION_INVALID");
        }
        Ok(ComputePluginPostRevalidationPromotionAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginPostRevalidationPromotionAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.trusted_now.timestamp_millis()
    }

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.observed_at > barrier
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        guard.validate_source(self.process_fence.cancellation_source())?;
        guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_promotion_binding(
        &self,
        promotion: &RevalidatedCandidatePromotion<'_>,
    ) -> Result<ComputePluginCandidatePromotionAuthorityFacts> {
        let guard = promotion.staged().archive().snapshot_cancellation_guard();
        self.validate_source(&guard)?;
        self.authority.with_deferred(|transaction| {
            binding::read_candidate_promotion_binding(transaction, self, promotion)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn persist_candidate_promotion(
        &self,
        permit: ValidatedCandidatePromotionStorePermit<'_, '_>,
    ) -> Result<()> {
        self.authority.with_immediate(|transaction| {
            write::persist_candidate_promotion(transaction, self, permit)
        })
    }
}

macro_rules! fact_getter {
    ($name:ident, $field:ident, $ty:ty) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> $ty {
            &self.$field
        }
    };
    ($name:ident, $field:ident, copy $ty:ty) => {
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> $ty {
            self.$field
        }
    };
}

impl ComputePluginCandidatePromotionAuthorityFacts {
    fact_getter!(authority_state_revision_before, authority_state_revision_before, copy i64);
    fact_getter!(authority_state_revision_after, authority_state_revision_after, copy i64);
    fact_getter!(inventory_revision_before, inventory_revision_before, copy i64);
    fact_getter!(inventory_revision_after, inventory_revision_after, copy i64);
    fact_getter!(inventory_digest_before, inventory_digest_before, str);
    fact_getter!(inventory_digest_after, inventory_digest_after, str);
    fact_getter!(authority_epoch_before, authority_epoch_before, copy i64);
    fact_getter!(authority_epoch_after, authority_epoch_after, copy i64);
    fact_getter!(process_owner_epoch, process_owner_epoch, copy i64);
    fact_getter!(trusted_time_high_water_ms_before, trusted_time_high_water_ms_before, copy i64);
    fact_getter!(authority_updated_at_ms_before, authority_updated_at_ms_before, copy i64);
    fact_getter!(promoted_at_ms, promoted_at_ms, copy i64);
    fact_getter!(candidate_token_digest, candidate_token_digest, str);
    fact_getter!(plugin_id, plugin_id, str);
    fact_getter!(slot_ref, slot_ref, str);
    fact_getter!(candidate_generation, candidate_generation, copy i64);
    fact_getter!(release, release, ComputePluginReleaseRef);
    fact_getter!(owner_plan_id, owner_plan_id, str);
    fact_getter!(owner_plan_digest, owner_plan_digest, str);
    fact_getter!(application_inventory_revision, application_inventory_revision, copy i64);
    fact_getter!(permission_grant_digest, permission_grant_digest, str);
    fact_getter!(
        signed_manifest_envelope_digest,
        signed_manifest_envelope_digest,
        str
    );
    fact_getter!(staging_id, staging_id, str);
    fact_getter!(staging_receipt_digest, staging_receipt_digest, str);
    fact_getter!(staging_run_digest, staging_run_digest, str);
    fact_getter!(extraction_plan_digest, extraction_plan_digest, str);
    fact_getter!(extraction_evidence_digest, extraction_evidence_digest, str);
    fact_getter!(
        staging_seal_payload_digest,
        staging_seal_payload_digest,
        str
    );
    fact_getter!(staging_seal_file_digest, staging_seal_file_digest, str);
    fact_getter!(
        staging_seal_identity_digest,
        staging_seal_identity_digest,
        str
    );
    fact_getter!(health_id, health_id, str);
    fact_getter!(health_receipt_digest, health_receipt_digest, str);
    fact_getter!(health_observation_digest, health_observation_digest, str);
    fact_getter!(install_generation_before, install_generation_before, copy i64);
    fact_getter!(install_generation_after, install_generation_after, copy i64);
    fact_getter!(activation_generation_before, activation_generation_before, copy i64);
    fact_getter!(activation_generation_after, activation_generation_after, copy i64);

    pub(in crate::node_agent_compute_plugin_host) fn previous_active(
        &self,
    ) -> Option<&ComputePluginPreviousActiveSlot> {
        self.previous_active.as_ref()
    }
}
