use std::fmt;

use anyhow::Error;

use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::DurableInstalledPluginSlot, identity::ComputePluginReleaseRef,
    local_authority::ComputePluginAuthorityInstanceBinding,
};

use super::{
    AuthorizedInstalledWorkAdmission, ComputePluginWorkAdmissionReceiptPair,
    DurableWorkAdmittedPluginSlot, RevalidatedInstalledWorkAdmission,
};

macro_rules! key_string_getters {
    ($($name:ident),* $(,)?) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

macro_rules! expectation_string_getters {
    ($($name:ident),* $(,)?) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> &str {
            &self.$name
        }
    )*};
}

macro_rules! expectation_number_getters {
    ($($name:ident),* $(,)?) => {$(
        pub(in crate::node_agent_compute_plugin_host) fn $name(&self) -> i64 {
            self.$name
        }
    )*};
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionExpectation {
    source_digest: String,
    expected_receipt_digest: String,
    install_receipt_digest: String,
    promotion_receipt_digest: String,
    install_generation: i64,
    activation_generation: i64,
    runtime_generation: i64,
    work_admission_generation_before: i64,
    work_admission_generation_after: i64,
    previous_work_admission_id: Option<String>,
    previous_work_admission_receipt_digest: Option<String>,
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
    admitted_at_ms: i64,
}

/// Non-cloneable process-bound recovery identity. It is neither Store authority nor a retry token.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    work_admission_id: String,
    plugin_id: String,
    slot_ref: String,
    release: ComputePluginReleaseRef,
    expectation: ComputePluginWorkAdmissionExpectation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum InstalledWorkAdmissionStorePhase {
    StoreOutcomeUncertain,
}

#[must_use = "uncertain work admission must be classified by recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionOutcomeUncertainCustody<
    'root,
> {
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
    recovery_key: ComputePluginWorkAdmissionRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionRecoveryStoreFailure<
    'root,
> {
    phase: InstalledWorkAdmissionStorePhase,
    error: Error,
    recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginWorkAdmissionRecoveryOutcome {
    NotCreated,
    AdmittedCurrent(ComputePluginWorkAdmissionReceiptPair),
    CommittedHistorical(ComputePluginWorkAdmissionReceiptPair),
    NotCreatedSuperseded,
}

#[must_use = "post-rehash recovery custody must receive a fresh recovery authority session"]
pub(in crate::node_agent_compute_plugin_host) struct PendingInstalledWorkAdmissionRecoveryAdoption<
    'root,
> {
    recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
    revalidated_at: std::time::Instant,
}

pub(in crate::node_agent_compute_plugin_host) enum InstalledWorkAdmissionRecoveryAdoption<'root> {
    NotCreated(DurableInstalledPluginSlot<'root>),
    AdmittedCurrent(DurableWorkAdmittedPluginSlot<'root>),
    CommittedHistorical {
        installed: DurableInstalledPluginSlot<'root>,
        receipts: ComputePluginWorkAdmissionReceiptPair,
    },
    NotCreatedSuperseded(DurableInstalledPluginSlot<'root>),
}

pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionRecoveryRevalidationFailure<
    'root,
> {
    error: Error,
    recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum InstalledWorkAdmissionRecoveryAdoptionPhase {
    RecoveryAuthorityNotPostRevalidation,
    RecoveryReadOutcomeUncertain,
    RecoveredOutcomePostconditionFailed,
}

pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionRecoveryAdoptionFailure<
    'root,
> {
    phase: InstalledWorkAdmissionRecoveryAdoptionPhase,
    error: Error,
    pending: PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
    observed: Option<ComputePluginWorkAdmissionRecoveryOutcome>,
}

impl ComputePluginWorkAdmissionRecoveryKey {
    pub(super) fn from_authorized(authorized: &AuthorizedInstalledWorkAdmission<'_, '_>) -> Self {
        let session = authorized.authority_session();
        let pair = authorized.receipts();
        let receipt = pair.receipt().receipt();
        let generations = receipt.generations();
        let authority = receipt.authority();
        Self {
            authority_instance_binding: session.authority_instance_binding().clone(),
            installation_id_digest: session.installation_id_digest().to_string(),
            clock_epoch_digest: session.clock_epoch_digest().to_string(),
            work_admission_id: receipt.work_admission_id().to_string(),
            plugin_id: receipt.plugin_id().to_string(),
            slot_ref: receipt.slot_ref().to_string(),
            release: receipt.release().clone(),
            expectation: ComputePluginWorkAdmissionExpectation {
                source_digest: pair.source().source_digest().to_string(),
                expected_receipt_digest: pair.receipt().receipt_digest().to_string(),
                install_receipt_digest: receipt.install_receipt_digest().to_string(),
                promotion_receipt_digest: receipt.promotion_receipt_digest().to_string(),
                install_generation: generations.install_generation(),
                activation_generation: generations.activation_generation(),
                runtime_generation: generations.runtime_generation(),
                work_admission_generation_before: generations.work_admission_generation_before(),
                work_admission_generation_after: generations.work_admission_generation_after(),
                previous_work_admission_id: generations
                    .previous_work_admission_id()
                    .map(str::to_string),
                previous_work_admission_receipt_digest: generations
                    .previous_work_admission_receipt_digest()
                    .map(str::to_string),
                authority_state_revision_before: authority.authority_state_revision_before(),
                authority_state_revision_after: authority.authority_state_revision_after(),
                inventory_revision_before: authority.inventory_revision_before(),
                inventory_revision_after: authority.inventory_revision_after(),
                inventory_digest_before: authority.inventory_digest_before().to_string(),
                inventory_digest_after: authority.inventory_digest_after().to_string(),
                authority_epoch_before: authority.authority_epoch_before(),
                authority_epoch_after: authority.authority_epoch_after(),
                process_owner_epoch: authority.process_owner_epoch(),
                trusted_time_high_water_ms_before: authority.trusted_time_high_water_ms_before(),
                authority_updated_at_ms_before: authority.authority_updated_at_ms_before(),
                admitted_at_ms: receipt.admitted_at_ms(),
            },
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    key_string_getters! {
        installation_id_digest, clock_epoch_digest, work_admission_id, plugin_id, slot_ref,
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn expectation(
        &self,
    ) -> &ComputePluginWorkAdmissionExpectation {
        &self.expectation
    }
}

impl ComputePluginWorkAdmissionExpectation {
    expectation_string_getters! {
        source_digest, expected_receipt_digest, install_receipt_digest,
        promotion_receipt_digest, inventory_digest_before, inventory_digest_after,
    }

    expectation_number_getters! {
        install_generation, activation_generation, runtime_generation,
        work_admission_generation_before, work_admission_generation_after,
        authority_state_revision_before, authority_state_revision_after,
        inventory_revision_before, inventory_revision_after, authority_epoch_before,
        authority_epoch_after, process_owner_epoch, trusted_time_high_water_ms_before,
        authority_updated_at_ms_before, admitted_at_ms,
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_work_admission_id(
        &self,
    ) -> Option<&str> {
        self.previous_work_admission_id.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn previous_work_admission_receipt_digest(
        &self,
    ) -> Option<&str> {
        self.previous_work_admission_receipt_digest.as_deref()
    }
}

impl InstalledWorkAdmissionOutcomeUncertainCustody<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginWorkAdmissionRecoveryKey {
        &self.recovery_key
    }
}

impl<'root> InstalledWorkAdmissionOutcomeUncertainCustody<'root> {
    pub(super) fn new(
        revalidated: RevalidatedInstalledWorkAdmission<'root>,
        recovery_key: ComputePluginWorkAdmissionRecoveryKey,
    ) -> Self {
        Self {
            revalidated,
            recovery_key,
        }
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        RevalidatedInstalledWorkAdmission<'root>,
        ComputePluginWorkAdmissionRecoveryKey,
    ) {
        (self.revalidated, self.recovery_key)
    }
}

impl<'root> PendingInstalledWorkAdmissionRecoveryAdoption<'root> {
    pub(super) fn new(
        recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
        revalidated_at: std::time::Instant,
    ) -> Self {
        Self {
            recovery,
            revalidated_at,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginWorkAdmissionRecoveryKey {
        self.recovery.recovery_key()
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> std::time::Instant {
        self.revalidated_at
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
        std::time::Instant,
    ) {
        (self.recovery, self.revalidated_at)
    }
}

impl<'root> InstalledWorkAdmissionRecoveryStoreFailure<'root> {
    pub(super) fn new(
        error: Error,
        recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
    ) -> Self {
        Self {
            phase: InstalledWorkAdmissionStorePhase::StoreOutcomeUncertain,
            error,
            recovery,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> InstalledWorkAdmissionStorePhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, InstalledWorkAdmissionOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl<'root> InstalledWorkAdmissionRecoveryRevalidationFailure<'root> {
    pub(super) fn new(
        error: Error,
        recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
    ) -> Self {
        Self { error, recovery }
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, InstalledWorkAdmissionOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl<'root> InstalledWorkAdmissionRecoveryAdoptionFailure<'root> {
    pub(super) fn new(
        phase: InstalledWorkAdmissionRecoveryAdoptionPhase,
        error: Error,
        pending: PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
        observed: Option<ComputePluginWorkAdmissionRecoveryOutcome>,
    ) -> Self {
        Self {
            phase,
            error,
            pending,
            observed,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> InstalledWorkAdmissionRecoveryAdoptionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
        Option<ComputePluginWorkAdmissionRecoveryOutcome>,
    ) {
        (self.error, self.pending, self.observed)
    }
}

impl fmt::Debug for ComputePluginWorkAdmissionRecoveryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComputePluginWorkAdmissionRecoveryKey")
            .field("work_admission_id", &"<redacted>")
            .field("plugin_id", &self.plugin_id)
            .field("slot_ref", &self.slot_ref)
            .finish_non_exhaustive()
    }
}
