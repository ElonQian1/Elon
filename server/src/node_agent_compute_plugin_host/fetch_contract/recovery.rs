use anyhow::{bail, Result};

use super::types::{
    AuthorizedComputePluginDownloadSegment, ComputePluginFetchAuthoritySnapshot,
    PreparedComputePluginFetchClaim,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::AdmittedComputePluginDownload,
    install_plan_admission_validation::is_identifier,
    local_authority::ComputePluginFetchAuthoritySession, manifest_validation::is_sha256,
};

mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginFetchClaimOutcome, ComputePluginFetchClaimOutcomeKind,
    ComputePluginFetchClaimRecoveryKey, ComputePluginFetchInitialClaimAbsenceSnapshot,
    ValidatedComputePluginFetchRecoveryAbortPermit,
};

/// Captures the exact claim identity and sealed Store-session provenance before mutation. Every
/// field was already derived from validated authority facts, so construction is deliberately
/// infallible: a Store result can be uncertain without stranding the claim recovery identity.
pub(super) fn capture_expected_claim_recovery_key(
    download: &AdmittedComputePluginDownload,
    claim: &PreparedComputePluginFetchClaim,
    observed_redirect_generation: i64,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
    authority_session: &ComputePluginFetchAuthoritySession<'_>,
) -> ComputePluginFetchClaimRecoveryKey {
    let facts = &snapshot.store;
    ComputePluginFetchClaimRecoveryKey {
        installation_id_digest: authority_session
            .recovery_installation_id_digest()
            .to_string(),
        claim_id: claim.claim_id.clone(),
        plan_id: claim.plan_id.clone(),
        plan_digest: claim.plan_digest.clone(),
        ordinal: claim.ordinal,
        candidate_token_digest: claim.candidate_token_digest.clone(),
        part_relative_path: claim.part_relative_path.clone(),
        artifact_digest: download.download.digest.clone(),
        artifact_size_bytes: download.download.size_bytes,
        authority_epoch: claim.authority_epoch,
        process_owner_epoch: claim.process_owner_epoch,
        cursor_generation: claim.cursor_generation,
        observed_redirect_generation,
        offset_bytes: claim.offset_bytes,
        length_bytes: claim.length_bytes,
        end_offset_bytes: claim.end_offset_bytes,
        prepared_at_ms: claim.prepared_at_ms,
        initial_absence: (claim.redirect_generation == 0 && facts.prepared_claim.is_none()).then(
            || ComputePluginFetchInitialClaimAbsenceSnapshot {
                expected_redirect_generation: claim.redirect_generation,
                authority_state_revision: facts.authority_state_revision,
                trusted_time_high_water_ms: facts.observed_trusted_time_high_water_ms,
                download_committed_offset: facts.committed_offset,
                download_cursor_generation: facts.download_cursor_generation,
                download_state: facts.download_state.clone(),
                download_updated_at_ms: facts.download_updated_at_ms,
            },
        ),
    }
}

impl ComputePluginFetchClaimRecoveryKey {
    /// A successful Store return proves the initial claim was created. Any later disappearance is
    /// corruption, so the pre-mutation absence snapshot must not escape with the authorization.
    pub(super) fn into_claim_observed(mut self) -> Self {
        self.initial_absence = None;
        self
    }
}

impl AuthorizedComputePluginDownloadSegment {
    /// Proves that a later resolution session belongs to the same installation and process fence
    /// as the session that originally returned this claim. Failure retains this whole handle and
    /// must occur before any Store mutation is attempted.
    pub(super) fn validate_recovery_session(
        &self,
        authority_session: &ComputePluginFetchAuthoritySession<'_>,
    ) -> Result<()> {
        let expected_end = self
            .claim
            .offset_bytes
            .checked_add(self.claim.length_bytes)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RECOVERY_RANGE_OVERFLOW"))?;
        let installation_id_digest = authority_session.recovery_installation_id_digest();
        let key = &self.recovery_key;
        if !is_sha256(installation_id_digest)
            || installation_id_digest != key.installation_id_digest
            || authority_session.recovery_process_owner_epoch() != key.process_owner_epoch
            || key.claim_id != self.claim.claim_id
            || key.plan_id != self.claim.plan_id
            || key.plan_digest != self.claim.plan_digest
            || key.ordinal != self.claim.ordinal
            || key.candidate_token_digest != self.claim.candidate_token_digest
            || key.part_relative_path != self.claim.part_relative_path
            || key.artifact_digest != self.download.download.digest
            || key.artifact_size_bytes != self.download.download.size_bytes
            || key.authority_epoch != self.claim.authority_epoch
            || key.process_owner_epoch != self.claim.process_owner_epoch
            || key.cursor_generation != self.claim.cursor_generation
            || key.observed_redirect_generation < 0
            || key.observed_redirect_generation > self.claim.redirect_generation
            || key.offset_bytes != self.claim.offset_bytes
            || key.length_bytes != self.claim.length_bytes
            || key.end_offset_bytes != self.claim.end_offset_bytes
            || key.prepared_at_ms != self.claim.prepared_at_ms
            || key.initial_absence.is_some()
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
        Ok(())
    }

    /// Consumes the old mutation capability and leaves only a non-authorizing outcome probe.
    pub(super) fn into_recovery_key(self) -> ComputePluginFetchClaimRecoveryKey {
        self.recovery_key
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
