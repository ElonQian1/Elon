use std::fmt;

use super::authorization::AuthorizedCandidateHealthStore;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, local_authority::ComputePluginAuthorityInstanceBinding,
};

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthReceiptExpectation {
    pub(in crate::node_agent_compute_plugin_host) evaluation_id: String,
    pub(in crate::node_agent_compute_plugin_host) health_observation_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_state_revision: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_revision: i64,
    pub(in crate::node_agent_compute_plugin_host) inventory_digest: String,
    pub(in crate::node_agent_compute_plugin_host) authority_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) process_owner_epoch: i64,
    pub(in crate::node_agent_compute_plugin_host) trusted_time_high_water_ms_before: i64,
    pub(in crate::node_agent_compute_plugin_host) recorded_at_ms: i64,
    pub(in crate::node_agent_compute_plugin_host) expires_at_ms: i64,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthStagingExpectation {
    pub(in crate::node_agent_compute_plugin_host) candidate_token: String,
    pub(in crate::node_agent_compute_plugin_host) candidate_token_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_id: String,
    pub(in crate::node_agent_compute_plugin_host) staging_receipt_digest: String,
    pub(in crate::node_agent_compute_plugin_host) staging_run_digest: String,
    pub(in crate::node_agent_compute_plugin_host) plugin_id: String,
    pub(in crate::node_agent_compute_plugin_host) slot_ref: String,
    pub(in crate::node_agent_compute_plugin_host) release: ComputePluginReleaseRef,
}

/// Process-local identity for classifying an uncertain health receipt commit. It cannot authorize
/// another Store attempt or promotion.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    health_id: String,
    receipt: CandidateHealthReceiptExpectation,
    staging: CandidateHealthStagingExpectation,
}

impl ComputePluginCandidateHealthRecoveryKey {
    pub(super) fn from_authorized(authorized: &AuthorizedCandidateHealthStore<'_, '_>) -> Self {
        let observation = &authorized.publication.observation().observation;
        let staged = authorized.publication.staged();
        let staging_key = staged.recovery_key();
        let staging_slot = staging_key.slot_expectation();
        Self {
            authority_instance_binding: authorized
                .authority_session
                .authority_instance_binding()
                .clone(),
            installation_id_digest: observation.installation_id_digest.clone(),
            clock_epoch_digest: observation.clock_epoch_digest.clone(),
            health_id: authorized.health_id.clone(),
            receipt: CandidateHealthReceiptExpectation {
                evaluation_id: observation.evaluation_id.clone(),
                health_observation_digest: authorized
                    .publication
                    .observation()
                    .observation_digest
                    .clone(),
                authority_state_revision: authorized.facts.authority_state_revision(),
                inventory_revision: authorized.facts.inventory_revision(),
                inventory_digest: authorized.facts.inventory_digest().to_string(),
                authority_epoch: authorized.facts.authority_epoch(),
                process_owner_epoch: authorized.facts.process_owner_epoch(),
                trusted_time_high_water_ms_before: authorized.facts.trusted_time_high_water_ms(),
                recorded_at_ms: authorized.facts.recorded_at_ms(),
                expires_at_ms: authorized.facts.expires_at_ms(),
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

    pub(in crate::node_agent_compute_plugin_host) fn health_id(&self) -> &str {
        &self.health_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt_expectation(
        &self,
    ) -> &CandidateHealthReceiptExpectation {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_expectation(
        &self,
    ) -> &CandidateHealthStagingExpectation {
        &self.staging
    }
}

impl fmt::Debug for ComputePluginCandidateHealthRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateHealthRecoveryKey")
            .field("health_id", &"<redacted>")
            .field("evaluation_id", &"<redacted>")
            .field(
                "candidate_token_digest",
                &self.staging.candidate_token_digest,
            )
            .field(
                "authority_state_revision",
                &self.receipt.authority_state_revision,
            )
            .field("inventory_revision", &self.receipt.inventory_revision)
            .field("authority_epoch", &self.receipt.authority_epoch)
            .finish()
    }
}
