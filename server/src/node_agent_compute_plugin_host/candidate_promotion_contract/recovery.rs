use anyhow::Error;

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, local_authority::ComputePluginAuthorityInstanceBinding,
};

use super::{
    AuthorizedCandidatePromotion, CandidatePromotionReceiptPair, ComputePluginPreviousActiveSlot,
    DurableInstalledPluginSlot, RevalidatedCandidatePromotion,
};

mod debug;

macro_rules! expectation_number_getters {
    ($($name:ident,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
            self.$name
        }
    )*};
}

macro_rules! expectation_string_getters {
    ($($name:ident,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

macro_rules! key_string_getters {
    ($($name:ident,)*) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

/// Exact postcondition expected from one install-and-promote transaction. This is recovery
/// identity, never a retry permit.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidatePromotionExpectation {
    candidate_generation: i64,
    staging_receipt_digest: String,
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
    install_generation_before: i64,
    install_generation_after: i64,
    activation_generation_before: i64,
    activation_generation_after: i64,
    previous_active_slot_ref: Option<String>,
    previous_active_release: Option<ComputePluginReleaseRef>,
    previous_active_install_receipt_digest: Option<String>,
    previous_active_promotion_receipt_digest: Option<String>,
    expected_install_receipt_digest: String,
    expected_promotion_receipt_digest: String,
}

/// Process-local key for classifying an uncertain commit. It contains no Store authority and
/// cannot be cloned into competing recovery paths.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidatePromotionRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    install_id: String,
    promotion_id: String,
    candidate_token_digest: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    expectation: ComputePluginCandidatePromotionExpectation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidatePromotionStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain candidate promotion must be inspected through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionOutcomeUncertainCustody<
    'root,
> {
    revalidated: RevalidatedCandidatePromotion<'root>,
    recovery_key: ComputePluginCandidatePromotionRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionStoreFailure<'root> {
    phase: CandidatePromotionStorePhase,
    error: Error,
    recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidatePromotionRecoveryOutcome {
    NotCreated,
    Installed(CandidatePromotionReceiptPair),
}

pub(in crate::node_agent_compute_plugin_host) enum CandidatePromotionRecoveryAdoption<'root> {
    NotCreated(
        crate::node_agent_compute_plugin_host::candidate_health_contract::DurableCandidateHealthPublication<
            'root,
        >,
    ),
    Installed(DurableInstalledPluginSlot<'root>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidatePromotionRecoveryAdoptionPhase {
    RecoveryReadOutcomeUncertain,
    RecoveredOutcomePostconditionFailed,
    RetainedContentRevalidationFailed,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionRecoveryAdoptionFailure<
    'root,
> {
    phase: CandidatePromotionRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
    observed: Option<ComputePluginCandidatePromotionRecoveryOutcome>,
}

impl ComputePluginCandidatePromotionExpectation {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
        candidate_generation: i64,
        staging_receipt_digest: String,
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
        install_generation_before: i64,
        install_generation_after: i64,
        activation_generation_before: i64,
        activation_generation_after: i64,
        previous_active: Option<ComputePluginPreviousActiveSlot>,
        expected_install_receipt_digest: String,
        expected_promotion_receipt_digest: String,
    ) -> Self {
        let (
            previous_active_slot_ref,
            previous_active_release,
            previous_active_install_receipt_digest,
            previous_active_promotion_receipt_digest,
        ) = match previous_active {
            Some(previous) => (
                Some(previous.slot_ref().to_string()),
                Some(previous.release().clone()),
                Some(previous.install_receipt_digest().to_string()),
                Some(previous.promotion_receipt_digest().to_string()),
            ),
            None => (None, None, None, None),
        };
        Self {
            candidate_generation,
            staging_receipt_digest,
            health_receipt_digest,
            owner_plan_id,
            owner_plan_digest,
            application_inventory_revision,
            permission_grant_digest,
            signed_manifest_envelope_digest,
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
            promoted_at_ms,
            install_generation_before,
            install_generation_after,
            activation_generation_before,
            activation_generation_after,
            previous_active_slot_ref,
            previous_active_release,
            previous_active_install_receipt_digest,
            previous_active_promotion_receipt_digest,
            expected_install_receipt_digest,
            expected_promotion_receipt_digest,
        }
    }

    expectation_number_getters! {
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
        promoted_at_ms,
        install_generation_before,
        install_generation_after,
        activation_generation_before,
        activation_generation_after,
    }

    expectation_string_getters! {
        staging_receipt_digest,
        health_receipt_digest,
        owner_plan_id,
        owner_plan_digest,
        permission_grant_digest,
        signed_manifest_envelope_digest,
        inventory_digest_before,
        inventory_digest_after,
        expected_install_receipt_digest,
        expected_promotion_receipt_digest,
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
}

impl ComputePluginCandidatePromotionRecoveryKey {
    pub(super) fn from_authorized(authorized: &AuthorizedCandidatePromotion<'_, '_>) -> Self {
        let facts = authorized.facts();
        let session = authorized.authority_session();
        let receipts = authorized.receipts();
        let previous_active = facts.previous_active().map(|previous| {
            ComputePluginPreviousActiveSlot::new(
                previous.slot_ref().to_string(),
                previous.release().clone(),
                previous.install_receipt_digest().to_string(),
                previous.promotion_receipt_digest().to_string(),
            )
        });
        let expectation = ComputePluginCandidatePromotionExpectation::new(
            facts.candidate_generation(),
            facts.staging_receipt_digest().to_string(),
            facts.health_receipt_digest().to_string(),
            facts.owner_plan_id().to_string(),
            facts.owner_plan_digest().to_string(),
            facts.application_inventory_revision(),
            facts.permission_grant_digest().to_string(),
            facts.signed_manifest_envelope_digest().to_string(),
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
            facts.promoted_at_ms(),
            facts.install_generation_before(),
            facts.install_generation_after(),
            facts.activation_generation_before(),
            facts.activation_generation_after(),
            previous_active,
            receipts.install().receipt_digest().to_string(),
            receipts.promotion().receipt_digest().to_string(),
        );
        Self::new(
            session.authority_instance_binding().clone(),
            session.installation_id_digest().to_string(),
            session.clock_epoch_digest().to_string(),
            authorized.install_id().to_string(),
            authorized.promotion_id().to_string(),
            facts.candidate_token_digest().to_string(),
            facts.plugin_id().to_string(),
            facts.slot_ref().to_string(),
            facts.release().clone(),
            expectation,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
        authority_instance_binding: ComputePluginAuthorityInstanceBinding,
        installation_id_digest: String,
        clock_epoch_digest: String,
        install_id: String,
        promotion_id: String,
        candidate_token_digest: String,
        plugin_id: String,
        slot_ref: String,
        release: ComputePluginReleaseRef,
        expectation: ComputePluginCandidatePromotionExpectation,
    ) -> Self {
        Self {
            authority_instance_binding,
            installation_id_digest,
            clock_epoch_digest,
            install_id,
            promotion_id,
            candidate_token_digest,
            plugin_id,
            slot_ref,
            release,
            expectation,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    key_string_getters! {
        installation_id_digest,
        clock_epoch_digest,
        install_id,
        promotion_id,
        candidate_token_digest,
        plugin_id,
        slot_ref,
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn expectation(
        &self,
    ) -> &ComputePluginCandidatePromotionExpectation {
        &self.expectation
    }
}

impl CandidatePromotionOutcomeUncertainCustody<'_> {
    pub(super) fn revalidated(&self) -> &RevalidatedCandidatePromotion<'_> {
        &self.revalidated
    }

    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginCandidatePromotionRecoveryKey {
        &self.recovery_key
    }
}

impl<'root> CandidatePromotionOutcomeUncertainCustody<'root> {
    pub(super) fn new(
        revalidated: RevalidatedCandidatePromotion<'root>,
        recovery_key: ComputePluginCandidatePromotionRecoveryKey,
    ) -> Self {
        Self {
            revalidated,
            recovery_key,
        }
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        RevalidatedCandidatePromotion<'root>,
        ComputePluginCandidatePromotionRecoveryKey,
    ) {
        (self.revalidated, self.recovery_key)
    }
}

impl CandidatePromotionStoreFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(&self) -> CandidatePromotionStorePhase {
        self.phase
    }
}

impl<'root> CandidatePromotionStoreFailure<'root> {
    pub(super) fn new(
        phase: CandidatePromotionStorePhase,
        error: Error,
        recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
    ) -> Self {
        Self {
            phase,
            error,
            recovery,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidatePromotionOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl CandidatePromotionRecoveryAdoptionFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidatePromotionRecoveryAdoptionPhase {
        self.phase
    }
}

impl<'root> CandidatePromotionRecoveryAdoptionFailure<'root> {
    pub(super) fn new(
        phase: CandidatePromotionRecoveryAdoptionPhase,
        error: Error,
        recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
        observed: Option<ComputePluginCandidatePromotionRecoveryOutcome>,
    ) -> Self {
        Self {
            phase,
            error,
            recovery,
            observed,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidatePromotionOutcomeUncertainCustody<'root>,
        Option<ComputePluginCandidatePromotionRecoveryOutcome>,
    ) {
        (self.error, self.recovery, self.observed)
    }
}
