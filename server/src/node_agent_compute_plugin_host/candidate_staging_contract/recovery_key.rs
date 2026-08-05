use std::fmt;

use super::capability::AuthorizedComputePluginCandidateStaging;
use crate::node_agent_compute_plugin_host::local_authority::ComputePluginAuthorityInstanceBinding;

/// Process-local identity for classifying an uncertain staging commit. It is not a retry permit.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateStagingRecoveryKey {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    clock_epoch_digest: String,
    staging_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    verification_id: String,
    staging_run_digest: String,
    process_owner_epoch: i64,
}

impl fmt::Debug for ComputePluginCandidateStagingRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateStagingRecoveryKey")
            .field("staging_id", &"<redacted>")
            .field("candidate_token", &"<redacted>")
            .field("candidate_token_digest", &self.candidate_token_digest)
            .field("verification_id", &"<redacted>")
            .field("staging_run_digest", &"<redacted>")
            .field("process_owner_epoch", &self.process_owner_epoch)
            .finish()
    }
}

impl ComputePluginCandidateStagingRecoveryKey {
    pub(super) fn from_authorized(
        authorized: &AuthorizedComputePluginCandidateStaging<'_, '_>,
        staging_id: String,
    ) -> Self {
        let key = authorized.revalidated.archive().verification_recovery_key();
        Self {
            authority_instance_binding: key.authority_instance_binding().clone(),
            installation_id_digest: key.installation_id_digest().to_string(),
            clock_epoch_digest: key.clock_epoch_digest().to_string(),
            staging_id,
            candidate_token: key.candidate_token().to_string(),
            candidate_token_digest: key.candidate_token_digest().to_string(),
            verification_id: key.verification_id().to_string(),
            staging_run_digest: authorized
                .revalidated
                .archive()
                .evidence()
                .evidence
                .staging_run_digest
                .clone(),
            process_owner_epoch: key.process_owner_epoch(),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_id(&self) -> &str {
        &self.staging_id
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

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token(&self) -> &str {
        &self.candidate_token
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        &self.candidate_token_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verification_id(&self) -> &str {
        &self.verification_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }
}
