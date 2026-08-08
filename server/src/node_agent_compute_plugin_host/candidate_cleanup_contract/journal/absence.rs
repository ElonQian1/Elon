use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error};

use super::{
    build_parent_namespace_absence_event, validate_hashed_cleanup_step_event,
    CandidateCleanupParentAbsenceRecoveryKey, HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::ObservedCandidateCleanupParentAbsence,
    local_authority::{
        ComputePluginCandidateCleanupParentAbsenceAuthoritySession,
        ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession,
        ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[must_use = "prepared parent absence must be stored or retain exact parent custody"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupParentAbsence<
    'authority,
> {
    pub(super) observed: ObservedCandidateCleanupParentAbsence,
    pub(super) authority_session:
        ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'authority>,
    pub(super) event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) prepared_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupParentAbsencePermit<
    'permit,
> {
    prepared: &'permit PreparedCandidateCleanupParentAbsence<'permit>,
}

#[must_use = "durable parent absence must proceed to namespace durability or remain retained"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupParentAbsence {
    observed: ObservedCandidateCleanupParentAbsence,
    event: HashedComputePluginCandidateCleanupStepEvent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentAbsencePreparationFailure
{
    error: Error,
    observed: ObservedCandidateCleanupParentAbsence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupParentAbsenceStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain parent absence store retains observed custody until classified"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentAbsenceOutcomeUncertainCustody
{
    observed: ObservedCandidateCleanupParentAbsence,
    recovery_key: CandidateCleanupParentAbsenceRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentAbsenceStoreFailure {
    phase: CandidateCleanupParentAbsenceStorePhase,
    error: Error,
    recovery: CandidateCleanupParentAbsenceOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupParentAbsenceRecoveryAdoption {
    NotCreated(ObservedCandidateCleanupParentAbsence),
    Durable(DurableCandidateCleanupParentAbsence),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupParentAbsenceRecoveryAdoptionPhase
{
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentAbsenceRecoveryAdoptionFailure
{
    phase: CandidateCleanupParentAbsenceRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupParentAbsenceOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_parent_absence<
    'authority,
>(
    observed: ObservedCandidateCleanupParentAbsence,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> Result<
    PreparedCandidateCleanupParentAbsence<'authority>,
    CandidateCleanupParentAbsencePreparationFailure,
> {
    let prepared_at = Instant::now();
    let authority_session = match authority.bind_candidate_cleanup_parent_absence_authority_session(
        process_fence,
        observation,
        &observed,
        prepared_at,
    ) {
        Ok(session) => session,
        Err(error) => return Err(preparation_failure(error, observed)),
    };
    let event = match build_parent_namespace_absence_event(
        observed.plan(),
        observed.intent_event(),
        observed.disposition_event(),
        observed.object_binding(),
        authority_session.trusted_now_ms(),
    ) {
        Ok(event) => event,
        Err(error) => return Err(preparation_failure(error, observed)),
    };
    if let Err(error) =
        authority_session.validate_candidate_cleanup_parent_absence(&observed, &event)
    {
        return Err(preparation_failure(error, observed));
    }
    Ok(PreparedCandidateCleanupParentAbsence {
        observed,
        authority_session,
        event,
        prepared_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_parent_absence(
    prepared: PreparedCandidateCleanupParentAbsence<'_>,
) -> Result<DurableCandidateCleanupParentAbsence, CandidateCleanupParentAbsenceStoreFailure> {
    let recovery_key = CandidateCleanupParentAbsenceRecoveryKey::from_prepared(&prepared);
    if let Err(error) = validate_hashed_cleanup_step_event(&prepared.event) {
        return Err(store_failure(
            CandidateCleanupParentAbsenceStorePhase::PreStorePreparation,
            error,
            prepared.observed,
            recovery_key,
        ));
    }
    let stored = {
        let permit = ValidatedCandidateCleanupParentAbsencePermit {
            prepared: &prepared,
        };
        prepared
            .authority_session
            .persist_candidate_cleanup_parent_absence(permit)
    };
    let event = match stored {
        Ok(event) => event,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupParentAbsenceStorePhase::StoreOutcomeUncertain,
                error,
                prepared.observed,
                recovery_key,
            ))
        }
    };
    if event != prepared.event {
        return Err(store_failure(
            CandidateCleanupParentAbsenceStorePhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_POSTCONDITION_CHANGED"
            ),
            prepared.observed,
            recovery_key,
        ));
    }
    Ok(DurableCandidateCleanupParentAbsence {
        observed: prepared.observed,
        event,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_parent_absence(
    recovery: CandidateCleanupParentAbsenceOutcomeUncertainCustody,
    observed_outcome: &ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome,
    session: ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
) -> Result<
    CandidateCleanupParentAbsenceRecoveryAdoption,
    CandidateCleanupParentAbsenceRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &session) {
        return Err(adoption_failure(
            CandidateCleanupParentAbsenceRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match session.read_candidate_cleanup_parent_absence_outcome(&recovery.recovery_key)
    {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateCleanupParentAbsenceRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed_outcome {
        return Err(adoption_failure(
            CandidateCleanupParentAbsenceRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = session.validate_source(recovery.observed.state().cancellation_guard()) {
        return Err(adoption_failure(
            CandidateCleanupParentAbsenceRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome::NotCreated => Ok(
            CandidateCleanupParentAbsenceRecoveryAdoption::NotCreated(recovery.observed),
        ),
        ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome::Durable(event) => {
            if event != *recovery.recovery_key.absence_event() {
                return Err(adoption_failure(
                    CandidateCleanupParentAbsenceRecoveryAdoptionPhase::OutcomeChanged,
                    anyhow::anyhow!(
                        "COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_EVENT_CHANGED"
                    ),
                    recovery,
                ));
            }
            Ok(CandidateCleanupParentAbsenceRecoveryAdoption::Durable(
                DurableCandidateCleanupParentAbsence {
                    observed: recovery.observed,
                    event,
                },
            ))
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupParentAbsenceOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
) -> anyhow::Result<()> {
    let key = &recovery.recovery_key;
    let observed = &recovery.observed;
    let staging = observed.state().staging_recovery_key();
    let owner_slot = staging.slot_expectation();
    let owner_receipt = staging.receipt_expectation();
    let expected_absence = build_parent_namespace_absence_event(
        observed.plan(),
        observed.intent_event(),
        observed.disposition_event(),
        observed.object_binding(),
        key.absence_event().event().recorded_at_ms(),
    )?;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || key.parent_absence_observed_at() != observed.observed_at()
        || observed.observed_at() <= observed.disposition_set_at()
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
            != observed
                .state()
                .authorization_receipt()
                .receipt()
                .authorized_at_ms()
        || key.authorization_receipt() != observed.state().authorization_receipt()
        || key.plan() != observed.plan()
        || key.intent_event() != observed.intent_event()
        || key.disposition_event() != observed.disposition_event()
        || key.absence_event() != &expected_absence
        || session.observed_at() <= key.prepared_at()
        || session.observed_at() <= key.parent_absence_observed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(recovery.observed.state().cancellation_guard())
}

fn preparation_failure(
    error: Error,
    observed: ObservedCandidateCleanupParentAbsence,
) -> CandidateCleanupParentAbsencePreparationFailure {
    CandidateCleanupParentAbsencePreparationFailure { error, observed }
}

fn store_failure(
    phase: CandidateCleanupParentAbsenceStorePhase,
    error: Error,
    observed: ObservedCandidateCleanupParentAbsence,
    recovery_key: CandidateCleanupParentAbsenceRecoveryKey,
) -> CandidateCleanupParentAbsenceStoreFailure {
    CandidateCleanupParentAbsenceStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupParentAbsenceOutcomeUncertainCustody {
            observed,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupParentAbsenceRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupParentAbsenceOutcomeUncertainCustody,
) -> CandidateCleanupParentAbsenceRecoveryAdoptionFailure {
    CandidateCleanupParentAbsenceRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl ValidatedCandidateCleanupParentAbsencePermit<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn observed(
        &self,
    ) -> &ObservedCandidateCleanupParentAbsence {
        &self.prepared.observed
    }
    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.prepared.event
    }
}

impl DurableCandidateCleanupParentAbsence {
    pub(in crate::node_agent_compute_plugin_host) fn observed(
        &self,
    ) -> &ObservedCandidateCleanupParentAbsence {
        &self.observed
    }
    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.event
    }
    pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn into_parts(
        self,
    ) -> (
        ObservedCandidateCleanupParentAbsence,
        HashedComputePluginCandidateCleanupStepEvent,
    ) {
        (self.observed, self.event)
    }
}

impl CandidateCleanupParentAbsencePreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, ObservedCandidateCleanupParentAbsence) {
        (self.error, self.observed)
    }
}

impl CandidateCleanupParentAbsenceStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupParentAbsenceStorePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupParentAbsenceOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupParentAbsenceOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupParentAbsenceRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupParentAbsenceRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupParentAbsenceRecoveryAdoptionPhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupParentAbsenceOutcomeUncertainCustody) {
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

impl_failure!(CandidateCleanupParentAbsencePreparationFailure);
impl_failure!(CandidateCleanupParentAbsenceStoreFailure);
impl_failure!(CandidateCleanupParentAbsenceRecoveryAdoptionFailure);
