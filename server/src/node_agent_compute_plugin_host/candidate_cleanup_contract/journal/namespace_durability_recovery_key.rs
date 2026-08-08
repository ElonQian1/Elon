use std::{fmt, time::Instant};

use super::{
    HashedComputePluginCandidateCleanupStepEvent, PreparedCandidateCleanupNamespaceDurability,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::HashedComputePluginCandidateCleanupExecutionPlan,
    identity::ComputePluginReleaseRef,
    local_authority::{
        ComputePluginAuthorityInstanceBinding,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
    },
};

/// Process-local classifier for one uncertain sequence-4 commit. It owns no physical authority;
/// the accompanying uncertain custody retains the already completed native barrier capability.
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupNamespaceDurabilityRecoveryKey
{
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    prepared_at: Instant,
    disposition_set_at: Instant,
    parent_absence_observed_at: Instant,
    barrier_completed_at: Instant,
    post_absence_observed_at: Instant,
    namespace_completed_at: Instant,
    namespace_durability_kind: String,
    filesystem_kind: String,
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
    namespace_event: HashedComputePluginCandidateCleanupStepEvent,
}

impl CandidateCleanupNamespaceDurabilityRecoveryKey {
    pub(super) fn from_prepared(
        prepared: &PreparedCandidateCleanupNamespaceDurability<'_>,
    ) -> Self {
        let physical = &prepared.physical;
        let staging = physical.state().staging_recovery_key();
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
            disposition_set_at: physical.disposition_set_at(),
            parent_absence_observed_at: physical.parent_absence_observed_at(),
            barrier_completed_at: physical.namespace().barrier_completed_at(),
            post_absence_observed_at: physical.namespace().post_absence_observed_at(),
            namespace_completed_at: physical.namespace().completed_at(),
            namespace_durability_kind: physical.namespace().namespace_durability_kind().to_string(),
            filesystem_kind: physical.namespace().filesystem_kind().to_string(),
            candidate_token: staging.candidate_token().to_string(),
            owner_plugin_id: slot.plugin_id.clone(),
            owner_slot_ref: slot.slot_ref.clone(),
            owner_release: slot.release.clone(),
            owner_candidate_generation: receipt.candidate_generation,
            owner_plan_id: receipt.owner_plan_id.clone(),
            owner_plan_digest: receipt.owner_plan_digest.clone(),
            owner_application_inventory_revision: receipt.application_inventory_revision,
            authorization_receipt: physical.state().authorization_receipt().clone(),
            plan: physical.plan().clone(),
            intent_event: physical.intent_event().clone(),
            disposition_event: physical.disposition_event().clone(),
            absence_event: physical.absence_event().clone(),
            namespace_event: prepared.event.clone(),
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
    pub(in crate::node_agent_compute_plugin_host) fn disposition_set_at(&self) -> Instant {
        self.disposition_set_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn parent_absence_observed_at(&self) -> Instant {
        self.parent_absence_observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn barrier_completed_at(&self) -> Instant {
        self.barrier_completed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn post_absence_observed_at(&self) -> Instant {
        self.post_absence_observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn namespace_completed_at(&self) -> Instant {
        self.namespace_completed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn namespace_durability_kind(&self) -> &str {
        &self.namespace_durability_kind
    }
    pub(in crate::node_agent_compute_plugin_host) fn filesystem_kind(&self) -> &str {
        &self.filesystem_kind
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
    pub(in crate::node_agent_compute_plugin_host) fn namespace_event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.namespace_event
    }
}

impl fmt::Debug for CandidateCleanupNamespaceDurabilityRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupNamespaceDurabilityRecoveryKey")
            .field("cleanup_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field(
                "namespace_event_digest",
                &self.namespace_event.event_digest(),
            )
            .finish()
    }
}
