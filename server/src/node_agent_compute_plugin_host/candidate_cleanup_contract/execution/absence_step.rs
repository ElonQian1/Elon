use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::CandidateCleanupExecutionState;
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    build_exact_handle_disposition_event, build_initial_delete_intent,
    validate_hashed_cleanup_step_event, validate_hashed_execution_plan,
    DurableCandidateCleanupDisposition, HashedComputePluginCandidateCleanupExecutionPlan,
    HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_managed_fs::{
    ManagedDeleteDisposition, ManagedExpectedIdentityMatchPresence, ManagedObjectBinding,
    ManagedParentRelativeAbsence, ManagedParentRelativeIdentityConflict,
    ManagedParentRelativeObservation, QuarantinedManagedNamespaceObject,
};

#[must_use = "observed absence must be journaled or retain exact parent custody"]
pub(in crate::node_agent_compute_plugin_host) struct ObservedCandidateCleanupParentAbsence {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence: ManagedParentRelativeAbsence,
    disposition_set_at: Instant,
    observed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupParentObservationFailurePhase {
    RejectedBeforeObservation,
    ObservationFailed,
    ObservationQuarantined,
    ExpectedIdentityStillPresent,
    IdentityConflict,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupParentObservationFailureCustody {
    Durable(DurableCandidateCleanupDisposition),
    Retry(CandidateCleanupParentObservationRetryCustody),
    Quarantined(CandidateCleanupParentObservationQuarantinedCustody),
    ExpectedIdentityMatch(CandidateCleanupExpectedIdentityMatchCustody),
    IdentityConflict(CandidateCleanupParentRelativeIdentityConflictCustody),
}

#[must_use = "failed parent observation must retry with the same disposition custody"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentObservationRetryCustody {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition: ManagedDeleteDisposition,
    disposition_set_at: Instant,
}

#[must_use = "failed same-name inspection must retain the opened object and exact disposition"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentObservationQuarantinedCustody
{
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition: ManagedDeleteDisposition,
    _observed_object: QuarantinedManagedNamespaceObject,
    disposition_set_at: Instant,
}

#[must_use = "same-name expected identity must remain retained for operator recovery"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupExpectedIdentityMatchCustody {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    presence: ManagedExpectedIdentityMatchPresence,
    disposition_set_at: Instant,
    observed_at: Instant,
}

#[must_use = "same-name identity conflict must remain retained for operator recovery"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentRelativeIdentityConflictCustody
{
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    conflict: ManagedParentRelativeIdentityConflict,
    disposition_set_at: Instant,
    observed_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentObservationFailure {
    phase: CandidateCleanupParentObservationFailurePhase,
    error: Error,
    custody: CandidateCleanupParentObservationFailureCustody,
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn observe_candidate_cleanup_parent_namespace(
    durable: DurableCandidateCleanupDisposition,
) -> std::result::Result<
    ObservedCandidateCleanupParentAbsence,
    CandidateCleanupParentObservationFailure,
> {
    if let Err(error) = validate_durable_disposition(&durable) {
        return Err(observation_failure(
            CandidateCleanupParentObservationFailurePhase::RejectedBeforeObservation,
            error,
            CandidateCleanupParentObservationFailureCustody::Durable(durable),
        ));
    }
    let (physical, disposition_event) = durable.into_parts();
    let (state, plan, intent_event, disposition, disposition_set_at) = physical.into_parts();
    execute_observation(CandidateCleanupParentObservationRetryCustody {
        state,
        plan,
        intent_event,
        disposition_event,
        disposition,
        disposition_set_at,
    })
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn retry_candidate_cleanup_parent_observation(
    retry: CandidateCleanupParentObservationRetryCustody,
) -> std::result::Result<
    ObservedCandidateCleanupParentAbsence,
    CandidateCleanupParentObservationFailure,
> {
    execute_observation(retry)
}

fn execute_observation(
    retry: CandidateCleanupParentObservationRetryCustody,
) -> std::result::Result<
    ObservedCandidateCleanupParentAbsence,
    CandidateCleanupParentObservationFailure,
> {
    if let Err(error) = validate_retry_custody(&retry) {
        return Err(observation_failure(
            CandidateCleanupParentObservationFailurePhase::RejectedBeforeObservation,
            error,
            CandidateCleanupParentObservationFailureCustody::Retry(retry),
        ));
    }
    let _operation = match retry.state.deletion_guard().enter_operation() {
        Ok(operation) => operation,
        Err(error) => {
            return Err(observation_failure(
                CandidateCleanupParentObservationFailurePhase::RejectedBeforeObservation,
                error,
                CandidateCleanupParentObservationFailureCustody::Retry(retry),
            ))
        }
    };
    let CandidateCleanupParentObservationRetryCustody {
        state,
        plan,
        intent_event,
        disposition_event,
        disposition,
        disposition_set_at,
    } = retry;
    match disposition.observe_parent_relative() {
        Ok(ManagedParentRelativeObservation::Absent(absence)) => {
            Ok(ObservedCandidateCleanupParentAbsence {
                state,
                plan,
                intent_event,
                disposition_event,
                absence,
                disposition_set_at,
                observed_at: Instant::now(),
            })
        }
        Ok(ManagedParentRelativeObservation::ExpectedIdentityMatch(presence)) => {
            Err(observation_failure(
                CandidateCleanupParentObservationFailurePhase::ExpectedIdentityStillPresent,
                anyhow::anyhow!(
                    "COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_EXPECTED_IDENTITY_PRESENT"
                ),
                CandidateCleanupParentObservationFailureCustody::ExpectedIdentityMatch(
                    CandidateCleanupExpectedIdentityMatchCustody {
                        state,
                        plan,
                        intent_event,
                        disposition_event,
                        presence,
                        disposition_set_at,
                        observed_at: Instant::now(),
                    },
                ),
            ))
        }
        Ok(ManagedParentRelativeObservation::IdentityConflict(conflict)) => {
            Err(observation_failure(
                CandidateCleanupParentObservationFailurePhase::IdentityConflict,
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_IDENTITY_CONFLICT"),
                CandidateCleanupParentObservationFailureCustody::IdentityConflict(
                    CandidateCleanupParentRelativeIdentityConflictCustody {
                        state,
                        plan,
                        intent_event,
                        disposition_event,
                        conflict,
                        disposition_set_at,
                        observed_at: Instant::now(),
                    },
                ),
            ))
        }
        Err(failure) => {
            let (error, disposition, quarantined_observed_object) = failure.into_parts();
            let error = Error::new(error);
            let (phase, custody) = match quarantined_observed_object {
                Some(observed_object) => (
                    CandidateCleanupParentObservationFailurePhase::ObservationQuarantined,
                    CandidateCleanupParentObservationFailureCustody::Quarantined(
                        CandidateCleanupParentObservationQuarantinedCustody {
                            state,
                            plan,
                            intent_event,
                            disposition_event,
                            disposition,
                            _observed_object: observed_object,
                            disposition_set_at,
                        },
                    ),
                ),
                None => (
                    CandidateCleanupParentObservationFailurePhase::ObservationFailed,
                    CandidateCleanupParentObservationFailureCustody::Retry(
                        CandidateCleanupParentObservationRetryCustody {
                            state,
                            plan,
                            intent_event,
                            disposition_event,
                            disposition,
                            disposition_set_at,
                        },
                    ),
                ),
            };
            Err(observation_failure(phase, error, custody))
        }
    }
}

fn validate_durable_disposition(durable: &DurableCandidateCleanupDisposition) -> Result<()> {
    validate_binding(
        durable.physical().state(),
        durable.physical().plan(),
        durable.physical().intent_event(),
        durable.event(),
        durable.physical().object_binding(),
    )
}

fn validate_retry_custody(retry: &CandidateCleanupParentObservationRetryCustody) -> Result<()> {
    validate_binding(
        &retry.state,
        &retry.plan,
        &retry.intent_event,
        &retry.disposition_event,
        retry.disposition.object_binding(),
    )
}

fn validate_binding(
    state: &CandidateCleanupExecutionState,
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    disposition: &HashedComputePluginCandidateCleanupStepEvent,
    binding: &ManagedObjectBinding,
) -> Result<()> {
    validate_hashed_execution_plan(plan)?;
    validate_hashed_cleanup_step_event(intent)?;
    validate_hashed_cleanup_step_event(disposition)?;
    let expected_intent = build_initial_delete_intent(plan, intent.event().recorded_at_ms())?;
    let expected_disposition = build_exact_handle_disposition_event(
        plan,
        intent,
        binding,
        disposition.event().recorded_at_ms(),
    )?;
    if expected_intent != *intent
        || expected_disposition != *disposition
        || state.completed_step_count() != 0
        || state.execution_plan_digest() != Some(plan.plan_digest())
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_OBSERVATION_BINDING_CHANGED");
    }
    state.deletion_guard().ensure_current()
}

fn observation_failure(
    phase: CandidateCleanupParentObservationFailurePhase,
    error: Error,
    custody: CandidateCleanupParentObservationFailureCustody,
) -> CandidateCleanupParentObservationFailure {
    CandidateCleanupParentObservationFailure {
        phase,
        error,
        custody,
    }
}

impl ObservedCandidateCleanupParentAbsence {
    pub(in crate::node_agent_compute_plugin_host) fn state(
        &self,
    ) -> &CandidateCleanupExecutionState {
        &self.state
    }
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionPlan {
        &self.plan
    }
    pub(in crate::node_agent_compute_plugin_host) fn intent_event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.intent_event
    }
    pub(in crate::node_agent_compute_plugin_host) fn disposition_event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.disposition_event
    }
    pub(in crate::node_agent_compute_plugin_host) fn object_binding(
        &self,
    ) -> &ManagedObjectBinding {
        self.absence.object_binding()
    }
    pub(in crate::node_agent_compute_plugin_host) fn disposition_set_at(&self) -> Instant {
        self.disposition_set_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }
    pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn into_parts(
        self,
    ) -> (
        CandidateCleanupExecutionState,
        HashedComputePluginCandidateCleanupExecutionPlan,
        HashedComputePluginCandidateCleanupStepEvent,
        HashedComputePluginCandidateCleanupStepEvent,
        ManagedParentRelativeAbsence,
        Instant,
        Instant,
    ) {
        (
            self.state,
            self.plan,
            self.intent_event,
            self.disposition_event,
            self.absence,
            self.disposition_set_at,
            self.observed_at,
        )
    }
}

impl CandidateCleanupParentObservationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupParentObservationFailurePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupParentObservationFailureCustody) {
        (self.error, self.custody)
    }
}

impl fmt::Display for CandidateCleanupParentObservationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupParentObservationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupParentObservationFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateCleanupParentObservationFailure {}
