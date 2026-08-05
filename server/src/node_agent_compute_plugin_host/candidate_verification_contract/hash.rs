use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};
use serde::Serialize;

use super::{
    begin::{
        AuthorizedComputePluginCandidateArtifactSet, CandidateVerificationBeginRecoveryCustody,
    },
    compute_file_set_binding_digest,
    hash_budget::ActiveCandidateVerificationHashBudget,
    PinnedComputePluginCandidateArtifactSet,
};
use crate::{
    node_agent_compute_plugin_host::{
        manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
    },
    node_agent_managed_fs::{ManagedFileHashPhase, ManagedFileHashResult},
};

const OBSERVED_ARTIFACT_SET_SCHEMA: &str = "elon.compute_plugin.candidate_observed_artifact_set.v1";

mod evidence;
use evidence::CandidateArtifactDigestMismatch;
pub(super) use evidence::CandidateArtifactSetHashEvidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateArtifactSetHashDisposition {
    Matched,
    DigestMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationHashPhase {
    BudgetAdmission,
    PreHashBinding,
    FileHash(ManagedFileHashPhase),
    BudgetAccounting,
    PostHashBinding,
}

pub(super) struct CandidateVerificationHashPermit {
    _private: (),
}

#[must_use = "hashed candidate custody must be trusted-time bound or explicitly abandoned"]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateArtifactSet {
    prepared:
        crate::node_agent_compute_plugin_host::local_authority::ComputePluginPreparedCandidateVerificationFacts,
    recovery_key: super::ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
    evidence: CandidateArtifactSetHashEvidence,
    hash_completed_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationHashFailure {
    phase: CandidateVerificationHashPhase,
    ordinal: Option<usize>,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
}

struct HashedCandidateArtifact {
    ordinal: usize,
    item_index: usize,
    expected_len: u64,
    expected_digest: String,
    observed_digest: String,
    file_identity_digest: String,
    completed_at: Instant,
}

#[derive(Serialize)]
struct ObservedArtifactSetBinding<'binding> {
    schema: &'static str,
    digest_algorithm: &'static str,
    verification_id: &'binding str,
    verification_generation: i64,
    file_set_binding_digest: &'binding str,
    expected_artifact_set_digest: &'binding str,
    artifact_count: usize,
    artifact_bytes: u64,
    artifacts: &'binding [ObservedArtifactDigest],
}

#[derive(Serialize)]
struct ObservedArtifactDigest {
    ordinal: usize,
    item_index: usize,
    size_bytes: u64,
    expected_digest: String,
    observed_digest: String,
    file_identity_digest: String,
}

impl HashedComputePluginCandidateArtifactSet {
    pub(in crate::node_agent_compute_plugin_host) fn disposition(
        &self,
    ) -> CandidateArtifactSetHashDisposition {
        self.evidence.disposition()
    }

    pub(super) fn into_post_hash_parts(
        self,
        _permit: super::post_hash::CandidateVerificationPostHashBindPermit,
    ) -> (
        crate::node_agent_compute_plugin_host::local_authority::ComputePluginPreparedCandidateVerificationFacts,
        super::ComputePluginCandidateVerificationRecoveryKey,
        PinnedComputePluginCandidateArtifactSet,
        CandidateArtifactSetHashEvidence,
        Instant,
    ){
        (
            self.prepared,
            self.recovery_key,
            self.pinned,
            self.evidence,
            self.hash_completed_at,
        )
    }
}

impl CandidateVerificationHashPermit {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl fmt::Debug for HashedComputePluginCandidateArtifactSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let barrier_state = if self.hash_completed_at <= Instant::now() {
            "<monotonic>"
        } else {
            "<invalid>"
        };
        formatter
            .debug_struct("HashedComputePluginCandidateArtifactSet")
            .field("prepared", &self.prepared)
            .field("recovery_key", &self.recovery_key)
            .field("pinned", &self.pinned)
            .field("evidence", &self.evidence)
            .field("hash_completed_at", &barrier_state)
            .finish()
    }
}

impl fmt::Debug for CandidateVerificationHashFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationHashFailure")
            .field("phase", &self.phase)
            .field("ordinal", &self.ordinal)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateVerificationHashFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateVerificationHashFailure {}

impl CandidateVerificationHashFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateVerificationHashPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn ordinal(&self) -> Option<usize> {
        self.ordinal
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateVerificationBeginRecoveryCustody {
        self.recovery
    }
}

pub(in crate::node_agent_compute_plugin_host) fn hash_authorized_candidate_artifact_set(
    authorized: AuthorizedComputePluginCandidateArtifactSet,
    budget: super::CandidateVerificationHashBudget,
) -> std::result::Result<HashedComputePluginCandidateArtifactSet, CandidateVerificationHashFailure>
{
    let (prepared, recovery_key, mut pinned) =
        authorized.into_hash_parts(CandidateVerificationHashPermit::new());
    let expected_artifact_bytes = match u64::try_from(recovery_key.artifact_bytes()) {
        Ok(bytes) if bytes > 0 => bytes,
        _ => {
            return Err(hash_failure(
                CandidateVerificationHashPhase::BudgetAdmission,
                None,
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_BUDGET_BINDING_INVALID"),
                recovery_key,
                pinned,
            ))
        }
    };
    let mut active_budget = match budget.activate(expected_artifact_bytes) {
        Ok(active) => active,
        Err(error) => {
            return Err(hash_failure(
                CandidateVerificationHashPhase::BudgetAdmission,
                None,
                error,
                recovery_key,
                pinned,
            ))
        }
    };
    if let Err(error) =
        validate_hash_binding_with_guard(&mut pinned, &prepared, &recovery_key, || {
            active_budget.ensure_current()
        })
    {
        return Err(hash_failure(
            CandidateVerificationHashPhase::PreHashBinding,
            None,
            error,
            recovery_key,
            pinned,
        ));
    }

    let mut hashed_artifact_count = 0_usize;
    let mut hashed_artifact_bytes = 0_u64;
    let mut last_file_hash_completed_at = None;
    let mut mismatch = None;
    let mut observations = Vec::with_capacity(recovery_key.artifact_count());
    for artifact_index in 0..pinned.artifacts.len() {
        let hashed = match hash_one_artifact(&mut pinned, artifact_index, &active_budget) {
            Ok(hashed) => hashed,
            Err((ordinal, phase, error)) => {
                return Err(hash_failure(
                    CandidateVerificationHashPhase::FileHash(phase),
                    Some(ordinal),
                    error,
                    recovery_key,
                    pinned,
                ))
            }
        };
        if let Err(error) = active_budget.record_hashed(hashed.expected_len) {
            return Err(hash_failure(
                CandidateVerificationHashPhase::BudgetAccounting,
                Some(hashed.ordinal),
                error,
                recovery_key,
                pinned,
            ));
        }
        hashed_artifact_count += 1;
        hashed_artifact_bytes = match hashed_artifact_bytes.checked_add(hashed.expected_len) {
            Some(total) => total,
            None => {
                return Err(hash_failure(
                    CandidateVerificationHashPhase::PostHashBinding,
                    Some(hashed.ordinal),
                    anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_BYTES_OVERFLOW"),
                    recovery_key,
                    pinned,
                ))
            }
        };
        last_file_hash_completed_at = Some(hashed.completed_at);
        if mismatch.is_none() && hashed.observed_digest != hashed.expected_digest {
            mismatch = Some(CandidateArtifactDigestMismatch::new(
                hashed.ordinal,
                hashed.expected_digest.clone(),
                hashed.observed_digest.clone(),
            ));
        }
        observations.push(ObservedArtifactDigest {
            ordinal: hashed.ordinal,
            item_index: hashed.item_index,
            size_bytes: hashed.expected_len,
            expected_digest: hashed.expected_digest,
            observed_digest: hashed.observed_digest,
            file_identity_digest: hashed.file_identity_digest,
        });
    }

    if let Err(error) =
        validate_hash_binding_with_guard(&mut pinned, &prepared, &recovery_key, || {
            active_budget.ensure_current()
        })
    {
        return Err(hash_failure(
            CandidateVerificationHashPhase::PostHashBinding,
            None,
            error,
            recovery_key,
            pinned,
        ));
    }
    if hashed_artifact_count != recovery_key.artifact_count()
        || observations.len() != recovery_key.artifact_count()
        || i64::try_from(hashed_artifact_bytes).ok() != Some(recovery_key.artifact_bytes())
    {
        return Err(hash_failure(
            CandidateVerificationHashPhase::PostHashBinding,
            None,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_CLOSURE_INCOMPLETE"),
            recovery_key,
            pinned,
        ));
    }
    let observed_artifact_set_digest = match jcs_sha256_hex(&ObservedArtifactSetBinding {
        schema: OBSERVED_ARTIFACT_SET_SCHEMA,
        digest_algorithm: "sha256",
        verification_id: recovery_key.verification_id(),
        verification_generation: recovery_key.verification_generation(),
        file_set_binding_digest: recovery_key.file_set_binding_digest(),
        expected_artifact_set_digest: recovery_key.expected_artifact_set_digest(),
        artifact_count: hashed_artifact_count,
        artifact_bytes: hashed_artifact_bytes,
        artifacts: &observations,
    }) {
        Ok(digest) => digest,
        Err(error) => {
            return Err(hash_failure(
                CandidateVerificationHashPhase::PostHashBinding,
                None,
                error,
                recovery_key,
                pinned,
            ))
        }
    };
    let disposition = if mismatch.is_some() {
        CandidateArtifactSetHashDisposition::DigestMismatch
    } else {
        CandidateArtifactSetHashDisposition::Matched
    };
    let evidence = CandidateArtifactSetHashEvidence::new(
        disposition,
        hashed_artifact_count,
        hashed_artifact_bytes,
        observed_artifact_set_digest,
        mismatch,
    );
    if let Err(error) = active_budget.finish(hashed_artifact_bytes) {
        return Err(hash_failure(
            CandidateVerificationHashPhase::BudgetAccounting,
            None,
            error,
            recovery_key,
            pinned,
        ));
    }
    if let Err(error) = pinned.cancellation_guard.ensure_current() {
        return Err(hash_failure(
            CandidateVerificationHashPhase::PostHashBinding,
            None,
            error,
            recovery_key,
            pinned,
        ));
    }
    let hash_completed_at = Instant::now();
    if last_file_hash_completed_at.is_none()
        || last_file_hash_completed_at.is_some_and(|completed_at| completed_at > hash_completed_at)
    {
        return Err(hash_failure(
            CandidateVerificationHashPhase::PostHashBinding,
            None,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_BARRIER_INVALID"),
            recovery_key,
            pinned,
        ));
    }
    Ok(HashedComputePluginCandidateArtifactSet {
        prepared,
        recovery_key,
        pinned,
        evidence,
        hash_completed_at,
    })
}

/// Explicitly abandons in-memory hash evidence without making the prepared Store run retryable.
/// The returned custody can only inspect or abort the existing run through authority recovery.
pub(in crate::node_agent_compute_plugin_host) fn abandon_hashed_candidate_artifact_set(
    hashed: HashedComputePluginCandidateArtifactSet,
) -> CandidateVerificationBeginRecoveryCustody {
    CandidateVerificationBeginRecoveryCustody {
        key: hashed.recovery_key,
        pinned: hashed.pinned,
    }
}

fn hash_one_artifact(
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    artifact_index: usize,
    budget: &ActiveCandidateVerificationHashBudget,
) -> std::result::Result<HashedCandidateArtifact, (usize, ManagedFileHashPhase, Error)> {
    let cancellation_guard = &pinned.cancellation_guard;
    let artifact = &mut pinned.artifacts[artifact_index];
    let hashed: ManagedFileHashResult = artifact
        .file
        .hash_sha256_and_revalidate(artifact.expected_len, || {
            cancellation_guard.ensure_current()?;
            budget.ensure_current()
        })
        .map_err(|failure| {
            let phase = failure.phase();
            (artifact.ordinal, phase, failure.into_error())
        })?;
    Ok(HashedCandidateArtifact {
        ordinal: artifact.ordinal,
        item_index: artifact.item_index,
        expected_len: artifact.expected_len,
        expected_digest: artifact.expected_digest.clone(),
        observed_digest: hashed.digest().to_string(),
        file_identity_digest: artifact.file.identity_digest().to_string(),
        completed_at: hashed.completed_at(),
    })
}

pub(super) fn validate_hash_binding(
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    prepared: &crate::node_agent_compute_plugin_host::local_authority::ComputePluginPreparedCandidateVerificationFacts,
    key: &super::ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    validate_hash_binding_with_guard(pinned, prepared, key, || Ok(()))
}

fn validate_hash_binding_with_guard(
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    prepared: &crate::node_agent_compute_plugin_host::local_authority::ComputePluginPreparedCandidateVerificationFacts,
    key: &super::ComputePluginCandidateVerificationRecoveryKey,
    mut ensure_budget: impl FnMut() -> Result<()>,
) -> Result<()> {
    ensure_budget()?;
    pinned.cancellation_guard.ensure_current()?;
    let artifact_bytes = pinned.artifacts.iter().try_fold(0_u64, |total, artifact| {
        total.checked_add(artifact.expected_len)
    });
    let ordinals_are_strict = pinned
        .artifacts
        .windows(2)
        .all(|pair| pair[0].ordinal < pair[1].ordinal);
    if !prepared.matches_recovery_key(key)
        || key.initial_absence().is_some()
        || pinned.verification_id != key.verification_id()
        || pinned.candidate_token != key.candidate_token()
        || pinned.installation_binding_digest != key.installation_id_digest()
        || pinned.root_identity_digest != key.root_identity_digest()
        || pinned.file_set_binding_digest != key.file_set_binding_digest()
        || pinned.artifacts.len() != key.artifact_count()
        || artifact_bytes.and_then(|bytes| i64::try_from(bytes).ok()) != Some(key.artifact_bytes())
        || !ordinals_are_strict
        || pinned.discovery_authority.expected_artifact_set_digest
            != key.expected_artifact_set_digest()
        || pinned
            .discovery_authority
            .recompute_durable_candidate_closure_digest()?
            != key.durable_candidate_closure_digest()
        || pinned.artifacts.iter().any(|artifact| {
            artifact.expected_len == 0
                || !is_sha256(&artifact.expected_digest)
                || !is_sha256(artifact.file.identity_digest())
        })
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_BINDING_INVALID");
    }
    for artifact in &mut pinned.artifacts {
        ensure_budget()?;
        pinned.cancellation_guard.ensure_current()?;
        artifact.file.revalidate_exact_len(artifact.expected_len)?;
    }
    ensure_budget()?;
    pinned.cancellation_guard.ensure_current()?;
    let rebound = compute_file_set_binding_digest(
        &pinned.verification_id,
        &pinned.discovery_authority,
        &pinned.installation_binding_digest,
        &pinned.root_identity_digest,
        &pinned.artifacts,
    )?;
    if rebound != pinned.file_set_binding_digest {
        bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_FILE_SET_CHANGED");
    }
    ensure_budget()?;
    Ok(())
}

fn hash_failure(
    phase: CandidateVerificationHashPhase,
    ordinal: Option<usize>,
    error: Error,
    key: super::ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
) -> CandidateVerificationHashFailure {
    CandidateVerificationHashFailure {
        phase,
        ordinal,
        error,
        recovery: CandidateVerificationBeginRecoveryCustody { key, pinned },
    }
}
