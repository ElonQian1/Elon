use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;

use super::CandidateCleanupExecutionState;
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    DurableCandidateCleanupParentAbsence, HashedComputePluginCandidateCleanupExecutionPlan,
    HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_managed_fs::{
    ManagedNamespaceDurabilityFailurePhase, ManagedNamespaceDurabilityRetainedCustody,
    ManagedNamespaceDurable, ManagedNamespaceMutationFence,
    ManagedNamespacePostBarrierObservationRetry, ManagedNamespacePreBarrierRetry,
};

/// The exact parent namespace passed a real native barrier and post-barrier absence proof. This is
/// physical custody only; sequence 4 must still be independently committed by its typed Store.
/// The route remains contract-private until an OS-enforced child-namespace ABA fence supplements
/// the retained process/root/owner authority fences.
#[must_use = "physical namespace durability must be journaled or retained for recovery"]
pub(in crate::node_agent_compute_plugin_host) struct PhysicallyDurableCandidateCleanupNamespace {
    pub(super) state: CandidateCleanupExecutionState,
    pub(super) plan: HashedComputePluginCandidateCleanupExecutionPlan,
    pub(super) intent_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) absence_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) namespace: ManagedNamespaceDurable,
    pub(super) disposition_set_at: Instant,
    pub(super) parent_absence_observed_at: Instant,
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
    pub(super) state: CandidateCleanupExecutionState,
    pub(super) plan: HashedComputePluginCandidateCleanupExecutionPlan,
    pub(super) intent_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) absence_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) retry: ManagedNamespacePreBarrierRetry,
    pub(super) disposition_set_at: Instant,
    pub(super) parent_absence_observed_at: Instant,
}

#[must_use = "post-barrier retry must not repeat the native namespace barrier"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespacePostBarrierRetryCustody
{
    pub(super) state: CandidateCleanupExecutionState,
    pub(super) plan: HashedComputePluginCandidateCleanupExecutionPlan,
    pub(super) intent_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) absence_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) retry: ManagedNamespacePostBarrierObservationRetry,
    pub(super) disposition_set_at: Instant,
    pub(super) parent_absence_observed_at: Instant,
}

#[must_use = "terminal durability failure must retain every namespace handle"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityRetainedCustody
{
    pub(super) _state: CandidateCleanupExecutionState,
    pub(super) _plan: HashedComputePluginCandidateCleanupExecutionPlan,
    pub(super) _intent_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) _disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) _absence_event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) _retained: ManagedNamespaceDurabilityRetainedCustody,
    pub(super) _disposition_set_at: Instant,
    pub(super) _parent_absence_observed_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityFailure {
    pub(super) phase: CandidateCleanupNamespaceDurabilityFailurePhase,
    pub(super) error: Error,
    pub(super) custody: CandidateCleanupNamespaceDurabilityFailureCustody,
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
