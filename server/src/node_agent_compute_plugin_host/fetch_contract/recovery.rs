use anyhow::{bail, Result};

use super::types::AuthorizedComputePluginDownloadSegment;
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    local_authority::ComputePluginFetchAuthoritySession, manifest_validation::is_sha256,
};

mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginFetchClaimOutcome, ComputePluginFetchClaimOutcomeKind,
    ComputePluginFetchClaimRecoveryKey, ValidatedComputePluginFetchRecoveryAbortPermit,
};

impl AuthorizedComputePluginDownloadSegment {
    /// Capture before any redirect/commit/abort that consumes this handle. The key can inspect
    /// history but cannot authorize network, file, cursor or claim mutations.
    pub(super) fn recovery_key(
        &self,
        authority_session: &ComputePluginFetchAuthoritySession<'_>,
    ) -> Result<ComputePluginFetchClaimRecoveryKey> {
        let expected_end = self
            .claim
            .offset_bytes
            .checked_add(self.claim.length_bytes)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RECOVERY_RANGE_OVERFLOW"))?;
        let installation_id_digest = authority_session.recovery_installation_id_digest();
        if !is_sha256(installation_id_digest)
            || !is_identifier(&self.claim.claim_id)
            || !is_identifier(&self.claim.plan_id)
            || !is_sha256(&self.claim.plan_digest)
            || !is_sha256(&self.claim.candidate_token_digest)
            || !is_sha256(&self.download.download.digest)
            || !super::relative_fetch_path_is_valid(&self.claim.part_relative_path)
            || self.download.ordinal != self.claim.ordinal
            || self.offset_bytes != self.claim.offset_bytes
            || self.length_bytes != self.claim.length_bytes
            || i64::from(self.redirect_hop) != self.claim.redirect_generation
            || self.claim.end_offset_bytes != expected_end
            || self.claim.end_offset_bytes > self.download.download.size_bytes
            || self.claim.authority_epoch <= 0
            || self.claim.process_owner_epoch <= 0
            || self.claim.cursor_generation <= 0
            || self.claim.redirect_generation < 0
            || self.claim.redirect_generation > 5
            || self.claim.prepared_at_ms < 0
        {
            bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_HANDLE_INVALID");
        }
        Ok(ComputePluginFetchClaimRecoveryKey {
            installation_id_digest: installation_id_digest.to_string(),
            claim_id: self.claim.claim_id.clone(),
            plan_id: self.claim.plan_id.clone(),
            plan_digest: self.claim.plan_digest.clone(),
            ordinal: self.claim.ordinal,
            candidate_token_digest: self.claim.candidate_token_digest.clone(),
            part_relative_path: self.claim.part_relative_path.clone(),
            artifact_digest: self.download.download.digest.clone(),
            artifact_size_bytes: self.download.download.size_bytes,
            authority_epoch: self.claim.authority_epoch,
            process_owner_epoch: self.claim.process_owner_epoch,
            cursor_generation: self.claim.cursor_generation,
            observed_redirect_generation: self.claim.redirect_generation,
            offset_bytes: self.claim.offset_bytes,
            length_bytes: self.claim.length_bytes,
            end_offset_bytes: self.claim.end_offset_bytes,
            prepared_at_ms: self.claim.prepared_at_ms,
        })
    }
}

/// Stable read for a same-process uncertain mutation or a terminal claim from an older process
/// epoch. Kept private until file cursor reconciliation is implemented.
pub(super) fn inspect_claim_outcome(
    key: &ComputePluginFetchClaimRecoveryKey,
    authority_session: &ComputePluginFetchAuthoritySession<'_>,
) -> Result<ComputePluginFetchClaimOutcome> {
    authority_session.read_claim_outcome(key)
}

/// Abort only a claim that a stable read observed as exact-current prepared. Any Store error is
/// outcome-uncertain; the caller retains `key` and must inspect again instead of retrying mutation.
pub(super) fn abort_recovered_prepared_claim(
    key: &ComputePluginFetchClaimRecoveryKey,
    observed: &ComputePluginFetchClaimOutcome,
    authority_session: ComputePluginFetchAuthoritySession<'_>,
) -> Result<ComputePluginFetchClaimOutcome> {
    if observed.kind() != ComputePluginFetchClaimOutcomeKind::Prepared {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_NOT_PREPARED");
    }
    let fresh = authority_session.read_claim_outcome(key)?;
    if &fresh != observed {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_OUTCOME_CHANGED");
    }
    let permit = ValidatedComputePluginFetchRecoveryAbortPermit::new(key, &fresh);
    authority_session.abort_recovered_prepared_claim(permit)
}
