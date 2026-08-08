use std::{fmt, time::Instant};

use super::{HashedComputePluginCandidateCleanupStepEvent, PreparedCandidateCleanupParentAbsence};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::HashedComputePluginCandidateCleanupExecutionPlan,
    identity::ComputePluginReleaseRef,
    local_authority::{
        ComputePluginAuthorityInstanceBinding,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
    },
};

/// Process-local classifier for one uncertain parent-absence event commit. It cannot repeat the
/// physical disposition or the already completed parent-relative observation.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupParentAbsenceRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    prepared_at: Instant,
    parent_absence_observed_at: Instant,
    candidate_token: String,
    owner_plugin_id: String,
    owner_slot_ref: String,
    owner_release: ComputePluginReleaseRef,
    owner_candidate_generation: i64,
    owner_plan_id: String,
    owner_plan_digest: String,
    owner_application_inventory_revision: i64,
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition_event: HashedComputePluginCandidateCleanupStepEvent,
    absence_event: HashedComputePluginCandidateCleanupStepEvent,
}

impl CandidateCleanupParentAbsenceRecoveryKey {
    pub(super) fn from_prepared(prepared: &PreparedCandidateCleanupParentAbsence<'_>) -> Self {
        let observed = &prepared.observed;
        let staging = observed.state().staging_recovery_key();
        let slot = staging.slot_expectation();
        let receipt = staging.receipt_expectation();
        Self {
            authority_instance_binding: prepared
                .authority_session
                .authority_instance_binding()
                .clone(),
            installation_id_digest: prepared
                .authority_session
                .installation_id_digest()
                .to_string(),
            clock_epoch_digest: prepared.authority_session.clock_epoch_digest().to_string(),
            prepared_at: prepared.prepared_at,
            parent_absence_observed_at: observed.observed_at(),
            candidate_token: staging.candidate_token().to_string(),
            owner_plugin_id: slot.plugin_id.clone(),
            owner_slot_ref: slot.slot_ref.clone(),
            owner_release: slot.release.clone(),
            owner_candidate_generation: receipt.candidate_generation,
            owner_plan_id: receipt.owner_plan_id.clone(),
            owner_plan_digest: receipt.owner_plan_digest.clone(),
            owner_application_inventory_revision: receipt.application_inventory_revision,
            authorization_receipt: observed.state().authorization_receipt().clone(),
            plan: observed.plan().clone(),
            intent_event: observed.intent_event().clone(),
            disposition_event: observed.disposition_event().clone(),
            absence_event: prepared.event.clone(),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn prepared_at(&self) -> Instant {
        self.prepared_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn parent_absence_observed_at(&self) -> Instant {
        self.parent_absence_observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_plugin_id(&self) -> &str {
        &self.owner_plugin_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_slot_ref(&self) -> &str {
        &self.owner_slot_ref
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_release(
        &self,
    ) -> &ComputePluginReleaseRef {
        &self.owner_release
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_candidate_generation(&self) -> i64 {
        self.owner_candidate_generation
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_id(&self) -> &str {
        &self.owner_plan_id
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_digest(&self) -> &str {
        &self.owner_plan_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn owner_application_inventory_revision(
        &self,
    ) -> i64 {
        self.owner_application_inventory_revision
    }
    pub(in crate::node_agent_compute_plugin_host) fn authorized_at_ms(&self) -> i64 {
        self.authorization_receipt.receipt().authorized_at_ms()
    }
    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupAuthorizationReceipt {
        &self.authorization_receipt
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
}

impl fmt::Debug for CandidateCleanupParentAbsenceRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupParentAbsenceRecoveryKey")
            .field("cleanup_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("absence_event_digest", &self.absence_event.event_digest())
            .finish()
    }
}
