use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{anyhow, bail, Error, Result};

use super::{
    begin::CandidateVerificationBeginRecoveryCustody,
    hash::CandidateArtifactSetHashEvidence,
    post_hash::{revalidate_hashed_parts, validate_session},
    CandidateArtifactSetHashDisposition, ComputePluginCandidateVerificationOutcome,
    ComputePluginCandidateVerificationOutcomeKind, ComputePluginCandidateVerificationRecoveryKey,
    PinnedComputePluginCandidateArtifactSet, TrustedHashedComputePluginCandidateArtifactSet,
};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::ComputePluginFetchCancellationGuard,
    lifecycle::{SLOT_FAILED, SLOT_VERIFYING},
    local_authority::{
        ComputePluginPostHashVerificationAuthoritySession,
        ComputePluginPostHashVerificationBindingFacts,
        ComputePluginPreparedCandidateVerificationFacts,
    },
    manifest_validation::is_sha256,
};

mod adoption;

pub(in crate::node_agent_compute_plugin_host) use adoption::{
    adopt_recovered_candidate_verification_resolution,
    CandidateVerificationResolutionAdoptionFailure, CandidateVerificationResolutionAdoptionPhase,
};

pub(super) struct CandidateVerificationResolutionPermit {
    _private: (),
}

pub(super) struct CandidateVerificationResolutionParts<'authority> {
    prepared: ComputePluginPreparedCandidateVerificationFacts,
    recovery_key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
    evidence: CandidateArtifactSetHashEvidence,
    binding_facts: ComputePluginPostHashVerificationBindingFacts,
    authority_session: ComputePluginPostHashVerificationAuthoritySession<'authority>,
    hash_completed_at: Instant,
}

/// The only Store-write permit for a candidate artifact resolution. It borrows one consumed
/// TrustedHashed capability and exposes only the exact immutable facts needed by the transaction.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateVerificationResolutionPermit<
    'permit,
> {
    key: &'permit ComputePluginCandidateVerificationRecoveryKey,
    prepared: &'permit ComputePluginPreparedCandidateVerificationFacts,
    s3_binding: &'permit ComputePluginPostHashVerificationBindingFacts,
    disposition: CandidateArtifactSetHashDisposition,
    observed_artifact_set_digest: &'permit str,
    mismatch_ordinal: Option<usize>,
    mismatch_expected_digest: Option<&'permit str>,
    mismatch_observed_digest: Option<&'permit str>,
    cancellation_guard: &'permit ComputePluginFetchCancellationGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationResolutionPhase {
    PreStoreBinding,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationResolutionFailure {
    phase: CandidateVerificationResolutionPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationResolutionCustody {
    Verified(VerifiedComputePluginCandidateArtifactSet),
    Rejected(RejectedComputePluginCandidateArtifactSetCustody),
}

/// Raw artifacts whose exact Store run durably reached `verified`. This is not an installed,
/// healthy, promotable, or commercially verified compute capability.
#[must_use = "verified artifact custody must be consumed by the next local install stage"]
pub(in crate::node_agent_compute_plugin_host) struct VerifiedComputePluginCandidateArtifactSet {
    outcome: ComputePluginCandidateVerificationOutcome,
    recovery_key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
}

/// Rejected artifacts remain pinned only for deterministic local cleanup and audit handoff.
#[must_use = "rejected artifact custody must be handed to deterministic cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct RejectedComputePluginCandidateArtifactSetCustody
{
    outcome: ComputePluginCandidateVerificationOutcome,
    recovery_key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
}

impl CandidateVerificationResolutionPermit {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl<'authority> CandidateVerificationResolutionParts<'authority> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        prepared: ComputePluginPreparedCandidateVerificationFacts,
        recovery_key: ComputePluginCandidateVerificationRecoveryKey,
        pinned: PinnedComputePluginCandidateArtifactSet,
        evidence: CandidateArtifactSetHashEvidence,
        binding_facts: ComputePluginPostHashVerificationBindingFacts,
        authority_session: ComputePluginPostHashVerificationAuthoritySession<'authority>,
        hash_completed_at: Instant,
    ) -> Self {
        Self {
            prepared,
            recovery_key,
            pinned,
            evidence,
            binding_facts,
            authority_session,
            hash_completed_at,
        }
    }

    fn into_recovery(self) -> CandidateVerificationBeginRecoveryCustody {
        CandidateVerificationBeginRecoveryCustody {
            key: self.recovery_key,
            pinned: self.pinned,
        }
    }
}

impl<'permit> ValidatedCandidateVerificationResolutionPermit<'permit> {
    fn new(parts: &'permit CandidateVerificationResolutionParts<'_>) -> Self {
        Self {
            key: &parts.recovery_key,
            prepared: &parts.prepared,
            s3_binding: &parts.binding_facts,
            disposition: parts.evidence.disposition(),
            observed_artifact_set_digest: parts.evidence.observed_artifact_set_digest(),
            mismatch_ordinal: parts.evidence.mismatch_ordinal(),
            mismatch_expected_digest: parts.evidence.mismatch_expected_digest(),
            mismatch_observed_digest: parts.evidence.mismatch_observed_digest(),
            cancellation_guard: &parts.pinned.cancellation_guard,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        self.key
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepared(
        &self,
    ) -> &ComputePluginPreparedCandidateVerificationFacts {
        self.prepared
    }

    pub(in crate::node_agent_compute_plugin_host) fn s3_binding(
        &self,
    ) -> &ComputePluginPostHashVerificationBindingFacts {
        self.s3_binding
    }

    pub(in crate::node_agent_compute_plugin_host) fn disposition(
        &self,
    ) -> CandidateArtifactSetHashDisposition {
        self.disposition
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_artifact_set_digest(&self) -> &str {
        self.observed_artifact_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_ordinal(&self) -> Option<usize> {
        self.mismatch_ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_expected_digest(
        &self,
    ) -> Option<&str> {
        self.mismatch_expected_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_observed_digest(
        &self,
    ) -> Option<&str> {
        self.mismatch_observed_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn cancellation_guard(
        &self,
    ) -> &ComputePluginFetchCancellationGuard {
        self.cancellation_guard
    }
}

impl CandidateVerificationResolutionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateVerificationResolutionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateVerificationBeginRecoveryCustody {
        self.recovery
    }
}

impl CandidateVerificationResolutionCustody {
    pub(in crate::node_agent_compute_plugin_host) fn outcome(
        &self,
    ) -> &ComputePluginCandidateVerificationOutcome {
        match self {
            Self::Verified(custody) => custody.outcome(),
            Self::Rejected(custody) => custody.outcome(),
        }
    }
}

impl VerifiedComputePluginCandidateArtifactSet {
    pub(in crate::node_agent_compute_plugin_host) fn outcome(
        &self,
    ) -> &ComputePluginCandidateVerificationOutcome {
        &self.outcome
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.recovery_key.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_token_digest(&self) -> &str {
        self.recovery_key.candidate_token_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        &self.recovery_key
    }

    pub(in crate::node_agent_compute_plugin_host) fn snapshot_cancellation_guard(
        &self,
    ) -> ComputePluginFetchCancellationGuard {
        self.pinned.cancellation_guard.clone()
    }

    pub(in crate::node_agent_compute_plugin_host) fn with_verified_package_file<T>(
        &mut self,
        item_index: usize,
        expected_digest: &str,
        expected_len: u64,
        operation: impl FnOnce(
            &mut crate::node_agent_managed_fs::PinnedManagedFile,
            crate::node_agent_compute_plugin_host::fetch_contract::ComputePluginFetchCancellationGuard,
        ) -> Result<T>,
    ) -> Result<T> {
        let cancellation_guard = self.pinned.cancellation_guard.clone();
        let package_ordinal = self
            .pinned
            .artifacts
            .iter()
            .filter(|artifact| artifact.item_index == item_index)
            .map(|artifact| artifact.ordinal)
            .min()
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_VERIFIED_PACKAGE_MISSING"))?;
        let package = self
            .pinned
            .artifacts
            .iter_mut()
            .find(|artifact| {
                artifact.item_index == item_index && artifact.ordinal == package_ordinal
            })
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_VERIFIED_PACKAGE_MISSING"))?;
        if package.expected_digest != expected_digest || package.expected_len != expected_len {
            bail!("COMPUTE_PLUGIN_VERIFIED_PACKAGE_BINDING_CHANGED");
        }
        operation(&mut package.file, cancellation_guard)
    }
}

impl RejectedComputePluginCandidateArtifactSetCustody {
    pub(in crate::node_agent_compute_plugin_host) fn outcome(
        &self,
    ) -> &ComputePluginCandidateVerificationOutcome {
        &self.outcome
    }
}

impl fmt::Debug for CandidateVerificationResolutionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationResolutionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateVerificationResolutionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateVerificationResolutionFailure {}

impl fmt::Debug for CandidateVerificationResolutionCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified(custody) => formatter.debug_tuple("Verified").field(custody).finish(),
            Self::Rejected(custody) => formatter.debug_tuple("Rejected").field(custody).finish(),
        }
    }
}

impl fmt::Debug for VerifiedComputePluginCandidateArtifactSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputePluginCandidateArtifactSet")
            .field("outcome", &self.outcome)
            .field("recovery_key", &self.recovery_key)
            .field("pinned", &self.pinned)
            .finish()
    }
}

impl fmt::Debug for RejectedComputePluginCandidateArtifactSetCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RejectedComputePluginCandidateArtifactSetCustody")
            .field("outcome", &self.outcome)
            .field("recovery_key", &self.recovery_key)
            .field("pinned", &self.pinned)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn resolve_trusted_hashed_candidate_artifact_set(
    trusted: TrustedHashedComputePluginCandidateArtifactSet<'_>,
) -> std::result::Result<
    CandidateVerificationResolutionCustody,
    CandidateVerificationResolutionFailure,
> {
    let mut parts = trusted.into_resolution_parts(CandidateVerificationResolutionPermit::new());
    if let Err(error) = validate_resolution_parts(&mut parts) {
        return Err(resolution_failure(
            CandidateVerificationResolutionPhase::PreStoreBinding,
            error,
            parts,
        ));
    }

    let store_result = parts.authority_session.resolve_candidate_verification(
        ValidatedCandidateVerificationResolutionPermit::new(&parts),
    );
    let outcome = match store_result {
        Ok(outcome) => outcome,
        Err(error) => {
            return Err(resolution_failure(
                CandidateVerificationResolutionPhase::StoreOutcomeUncertain,
                error,
                parts,
            ))
        }
    };
    if let Err(error) = validate_resolution_outcome(&parts, &outcome) {
        return Err(resolution_failure(
            CandidateVerificationResolutionPhase::StoreReturnedPostconditionFailed,
            error,
            parts,
        ));
    }
    Ok(into_resolution_custody(parts, outcome))
}

fn validate_resolution_parts(parts: &mut CandidateVerificationResolutionParts<'_>) -> Result<()> {
    revalidate_hashed_parts(
        &parts.prepared,
        &parts.recovery_key,
        &mut parts.pinned,
        &parts.evidence,
        parts.hash_completed_at,
    )?;
    validate_session(
        &parts.authority_session,
        &parts.recovery_key,
        &parts.pinned.cancellation_guard,
        parts.hash_completed_at,
    )?;
    let s3 = &parts.binding_facts;
    let key = &parts.recovery_key;
    if s3.outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Prepared
        || s3.authority_state_revision != key.authority_state_revision()
        || s3.inventory_revision != key.execution_inventory_revision()
        || s3.inventory_digest != key.inventory_digest()
        || s3.authority_epoch != key.authority_epoch()
        || s3.process_owner_epoch != key.process_owner_epoch()
        || s3.trusted_time_high_water_ms < key.prepared_at_ms()
        || s3.trusted_time_high_water_ms >= parts.authority_session.trusted_now_ms()
        || s3.durable_candidate_closure_digest != key.durable_candidate_closure_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_S3_BINDING_INVALID");
    }
    Ok(())
}

fn validate_resolution_outcome(
    parts: &CandidateVerificationResolutionParts<'_>,
    outcome: &ComputePluginCandidateVerificationOutcome,
) -> Result<()> {
    let expected_state_revision = parts.binding_facts.authority_state_revision.checked_add(1);
    let expected_inventory_revision = parts.binding_facts.inventory_revision.checked_add(1);
    let expected_authority_epoch = parts.binding_facts.authority_epoch.checked_add(1);
    let common_matches = outcome.resolved_at_ms() == Some(parts.authority_session.trusted_now_ms())
        && outcome.observed_artifact_set_digest()
            == Some(parts.evidence.observed_artifact_set_digest())
        && outcome.result_digest().is_some_and(is_sha256)
        && outcome.authority_state_revision_after() == expected_state_revision
        && outcome.inventory_revision_after() == expected_inventory_revision
        && outcome.inventory_digest_after().is_some_and(is_sha256)
        && outcome.inventory_digest_after() != Some(parts.binding_facts.inventory_digest.as_str())
        && outcome.authority_epoch_after() == expected_authority_epoch;
    let state_matches = match parts.evidence.disposition() {
        CandidateArtifactSetHashDisposition::Matched => {
            outcome.kind() == ComputePluginCandidateVerificationOutcomeKind::Verified
                && outcome.resolution_reason() == Some("artifact_set_verified")
                && outcome.mismatch().is_none()
                && outcome.slot_phase_after() == Some(SLOT_VERIFYING)
        }
        CandidateArtifactSetHashDisposition::DigestMismatch => {
            let mismatch = outcome.mismatch();
            outcome.kind() == ComputePluginCandidateVerificationOutcomeKind::Rejected
                && outcome.resolution_reason() == Some("artifact_digest_mismatch")
                && mismatch.map(|value| value.ordinal()) == parts.evidence.mismatch_ordinal()
                && mismatch.map(|value| value.expected_digest())
                    == parts.evidence.mismatch_expected_digest()
                && mismatch.map(|value| value.observed_digest())
                    == parts.evidence.mismatch_observed_digest()
                && outcome.slot_phase_after() == Some(SLOT_FAILED)
        }
    };
    if !common_matches || !state_matches {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_OUTCOME_CHANGED");
    }
    Ok(())
}

fn into_resolution_custody(
    parts: CandidateVerificationResolutionParts<'_>,
    outcome: ComputePluginCandidateVerificationOutcome,
) -> CandidateVerificationResolutionCustody {
    match parts.evidence.disposition() {
        CandidateArtifactSetHashDisposition::Matched => {
            CandidateVerificationResolutionCustody::Verified(
                VerifiedComputePluginCandidateArtifactSet {
                    outcome,
                    recovery_key: parts.recovery_key,
                    pinned: parts.pinned,
                },
            )
        }
        CandidateArtifactSetHashDisposition::DigestMismatch => {
            CandidateVerificationResolutionCustody::Rejected(
                RejectedComputePluginCandidateArtifactSetCustody {
                    outcome,
                    recovery_key: parts.recovery_key,
                    pinned: parts.pinned,
                },
            )
        }
    }
}

fn resolution_failure(
    phase: CandidateVerificationResolutionPhase,
    error: Error,
    parts: CandidateVerificationResolutionParts<'_>,
) -> CandidateVerificationResolutionFailure {
    CandidateVerificationResolutionFailure {
        phase,
        error,
        recovery: parts.into_recovery(),
    }
}
