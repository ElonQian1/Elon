use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::CandidateCleanupExecutionState;
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    build_exact_handle_disposition_event, build_initial_delete_intent,
    build_parent_namespace_absence_event, validate_hashed_cleanup_step_event,
    validate_hashed_execution_plan, DurableCandidateCleanupParentAbsence,
    HashedComputePluginCandidateCleanupExecutionPlan, HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_managed_fs::{
    ManagedNamespaceDurabilityFailureCustody, ManagedNamespaceDurabilityFailurePhase,
    ManagedNamespaceDurabilityRetainedCustody, ManagedNamespaceDurable,
    ManagedNamespaceMutationFence, ManagedNamespacePostBarrierObservationRetry,
    ManagedNamespacePreBarrierRetry, ManagedObjectBinding,
};

/// The exact parent namespace passed a real native barrier and post-barrier absence proof. This is
/// physical custody only; sequence 4 must still be independently committed by its typed Store.
/// The route remains contract-private until an OS-enforced child-namespace ABA fence supplements
/// the retained process/root/owner authority fences.
#[must_use = "physical namespace durability must be journaled or retained for recovery"]
pub(in crate::node_agent_compute_plugin_host) struct PhysicallyDurableCandidateCleanupNamespace {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence_event: HashedComputePluginCandidateCleanupStepEvent,
    namespace: ManagedNamespaceDurable,
    disposition_set_at: Instant,
    parent_absence_observed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupNamespaceDurabilityFailurePhase {
    RejectedBeforeDurability,
    Managed(ManagedNamespaceDurabilityFailurePhase),
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupNamespaceDurabilityFailureCustody
{
    RejectedParentAbsence {
        _durable: DurableCandidateCleanupParentAbsence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    RetryBeforeBarrier(CandidateCleanupNamespaceDurabilityRetryCustody),
    RetryAfterBarrier(CandidateCleanupNamespacePostBarrierRetryCustody),
    Retained(CandidateCleanupNamespaceDurabilityRetainedCustody),
}

#[must_use = "pre-barrier retry must retain the exact durable absence custody"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityRetryCustody
{
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence_event: HashedComputePluginCandidateCleanupStepEvent,
    retry: ManagedNamespacePreBarrierRetry,
    disposition_set_at: Instant,
    parent_absence_observed_at: Instant,
}

#[must_use = "post-barrier retry must not repeat the native namespace barrier"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespacePostBarrierRetryCustody
{
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence_event: HashedComputePluginCandidateCleanupStepEvent,
    retry: ManagedNamespacePostBarrierObservationRetry,
    disposition_set_at: Instant,
    parent_absence_observed_at: Instant,
}

#[must_use = "terminal durability failure must retain every namespace handle"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityRetainedCustody
{
    _state: CandidateCleanupExecutionState,
    _plan: HashedComputePluginCandidateCleanupExecutionPlan,
    _intent_event: HashedComputePluginCandidateCleanupStepEvent,
    _disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    _absence_event: HashedComputePluginCandidateCleanupStepEvent,
    _retained: ManagedNamespaceDurabilityRetainedCustody,
    _disposition_set_at: Instant,
    _parent_absence_observed_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityFailure {
    phase: CandidateCleanupNamespaceDurabilityFailurePhase,
    error: Error,
    custody: CandidateCleanupNamespaceDurabilityFailureCustody,
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn make_candidate_cleanup_namespace_durable(
    durable: DurableCandidateCleanupParentAbsence,
    mutation_fence: ManagedNamespaceMutationFence,
) -> std::result::Result<
    PhysicallyDurableCandidateCleanupNamespace,
    CandidateCleanupNamespaceDurabilityFailure,
> {
    if let Err(error) = validate_durable_parent_absence(&durable) {
        return Err(namespace_failure(
            CandidateCleanupNamespaceDurabilityFailurePhase::RejectedBeforeDurability,
            error,
            CandidateCleanupNamespaceDurabilityFailureCustody::RejectedParentAbsence {
                _durable: durable,
                _mutation_fence: mutation_fence,
            },
        ));
    }
    let (observed, absence_event) = durable.into_parts();
    let (
        state,
        plan,
        intent_event,
        disposition_event,
        absence,
        disposition_set_at,
        parent_absence_observed_at,
    ) = observed.into_parts();
    let result = absence.make_namespace_durable(
        mutation_fence,
        plan.plan().cleanup_id(),
        plan.plan_digest(),
        state.authorization_receipt().receipt_digest(),
        intent_event.event().object_digest(),
        plan.plan().installation_id_digest(),
        state
            .authorization_receipt()
            .receipt()
            .authority_epoch_after(),
        plan.plan().process_owner_epoch(),
        u64::try_from(intent_event.event().step_ordinal())
            .expect("validated cleanup step ordinal is non-negative"),
        parent_absence_observed_at,
    );
    finish_managed_result(
        result,
        NamespaceDurabilityFacts {
            state,
            plan,
            intent_event,
            disposition_event,
            absence_event,
            disposition_set_at,
            parent_absence_observed_at,
        },
    )
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn retry_candidate_cleanup_namespace_durability(
    retry: CandidateCleanupNamespaceDurabilityRetryCustody,
) -> std::result::Result<
    PhysicallyDurableCandidateCleanupNamespace,
    CandidateCleanupNamespaceDurabilityFailure,
> {
    if let Err(error) = validate_retry_binding(
        &retry.state,
        &retry.plan,
        &retry.intent_event,
        &retry.disposition_event,
        &retry.absence_event,
        retry.retry.object_binding(),
    ) {
        return Err(namespace_failure(
            CandidateCleanupNamespaceDurabilityFailurePhase::RejectedBeforeDurability,
            error,
            CandidateCleanupNamespaceDurabilityFailureCustody::RetryBeforeBarrier(retry),
        ));
    }
    let CandidateCleanupNamespaceDurabilityRetryCustody {
        state,
        plan,
        intent_event,
        disposition_event,
        absence_event,
        retry: managed_retry,
        disposition_set_at,
        parent_absence_observed_at,
    } = retry;
    let result = managed_retry.retry_pre_barrier(parent_absence_observed_at);
    finish_managed_result(
        result,
        NamespaceDurabilityFacts {
            state,
            plan,
            intent_event,
            disposition_event,
            absence_event,
            disposition_set_at,
            parent_absence_observed_at,
        },
    )
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn retry_candidate_cleanup_namespace_post_barrier_observation(
    retry: CandidateCleanupNamespacePostBarrierRetryCustody,
) -> std::result::Result<
    PhysicallyDurableCandidateCleanupNamespace,
    CandidateCleanupNamespaceDurabilityFailure,
> {
    if let Err(error) = validate_retry_binding(
        &retry.state,
        &retry.plan,
        &retry.intent_event,
        &retry.disposition_event,
        &retry.absence_event,
        retry.retry.object_binding(),
    ) {
        return Err(namespace_failure(
            CandidateCleanupNamespaceDurabilityFailurePhase::RejectedBeforeDurability,
            error,
            CandidateCleanupNamespaceDurabilityFailureCustody::RetryAfterBarrier(retry),
        ));
    }
    let CandidateCleanupNamespacePostBarrierRetryCustody {
        state,
        plan,
        intent_event,
        disposition_event,
        absence_event,
        retry: managed_retry,
        disposition_set_at,
        parent_absence_observed_at,
    } = retry;
    let result = managed_retry.retry_post_barrier_observation();
    finish_managed_result(
        result,
        NamespaceDurabilityFacts {
            state,
            plan,
            intent_event,
            disposition_event,
            absence_event,
            disposition_set_at,
            parent_absence_observed_at,
        },
    )
}

fn finish_managed_result(
    result: std::result::Result<
        ManagedNamespaceDurable,
        crate::node_agent_managed_fs::ManagedNamespaceDurabilityFailure,
    >,
    facts: NamespaceDurabilityFacts,
) -> std::result::Result<
    PhysicallyDurableCandidateCleanupNamespace,
    CandidateCleanupNamespaceDurabilityFailure,
> {
    match result {
        Ok(namespace) => Ok(PhysicallyDurableCandidateCleanupNamespace {
            state: facts.state,
            plan: facts.plan,
            intent_event: facts.intent_event,
            disposition_event: facts.disposition_event,
            absence_event: facts.absence_event,
            namespace,
            disposition_set_at: facts.disposition_set_at,
            parent_absence_observed_at: facts.parent_absence_observed_at,
        }),
        Err(failure) => Err(map_managed_failure(failure, facts)),
    }
}

struct NamespaceDurabilityFacts {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_set_at: Instant,
    parent_absence_observed_at: Instant,
}

fn map_managed_failure(
    failure: crate::node_agent_managed_fs::ManagedNamespaceDurabilityFailure,
    facts: NamespaceDurabilityFacts,
) -> CandidateCleanupNamespaceDurabilityFailure {
    let phase = CandidateCleanupNamespaceDurabilityFailurePhase::Managed(failure.phase());
    let (error, custody) = failure.into_parts();
    let custody = match custody {
        ManagedNamespaceDurabilityFailureCustody::RetryBeforeBarrier(absence) => {
            CandidateCleanupNamespaceDurabilityFailureCustody::RetryBeforeBarrier(
                CandidateCleanupNamespaceDurabilityRetryCustody {
                    state: facts.state,
                    plan: facts.plan,
                    intent_event: facts.intent_event,
                    disposition_event: facts.disposition_event,
                    absence_event: facts.absence_event,
                    retry: absence,
                    disposition_set_at: facts.disposition_set_at,
                    parent_absence_observed_at: facts.parent_absence_observed_at,
                },
            )
        }
        ManagedNamespaceDurabilityFailureCustody::RetryAfterBarrier(retry) => {
            CandidateCleanupNamespaceDurabilityFailureCustody::RetryAfterBarrier(
                CandidateCleanupNamespacePostBarrierRetryCustody {
                    state: facts.state,
                    plan: facts.plan,
                    intent_event: facts.intent_event,
                    disposition_event: facts.disposition_event,
                    absence_event: facts.absence_event,
                    retry,
                    disposition_set_at: facts.disposition_set_at,
                    parent_absence_observed_at: facts.parent_absence_observed_at,
                },
            )
        }
        ManagedNamespaceDurabilityFailureCustody::Retained(retained) => {
            CandidateCleanupNamespaceDurabilityFailureCustody::Retained(
                CandidateCleanupNamespaceDurabilityRetainedCustody {
                    _state: facts.state,
                    _plan: facts.plan,
                    _intent_event: facts.intent_event,
                    _disposition_event: facts.disposition_event,
                    _absence_event: facts.absence_event,
                    _retained: retained,
                    _disposition_set_at: facts.disposition_set_at,
                    _parent_absence_observed_at: facts.parent_absence_observed_at,
                },
            )
        }
    };
    namespace_failure(phase, Error::new(error), custody)
}

fn validate_durable_parent_absence(durable: &DurableCandidateCleanupParentAbsence) -> Result<()> {
    let observed = durable.observed();
    validate_binding(
        observed.state(),
        observed.plan(),
        observed.intent_event(),
        observed.disposition_event(),
        durable.event(),
        observed.object_binding(),
    )
}

fn validate_retry_binding(
    state: &CandidateCleanupExecutionState,
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    disposition: &HashedComputePluginCandidateCleanupStepEvent,
    absence: &HashedComputePluginCandidateCleanupStepEvent,
    binding: &ManagedObjectBinding,
) -> Result<()> {
    validate_binding(state, plan, intent, disposition, absence, binding)
}

fn validate_binding(
    state: &CandidateCleanupExecutionState,
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    disposition: &HashedComputePluginCandidateCleanupStepEvent,
    absence: &HashedComputePluginCandidateCleanupStepEvent,
    binding: &ManagedObjectBinding,
) -> Result<()> {
    validate_hashed_execution_plan(plan)?;
    validate_hashed_cleanup_step_event(intent)?;
    validate_hashed_cleanup_step_event(disposition)?;
    validate_hashed_cleanup_step_event(absence)?;
    let expected_intent = build_initial_delete_intent(plan, intent.event().recorded_at_ms())?;
    let expected_disposition = build_exact_handle_disposition_event(
        plan,
        intent,
        binding,
        disposition.event().recorded_at_ms(),
    )?;
    let expected_absence = build_parent_namespace_absence_event(
        plan,
        intent,
        disposition,
        binding,
        absence.event().recorded_at_ms(),
    )?;
    if expected_intent != *intent
        || expected_disposition != *disposition
        || expected_absence != *absence
        || state.completed_step_count() != 0
        || state.execution_plan_digest() != Some(plan.plan_digest())
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DURABILITY_BINDING_CHANGED");
    }
    state.cancellation_guard().ensure_current()
}

fn namespace_failure(
    phase: CandidateCleanupNamespaceDurabilityFailurePhase,
    error: Error,
    custody: CandidateCleanupNamespaceDurabilityFailureCustody,
) -> CandidateCleanupNamespaceDurabilityFailure {
    CandidateCleanupNamespaceDurabilityFailure {
        phase,
        error,
        custody,
    }
}

impl PhysicallyDurableCandidateCleanupNamespace {
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
    pub(in crate::node_agent_compute_plugin_host) fn absence_event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.absence_event
    }
    pub(in crate::node_agent_compute_plugin_host) fn namespace(&self) -> &ManagedNamespaceDurable {
        &self.namespace
    }
    pub(in crate::node_agent_compute_plugin_host) fn disposition_set_at(&self) -> Instant {
        self.disposition_set_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn parent_absence_observed_at(&self) -> Instant {
        self.parent_absence_observed_at
    }
    pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn into_parts(
        self,
    ) -> (
        CandidateCleanupExecutionState,
        HashedComputePluginCandidateCleanupExecutionPlan,
        HashedComputePluginCandidateCleanupStepEvent,
        HashedComputePluginCandidateCleanupStepEvent,
        HashedComputePluginCandidateCleanupStepEvent,
        ManagedNamespaceDurable,
        Instant,
        Instant,
    ) {
        (
            self.state,
            self.plan,
            self.intent_event,
            self.disposition_event,
            self.absence_event,
            self.namespace,
            self.disposition_set_at,
            self.parent_absence_observed_at,
        )
    }
}

impl CandidateCleanupNamespaceDurabilityFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupNamespaceDurabilityFailurePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupNamespaceDurabilityFailureCustody) {
        (self.error, self.custody)
    }
}

impl fmt::Display for CandidateCleanupNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupNamespaceDurabilityFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateCleanupNamespaceDurabilityFailure {}
