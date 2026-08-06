use std::fmt;

use super::AuthorizedCandidateHealthQuarantine;
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::CandidateHealthStagingExpectation,
    local_authority::ComputePluginAuthorityInstanceBinding,
};

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineReceiptExpectation {
    pub(in crate::node_agent_compute_plugin_host) evaluation_id: String,
    pub(in crate::node_agent_compute_plugin_host) failure_observation_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision_after: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision_before: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision_after: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest_before: String,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest_after: String,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_before: i64,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch_after: i64,
    pub(in crate::node_agent_compute_plugin_host) process_owner_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) trusted_time_high_water_ms_before: i64,
    pub(in crate::node_agent_compute_plugin_host) failed_at_ms: i64,
}

/// Process-local identity for classifying one uncertain quarantine commit. It cannot authorize a
/// retry, filesystem deletion, installation or promotion.
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    quarantine_id: String,
    receipt: CandidateHealthQuarantineReceiptExpectation,
    staging: CandidateHealthStagingExpectation,
}

impl CandidateHealthQuarantineRecoveryKey {
    pub(super) fn from_authorized(
        authorized: &AuthorizedCandidateHealthQuarantine<'_, '_>,
    ) -> Self {
        let observation = &authorized.publication.observation().observation;
        let staged = authorized.publication.staged();
        let staging_key = staged.recovery_key();
        let staging_slot = staging_key.slot_expectation();
        let facts = &authorized.facts;
        Self {
            authority_instance_binding: authorized
                .authority_session
                .authority_instance_binding()
                .clone(),
            installation_id_digest: observation.installation_id_digest.clone(),
            clock_epoch_digest: observation.clock_epoch_digest.clone(),
            quarantine_id: authorized.quarantine_id.clone(),
            receipt: CandidateHealthQuarantineReceiptExpectation {
                evaluation_id: observation.evaluation_id.clone(),
                failure_observation_digest: authorized
                    .publication
                    .observation()
                    .observation_digest
                    .clone(),
                authority_state_revision_before: facts.authority_state_revision_before(),
                authority_state_revision_after: facts.authority_state_revision_after(),
                inventory_revision_before: facts.inventory_revision_before(),
                inventory_revision_after: facts.inventory_revision_after(),
                inventory_digest_before: facts.inventory_digest_before().to_string(),
                inventory_digest_after: facts.inventory_digest_after().to_string(),
                authority_epoch_before: facts.authority_epoch_before(),
                authority_epoch_after: facts.authority_epoch_after(),
                process_owner_epoch: facts.process_owner_epoch(),
                trusted_time_high_water_ms_before: facts.trusted_time_high_water_ms_before(),
                failed_at_ms: facts.failed_at_ms(),
            },
            staging: CandidateHealthStagingExpectation {
                candidate_token: staging_key.candidate_token().to_string(),
                candidate_token_digest: staging_key.candidate_token_digest().to_string(),
                staging_id: staging_key.staging_id().to_string(),
                staging_receipt_digest: staged.receipt().receipt_digest().to_string(),
                staging_run_digest: staging_key.staging_run_digest().to_string(),
                plugin_id: staging_slot.plugin_id.clone(),
                slot_ref: staging_slot.slot_ref.clone(),
                release: staging_slot.release.clone(),
            },
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

    pub(in crate::node_agent_compute_plugin_host) fn quarantine_id(&self) -> &str {
        &self.quarantine_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_expectation(
        &self,
    ) -> &CandidateHealthQuarantineReceiptExpectation {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_expectation(
        &self,
    ) -> &CandidateHealthStagingExpectation {
        &self.staging
    }
}

impl fmt::Debug for CandidateHealthQuarantineRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthQuarantineRecoveryKey")
            .field("quarantine_id", &"<redacted>")
            .field("evaluation_id", &"<redacted>")
            .field(
                "candidate_token_digest",
                &self.staging.candidate_token_digest,
            )
            .field(
                "authority_state_revision_after",
                &self.receipt.authority_state_revision_after,
            )
            .field(
                "inventory_revision_after",
                &self.receipt.inventory_revision_after,
            )
            .field("authority_epoch_after", &self.receipt.authority_epoch_after)
            .finish()
    }
}
