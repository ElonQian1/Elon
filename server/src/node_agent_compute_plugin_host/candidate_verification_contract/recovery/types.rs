use std::fmt;

use crate::node_agent_compute_plugin_host::local_authority::ComputePluginAuthorityInstanceBinding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateVerificationOutcomeKind {
    NotCreated,
    Prepared,
    Aborted,
    Revoked,
}

#[derive(PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationInitialAbsence
{
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) authority_state_revision:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) inventory_revision:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) inventory_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) trusted_time_high_water_ms:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) next_verification_generation:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) durable_candidate_closure_digest:
        String,
}

impl ComputePluginCandidateVerificationInitialAbsence {
    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision(&self) -> i64 {
        self.authority_state_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_revision(&self) -> i64 {
        self.inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest(&self) -> &str {
        &self.inventory_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time_high_water_ms(&self) -> i64 {
        self.trusted_time_high_water_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn next_verification_generation(&self) -> i64 {
        self.next_verification_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn durable_candidate_closure_digest(
        &self,
    ) -> &str {
        &self.durable_candidate_closure_digest
    }
}

/// Process-local, non-cloneable and non-serializable probe for one possibly committed begin.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationRecoveryKey {
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) authority_instance_binding:
        ComputePluginAuthorityInstanceBinding,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) installation_id_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) clock_epoch_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) root_identity_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) verification_id:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) candidate_token:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) candidate_token_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) owner_plan_id:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) owner_plan_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) verification_generation:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) candidate_generation:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) application_inventory_revision:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) authority_state_revision:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) authority_epoch:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) process_owner_epoch:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) execution_inventory_revision:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) inventory_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) artifact_count:
        usize,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) artifact_bytes:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) expected_artifact_set_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) durable_candidate_closure_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) file_set_binding_digest:
        String,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) prepared_at_ms:
        i64,
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) initial_absence:
        Option<ComputePluginCandidateVerificationInitialAbsence>,
}

impl fmt::Debug for ComputePluginCandidateVerificationRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateVerificationRecoveryKey")
            .field("verification_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("owner_plan_id", &self.owner_plan_id)
            .field("verification_generation", &self.verification_generation)
            .field("candidate_generation", &self.candidate_generation)
            .field("artifact_count", &self.artifact_count)
            .field(
                "initial_absence",
                &self.initial_absence.as_ref().map(|_| "<retained>"),
            )
            .finish()
    }
}

impl ComputePluginCandidateVerificationRecoveryKey {
    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn into_run_observed(
        mut self,
    ) -> Self {
        self.initial_absence = None;
        self
    }

    pub(super) fn mark_run_observed(&mut self) {
        self.initial_absence = None;
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

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_id(&self) -> &str {
        &self.verification_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_id(&self) -> &str {
        &self.owner_plan_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn owner_plan_digest(&self) -> &str {
        &self.owner_plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_generation(&self) -> i64 {
        self.verification_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_generation(&self) -> i64 {
        self.candidate_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn application_inventory_revision(&self) -> i64 {
        self.application_inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_state_revision(&self) -> i64 {
        self.authority_state_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_epoch(&self) -> i64 {
        self.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host) fn execution_inventory_revision(&self) -> i64 {
        self.execution_inventory_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn inventory_digest(&self) -> &str {
        &self.inventory_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_count(&self) -> usize {
        self.artifact_count
    }

    pub(in crate::node_agent_compute_plugin_host) fn artifact_bytes(&self) -> i64 {
        self.artifact_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn expected_artifact_set_digest(&self) -> &str {
        &self.expected_artifact_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn durable_candidate_closure_digest(
        &self,
    ) -> &str {
        &self.durable_candidate_closure_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn file_set_binding_digest(&self) -> &str {
        &self.file_set_binding_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared_at_ms(&self) -> i64 {
        self.prepared_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn initial_absence(
        &self,
    ) -> Option<&ComputePluginCandidateVerificationInitialAbsence> {
        self.initial_absence.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationOutcome {
    kind: ComputePluginCandidateVerificationOutcomeKind,
    verification_generation: i64,
    candidate_generation: i64,
    application_inventory_revision: i64,
    artifact_count: usize,
    artifact_bytes: i64,
    expected_artifact_set_digest: String,
    file_set_binding_digest: String,
    prepared_at_ms: i64,
    resolved_at_ms: Option<i64>,
    resolution_reason: Option<&'static str>,
    result_digest: Option<String>,
}

impl ComputePluginCandidateVerificationOutcome {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn from_store(
        kind: ComputePluginCandidateVerificationOutcomeKind,
        key: &ComputePluginCandidateVerificationRecoveryKey,
        resolved_at_ms: Option<i64>,
        resolution_reason: Option<&'static str>,
        result_digest: Option<String>,
    ) -> Self {
        Self {
            kind,
            verification_generation: key.verification_generation,
            candidate_generation: key.candidate_generation,
            application_inventory_revision: key.application_inventory_revision,
            artifact_count: key.artifact_count,
            artifact_bytes: key.artifact_bytes,
            expected_artifact_set_digest: key.expected_artifact_set_digest.clone(),
            file_set_binding_digest: key.file_set_binding_digest.clone(),
            prepared_at_ms: key.prepared_at_ms,
            resolved_at_ms,
            resolution_reason,
            result_digest,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn kind(
        &self,
    ) -> ComputePluginCandidateVerificationOutcomeKind {
        self.kind
    }

    pub(in crate::node_agent_compute_plugin_host) fn resolved_at_ms(&self) -> Option<i64> {
        self.resolved_at_ms
    }

    pub(in crate::node_agent_compute_plugin_host) fn resolution_reason(
        &self,
    ) -> Option<&'static str> {
        self.resolution_reason
    }
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateVerificationRecoveryAbortPermit<
    'permit,
> {
    key: &'permit ComputePluginCandidateVerificationRecoveryKey,
    observed: &'permit ComputePluginCandidateVerificationOutcome,
}

impl<'permit> ValidatedCandidateVerificationRecoveryAbortPermit<'permit> {
    pub(super) fn new(
        key: &'permit ComputePluginCandidateVerificationRecoveryKey,
        observed: &'permit ComputePluginCandidateVerificationOutcome,
    ) -> Self {
        Self { key, observed }
    }

    pub(in crate::node_agent_compute_plugin_host) fn key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        self.key
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed(
        &self,
    ) -> &ComputePluginCandidateVerificationOutcome {
        self.observed
    }
}
