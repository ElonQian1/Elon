use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    CandidateVerificationResolutionCustody, RejectedComputePluginCandidateArtifactSetCustody,
    VerifiedComputePluginCandidateArtifactSet,
};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        begin::CandidateVerificationBeginRecoveryCustody, compute_file_set_binding_digest,
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationOutcomeKind,
        ComputePluginCandidateVerificationRecoveryKey, PinnedComputePluginCandidateArtifactSet,
    },
    local_authority::ComputePluginCandidateVerificationRecoveryAuthoritySession,
    manifest_validation::is_sha256,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationResolutionAdoptionPhase {
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    PinnedBindingChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationResolutionAdoptionFailure
{
    phase: CandidateVerificationResolutionAdoptionPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
}

impl CandidateVerificationResolutionAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateVerificationResolutionAdoptionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateVerificationBeginRecoveryCustody {
        self.recovery
    }
}

impl fmt::Debug for CandidateVerificationResolutionAdoptionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationResolutionAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateVerificationResolutionAdoptionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateVerificationResolutionAdoptionFailure {}

/// Adopts an exact Verified/Rejected terminal row after a resolution Store call returned an
/// uncertain error. The caller must first inspect the recovery key and pass that observed outcome;
/// this function consumes the retained handles, re-reads the exact row through a fresh recovery
/// session, and only restores typed custody when both observations are identical.
pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_verification_resolution(
    mut recovery: CandidateVerificationBeginRecoveryCustody,
    observed: &ComputePluginCandidateVerificationOutcome,
    authority_session: ComputePluginCandidateVerificationRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    CandidateVerificationResolutionCustody,
    CandidateVerificationResolutionAdoptionFailure,
> {
    if let Err(error) = validate_adoption_provenance(&recovery, observed, &authority_session) {
        return Err(adoption_failure(
            CandidateVerificationResolutionAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }

    let fresh = match authority_session.read_candidate_verification_outcome(&recovery.key) {
        Ok(fresh) => fresh,
        Err(failure) => {
            if failure.run_observed() {
                recovery = mark_run_observed(recovery);
            }
            return Err(adoption_failure(
                CandidateVerificationResolutionAdoptionPhase::OutcomeReadFailed,
                failure.into_error(),
                recovery,
            ));
        }
    };
    if fresh.kind() != ComputePluginCandidateVerificationOutcomeKind::NotCreated {
        recovery = mark_run_observed(recovery);
    }
    if &fresh != observed
        || !matches!(
            fresh.kind(),
            ComputePluginCandidateVerificationOutcomeKind::Verified
                | ComputePluginCandidateVerificationOutcomeKind::Rejected
        )
    {
        return Err(adoption_failure(
            CandidateVerificationResolutionAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_ADOPTION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = validate_recovered_pinned_binding(&mut recovery) {
        return Err(adoption_failure(
            CandidateVerificationResolutionAdoptionPhase::PinnedBindingChanged,
            error,
            recovery,
        ));
    }
    Ok(into_adopted_custody(recovery, fresh))
}

fn validate_adoption_provenance(
    recovery: &CandidateVerificationBeginRecoveryCustody,
    observed: &ComputePluginCandidateVerificationOutcome,
    authority_session: &ComputePluginCandidateVerificationRecoveryAuthoritySession<'_>,
) -> Result<()> {
    if !recovery
        .key
        .authority_instance_binding()
        .matches(authority_session.authority_instance_binding())
        || recovery.key.clock_epoch_digest() != authority_session.clock_epoch_digest()
        || !matches!(
            observed.kind(),
            ComputePluginCandidateVerificationOutcomeKind::Verified
                | ComputePluginCandidateVerificationOutcomeKind::Rejected
        )
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ADOPTION_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn validate_recovered_pinned_binding(
    recovery: &mut CandidateVerificationBeginRecoveryCustody,
) -> Result<()> {
    let key = &recovery.key;
    let pinned = &mut recovery.pinned;
    let artifact_bytes = pinned.artifacts.iter().try_fold(0_u64, |total, artifact| {
        total.checked_add(artifact.expected_len)
    });
    let ordinals_are_strict = pinned
        .artifacts
        .windows(2)
        .all(|pair| pair[0].ordinal < pair[1].ordinal);
    if key.initial_absence().is_some()
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
        bail!("COMPUTE_PLUGIN_VERIFICATION_ADOPTION_PINNED_BINDING_CHANGED");
    }
    for artifact in &mut pinned.artifacts {
        artifact.file.revalidate_exact_len(artifact.expected_len)?;
    }
    let rebound = compute_file_set_binding_digest(
        &pinned.verification_id,
        &pinned.discovery_authority,
        &pinned.installation_binding_digest,
        &pinned.root_identity_digest,
        &pinned.artifacts,
    )?;
    if rebound != pinned.file_set_binding_digest {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ADOPTION_FILE_SET_CHANGED");
    }
    Ok(())
}

fn mark_run_observed(
    recovery: CandidateVerificationBeginRecoveryCustody,
) -> CandidateVerificationBeginRecoveryCustody {
    CandidateVerificationBeginRecoveryCustody {
        key: recovery.key.into_run_observed(),
        pinned: recovery.pinned,
    }
}

fn into_adopted_custody(
    recovery: CandidateVerificationBeginRecoveryCustody,
    outcome: ComputePluginCandidateVerificationOutcome,
) -> CandidateVerificationResolutionCustody {
    if outcome.kind() == ComputePluginCandidateVerificationOutcomeKind::Verified {
        CandidateVerificationResolutionCustody::Verified(
            VerifiedComputePluginCandidateArtifactSet {
                outcome,
                recovery_key: recovery.key,
                pinned: recovery.pinned,
            },
        )
    } else {
        CandidateVerificationResolutionCustody::Rejected(
            RejectedComputePluginCandidateArtifactSetCustody {
                outcome,
                recovery_key: recovery.key,
                pinned: recovery.pinned,
            },
        )
    }
}

fn adoption_failure(
    phase: CandidateVerificationResolutionAdoptionPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
) -> CandidateVerificationResolutionAdoptionFailure {
    CandidateVerificationResolutionAdoptionFailure {
        phase,
        error,
        recovery,
    }
}
