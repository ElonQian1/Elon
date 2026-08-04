use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::{
    begin::CandidateVerificationBeginRecoveryCustody,
    hash::{validate_hash_binding, CandidateArtifactSetHashEvidence},
    CandidateArtifactSetHashDisposition, ComputePluginCandidateVerificationRecoveryKey,
    HashedComputePluginCandidateArtifactSet, PinnedComputePluginCandidateArtifactSet,
};
use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::{
        ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
        ComputePluginPostHashVerificationAuthoritySession,
        ComputePluginPostHashVerificationBindingFacts,
        ComputePluginPreparedCandidateVerificationFacts,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) struct CandidateVerificationPostHashBindPermit {
    _private: (),
}

/// Linear evidence that the exact prepared run and pinned file set were re-observed through an
/// authenticated trusted-time session strictly after the full hash barrier. This capability has
/// no generic Store writer; the later resolution kernel must consume it and repeat the exact read
/// inside one `BEGIN IMMEDIATE` transaction.
#[must_use = "trusted hash custody must be resolved or explicitly abandoned"]
pub(in crate::node_agent_compute_plugin_host) struct TrustedHashedComputePluginCandidateArtifactSet<
    'authority,
> {
    prepared: ComputePluginPreparedCandidateVerificationFacts,
    recovery_key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
    evidence: CandidateArtifactSetHashEvidence,
    binding_facts: ComputePluginPostHashVerificationBindingFacts,
    authority_session: ComputePluginPostHashVerificationAuthoritySession<'authority>,
    hash_completed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationPostHashBindingPhase {
    PreTrustedTimeBinding,
    PreparedRunRead,
    PostTrustedTimeBinding,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationPostHashBindingFailure {
    phase: CandidateVerificationPostHashBindingPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
}

impl CandidateVerificationPostHashBindPermit {
    fn new() -> Self {
        Self { _private: () }
    }
}

impl TrustedHashedComputePluginCandidateArtifactSet<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn disposition(
        &self,
    ) -> CandidateArtifactSetHashDisposition {
        self.evidence.disposition()
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_artifact_set_digest(&self) -> &str {
        self.evidence.observed_artifact_set_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_ordinal(&self) -> Option<usize> {
        self.evidence.mismatch_ordinal()
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_observed_digest(
        &self,
    ) -> Option<&str> {
        self.evidence.mismatch_observed_digest()
    }
}

impl fmt::Debug for TrustedHashedComputePluginCandidateArtifactSet<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TrustedHashedComputePluginCandidateArtifactSet")
            .field("prepared", &self.prepared)
            .field("recovery_key", &self.recovery_key)
            .field("pinned", &self.pinned)
            .field("evidence", &self.evidence)
            .field("binding_facts", &self.binding_facts)
            .field("authority_session", &"<post-hash-trusted-time>")
            .field("hash_completed_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for CandidateVerificationPostHashBindingFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationPostHashBindingFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateVerificationPostHashBindingFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateVerificationPostHashBindingFailure {}

impl CandidateVerificationPostHashBindingFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateVerificationPostHashBindingPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateVerificationBeginRecoveryCustody {
        self.recovery
    }
}

pub(in crate::node_agent_compute_plugin_host) fn bind_hashed_candidate_artifact_set<'authority>(
    hashed: HashedComputePluginCandidateArtifactSet,
    observation: ComputePluginTrustedTimeObservation,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
) -> std::result::Result<
    TrustedHashedComputePluginCandidateArtifactSet<'authority>,
    CandidateVerificationPostHashBindingFailure,
> {
    let (prepared, recovery_key, mut pinned, evidence, hash_completed_at) =
        hashed.into_post_hash_parts(CandidateVerificationPostHashBindPermit::new());

    if let Err(error) = revalidate_hashed_parts(
        &prepared,
        &recovery_key,
        &mut pinned,
        &evidence,
        hash_completed_at,
    ) {
        return Err(parts_failure(
            CandidateVerificationPostHashBindingPhase::PreTrustedTimeBinding,
            error,
            recovery_key,
            pinned,
        ));
    }

    if let Err(error) = validate_observation(&observation, &recovery_key, hash_completed_at) {
        return Err(parts_failure(
            CandidateVerificationPostHashBindingPhase::PreTrustedTimeBinding,
            error,
            recovery_key,
            pinned,
        ));
    }
    let authority_session = match authority.bind_post_hash_verification_authority_session(
        process_fence,
        observation,
        roots,
    ) {
        Ok(session) => session,
        Err(error) => {
            return Err(parts_failure(
                CandidateVerificationPostHashBindingPhase::PreTrustedTimeBinding,
                error,
                recovery_key,
                pinned,
            ))
        }
    };
    if let Err(error) = validate_session(
        &authority_session,
        &recovery_key,
        &pinned.cancellation_guard,
        hash_completed_at,
    ) {
        return Err(parts_failure(
            CandidateVerificationPostHashBindingPhase::PreTrustedTimeBinding,
            error,
            recovery_key,
            pinned,
        ));
    }
    let binding_facts_before =
        match authority_session.read_prepared_candidate_verification_binding(&recovery_key) {
            Ok(outcome) => outcome,
            Err(error) => {
                return Err(parts_failure(
                    CandidateVerificationPostHashBindingPhase::PreparedRunRead,
                    error,
                    recovery_key,
                    pinned,
                ))
            }
        };
    if let Err(error) = revalidate_hashed_parts(
        &prepared,
        &recovery_key,
        &mut pinned,
        &evidence,
        hash_completed_at,
    )
    .and_then(|_| {
        validate_session(
            &authority_session,
            &recovery_key,
            &pinned.cancellation_guard,
            hash_completed_at,
        )
    }) {
        return Err(parts_failure(
            CandidateVerificationPostHashBindingPhase::PostTrustedTimeBinding,
            error,
            recovery_key,
            pinned,
        ));
    }
    let binding_facts =
        match authority_session.read_prepared_candidate_verification_binding(&recovery_key) {
            Ok(facts) if facts == binding_facts_before => facts,
            Ok(_) => {
                return Err(parts_failure(
                    CandidateVerificationPostHashBindingPhase::PostTrustedTimeBinding,
                    anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_SNAPSHOT_CHANGED"),
                    recovery_key,
                    pinned,
                ))
            }
            Err(error) => {
                return Err(parts_failure(
                    CandidateVerificationPostHashBindingPhase::PostTrustedTimeBinding,
                    error,
                    recovery_key,
                    pinned,
                ))
            }
        };
    if let Err(error) = validate_session(
        &authority_session,
        &recovery_key,
        &pinned.cancellation_guard,
        hash_completed_at,
    ) {
        return Err(parts_failure(
            CandidateVerificationPostHashBindingPhase::PostTrustedTimeBinding,
            error,
            recovery_key,
            pinned,
        ));
    }

    Ok(TrustedHashedComputePluginCandidateArtifactSet {
        prepared,
        recovery_key,
        pinned,
        evidence,
        binding_facts,
        authority_session,
        hash_completed_at,
    })
}

/// Explicitly abandons trusted hash evidence without making the exact prepared Store run
/// retryable. Recovery can only inspect or abort that already-created run.
pub(in crate::node_agent_compute_plugin_host) fn abandon_trusted_hashed_candidate_artifact_set(
    bound: TrustedHashedComputePluginCandidateArtifactSet<'_>,
) -> CandidateVerificationBeginRecoveryCustody {
    CandidateVerificationBeginRecoveryCustody {
        key: bound.recovery_key,
        pinned: bound.pinned,
    }
}

fn revalidate_hashed_parts(
    prepared: &ComputePluginPreparedCandidateVerificationFacts,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    evidence: &CandidateArtifactSetHashEvidence,
    hash_completed_at: Instant,
) -> Result<()> {
    validate_hash_binding(pinned, prepared, key)?;
    if hash_completed_at > Instant::now() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_EVIDENCE_INVALID");
    }
    evidence.validate(key, pinned)
}

fn validate_observation(
    observation: &ComputePluginTrustedTimeObservation,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    hash_completed_at: Instant,
) -> Result<()> {
    if observation.installation_id_digest() != key.installation_id_digest()
        || observation.clock_epoch_digest() != key.clock_epoch_digest()
        || observation.observed_at() <= hash_completed_at
        || observation.trusted_now().timestamp_millis() <= key.prepared_at_ms()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_OBSERVATION_INVALID");
    }
    Ok(())
}

fn validate_session(
    session: &ComputePluginPostHashVerificationAuthoritySession<'_>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    cancellation_guard: &crate::node_agent_compute_plugin_host::fetch_contract::ComputePluginFetchCancellationGuard,
    hash_completed_at: Instant,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.process_owner_epoch() != session.process_owner_epoch()
        || !session.was_observed_strictly_after(hash_completed_at)
        || session.trusted_now_ms() <= key.prepared_at_ms()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_POST_HASH_SESSION_INVALID");
    }
    session.validate_post_hash_source(cancellation_guard)
}

fn parts_failure(
    phase: CandidateVerificationPostHashBindingPhase,
    error: Error,
    key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
) -> CandidateVerificationPostHashBindingFailure {
    binding_failure(
        phase,
        error,
        CandidateVerificationBeginRecoveryCustody { key, pinned },
    )
}

fn binding_failure(
    phase: CandidateVerificationPostHashBindingPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
) -> CandidateVerificationPostHashBindingFailure {
    CandidateVerificationPostHashBindingFailure {
        phase,
        error,
        recovery,
    }
}
