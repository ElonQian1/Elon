use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error};

use super::{
    build_namespace_durable_event, validate_hashed_cleanup_step_event,
    CandidateCleanupNamespaceDurabilityRecoveryKey, HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::PhysicallyDurableCandidateCleanupNamespace,
    local_authority::{
        ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession,
        ComputePluginCandidateCleanupNamespaceDurabilityRecoveryAuthoritySession,
        ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome,
        ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[must_use = "prepared namespace durability must be stored or retain physical custody"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupNamespaceDurability<
    'authority,
> {
    pub(super) physical: PhysicallyDurableCandidateCleanupNamespace,
    pub(super) authority_session:
        ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession<'authority>,
    pub(super) event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) prepared_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupNamespaceDurabilityPermit<
    'permit,
> {
    prepared: &'permit PreparedCandidateCleanupNamespaceDurability<'permit>,
}

#[must_use = "durable namespace journal must proceed to the next ordinal or remain retained"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupNamespace {
    physical: PhysicallyDurableCandidateCleanupNamespace,
    event: HashedComputePluginCandidateCleanupStepEvent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityPreparationFailure
{
    error: Error,
    physical: PhysicallyDurableCandidateCleanupNamespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupNamespaceDurabilityStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain namespace store retains the completed physical barrier until classified"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody
{
    physical: PhysicallyDurableCandidateCleanupNamespace,
    recovery_key: CandidateCleanupNamespaceDurabilityRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityStoreFailure
{
    phase: CandidateCleanupNamespaceDurabilityStorePhase,
    error: Error,
    recovery: CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupNamespaceDurabilityRecoveryAdoption
{
    NotCreated(PhysicallyDurableCandidateCleanupNamespace),
    Durable(DurableCandidateCleanupNamespace),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase
{
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure
{
    phase: CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_namespace_durability<
    'authority,
>(
    physical: PhysicallyDurableCandidateCleanupNamespace,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> Result<
    PreparedCandidateCleanupNamespaceDurability<'authority>,
    CandidateCleanupNamespaceDurabilityPreparationFailure,
> {
    let prepared_at = Instant::now();
    let authority_session = match authority
        .bind_candidate_cleanup_namespace_durability_authority_session(
            process_fence,
            observation,
            &physical,
            prepared_at,
        ) {
        Ok(session) => session,
        Err(error) => return Err(preparation_failure(error, physical)),
    };
    let event = match build_namespace_durable_event(
        physical.plan(),
        physical.intent_event(),
        physical.disposition_event(),
        physical.absence_event(),
        physical.namespace(),
        authority_session.trusted_now_ms(),
    ) {
        Ok(event) => event,
        Err(error) => return Err(preparation_failure(error, physical)),
    };
    if let Err(error) =
        authority_session.validate_candidate_cleanup_namespace_durability(&physical, &event)
    {
        return Err(preparation_failure(error, physical));
    }
    Ok(PreparedCandidateCleanupNamespaceDurability {
        physical,
        authority_session,
        event,
        prepared_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_namespace_durability(
    prepared: PreparedCandidateCleanupNamespaceDurability<'_>,
) -> Result<DurableCandidateCleanupNamespace, CandidateCleanupNamespaceDurabilityStoreFailure> {
    let recovery_key = CandidateCleanupNamespaceDurabilityRecoveryKey::from_prepared(&prepared);
    if let Err(error) = validate_hashed_cleanup_step_event(&prepared.event) {
        return Err(store_failure(
            CandidateCleanupNamespaceDurabilityStorePhase::PreStorePreparation,
            error,
            prepared.physical,
            recovery_key,
        ));
    }
    let stored = {
        let permit = ValidatedCandidateCleanupNamespaceDurabilityPermit {
            prepared: &prepared,
        };
        prepared
            .authority_session
            .persist_candidate_cleanup_namespace_durability(permit)
    };
    let event = match stored {
        Ok(event) => event,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupNamespaceDurabilityStorePhase::StoreOutcomeUncertain,
                error,
                prepared.physical,
                recovery_key,
            ))
        }
    };
    if event != prepared.event {
        return Err(store_failure(
            CandidateCleanupNamespaceDurabilityStorePhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_POSTCONDITION_CHANGED"),
            prepared.physical,
            recovery_key,
        ));
    }
    Ok(DurableCandidateCleanupNamespace {
        physical: prepared.physical,
        event,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_namespace_durability(
    recovery: CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
    observed_outcome: &ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome,
    session: ComputePluginCandidateCleanupNamespaceDurabilityRecoveryAuthoritySession<'_>,
) -> Result<
    CandidateCleanupNamespaceDurabilityRecoveryAdoption,
    CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &session) {
        return Err(adoption_failure(
            CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh =
        match session.read_candidate_cleanup_namespace_durability_outcome(&recovery.recovery_key) {
            Ok(fresh) => fresh,
            Err(error) => {
                return Err(adoption_failure(
                    CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase::OutcomeReadFailed,
                    error,
                    recovery,
                ))
            }
        };
    if &fresh != observed_outcome {
        return Err(adoption_failure(
            CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = session.validate_source(recovery.physical.state().cancellation_guard()) {
        return Err(adoption_failure(
            CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome::NotCreated => {
            Ok(CandidateCleanupNamespaceDurabilityRecoveryAdoption::NotCreated(recovery.physical))
        }
        ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome::Durable(event) => {
            if event != *recovery.recovery_key.namespace_event() {
                return Err(adoption_failure(
                    CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase::OutcomeChanged,
                    anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_EVENT_CHANGED"),
                    recovery,
                ));
            }
            Ok(
                CandidateCleanupNamespaceDurabilityRecoveryAdoption::Durable(
                    DurableCandidateCleanupNamespace {
                        physical: recovery.physical,
                        event,
                    },
                ),
            )
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupNamespaceDurabilityRecoveryAuthoritySession<'_>,
) -> anyhow::Result<()> {
    let key = &recovery.recovery_key;
    let physical = &recovery.physical;
    let staging = physical.state().staging_recovery_key();
    let owner_slot = staging.slot_expectation();
    let owner_receipt = staging.receipt_expectation();
    let expected_event = build_namespace_durable_event(
        physical.plan(),
        physical.intent_event(),
        physical.disposition_event(),
        physical.absence_event(),
        physical.namespace(),
        key.namespace_event().event().recorded_at_ms(),
    )?;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || key.disposition_set_at() != physical.disposition_set_at()
        || key.parent_absence_observed_at() != physical.parent_absence_observed_at()
        || key.barrier_completed_at() != physical.namespace().barrier_completed_at()
        || key.post_absence_observed_at() != physical.namespace().post_absence_observed_at()
        || key.namespace_completed_at() != physical.namespace().completed_at()
        || key.namespace_durability_kind() != physical.namespace().namespace_durability_kind()
        || key.filesystem_kind() != physical.namespace().filesystem_kind()
        || physical.parent_absence_observed_at() <= physical.disposition_set_at()
        || physical.state().completed_step_count() != 0
        || physical.state().execution_plan_digest() != Some(physical.plan().plan_digest())
        || physical.namespace().barrier_completed_at() <= physical.parent_absence_observed_at()
        || physical.namespace().post_absence_observed_at()
            <= physical.namespace().barrier_completed_at()
        || physical.namespace().completed_at() < physical.namespace().post_absence_observed_at()
        || key.candidate_token() != staging.candidate_token()
        || key.owner_plugin_id() != owner_slot.plugin_id.as_str()
        || key.owner_slot_ref() != owner_slot.slot_ref.as_str()
        || key.owner_release() != &owner_slot.release
        || key.owner_candidate_generation() != owner_receipt.candidate_generation
        || key.owner_plan_id() != owner_receipt.owner_plan_id.as_str()
        || key.owner_plan_digest() != owner_receipt.owner_plan_digest.as_str()
        || key.owner_application_inventory_revision()
            != owner_receipt.application_inventory_revision
        || key.authorized_at_ms()
            != physical
                .state()
                .authorization_receipt()
                .receipt()
                .authorized_at_ms()
        || key.authorization_receipt() != physical.state().authorization_receipt()
        || key.plan() != physical.plan()
        || key.intent_event() != physical.intent_event()
        || key.disposition_event() != physical.disposition_event()
        || key.absence_event() != physical.absence_event()
        || key.namespace_event() != &expected_event
        || session.observed_at() <= key.prepared_at()
        || session.observed_at() <= key.namespace_completed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(physical.state().cancellation_guard())
}

fn preparation_failure(
    error: Error,
    physical: PhysicallyDurableCandidateCleanupNamespace,
) -> CandidateCleanupNamespaceDurabilityPreparationFailure {
    CandidateCleanupNamespaceDurabilityPreparationFailure { error, physical }
}

fn store_failure(
    phase: CandidateCleanupNamespaceDurabilityStorePhase,
    error: Error,
    physical: PhysicallyDurableCandidateCleanupNamespace,
    recovery_key: CandidateCleanupNamespaceDurabilityRecoveryKey,
) -> CandidateCleanupNamespaceDurabilityStoreFailure {
    CandidateCleanupNamespaceDurabilityStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody {
            physical,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
) -> CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure {
    CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl ValidatedCandidateCleanupNamespaceDurabilityPermit<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn physical(
        &self,
    ) -> &PhysicallyDurableCandidateCleanupNamespace {
        &self.prepared.physical
    }
    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.prepared.event
    }
}

impl DurableCandidateCleanupNamespace {
    pub(in crate::node_agent_compute_plugin_host) fn physical(
        &self,
    ) -> &PhysicallyDurableCandidateCleanupNamespace {
        &self.physical
    }
    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.event
    }
}

impl CandidateCleanupNamespaceDurabilityPreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, PhysicallyDurableCandidateCleanupNamespace) {
        (self.error, self.physical)
    }
}

impl CandidateCleanupNamespaceDurabilityStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupNamespaceDurabilityStorePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
    ) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupNamespaceDurabilityRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupNamespaceDurabilityRecoveryAdoptionPhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateCleanupNamespaceDurabilityOutcomeUncertainCustody,
    ) {
        (self.error, self.recovery)
    }
}

macro_rules! impl_failure {
    ($failure:ident) => {
        impl fmt::Display for $failure {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:#}", self.error)
            }
        }
        impl fmt::Debug for $failure {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($failure))
                    .field("error", &self.error)
                    .finish_non_exhaustive()
            }
        }
        impl StdError for $failure {}
    };
}

impl_failure!(CandidateCleanupNamespaceDurabilityPreparationFailure);
impl_failure!(CandidateCleanupNamespaceDurabilityStoreFailure);
impl_failure!(CandidateCleanupNamespaceDurabilityRecoveryAdoptionFailure);
