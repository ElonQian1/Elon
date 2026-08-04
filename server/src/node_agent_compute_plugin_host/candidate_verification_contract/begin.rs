use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    compute_file_set_binding_digest, ObservedComputePluginCandidateArtifactSet,
    PinnedComputePluginCandidateArtifactSet,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    local_authority::{
        ComputePluginCandidateVerificationAuthorityFacts,
        ComputePluginPostPinVerificationAuthoritySession,
        ComputePluginPreparedCandidateVerificationFacts,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::recovery::{
    ComputePluginCandidateVerificationInitialAbsence, ComputePluginCandidateVerificationRecoveryKey,
};

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateVerificationBeginPermit<
    'permit,
> {
    key: &'permit ComputePluginCandidateVerificationRecoveryKey,
    authority: &'permit ComputePluginCandidateVerificationAuthorityFacts,
}

impl<'permit> ValidatedCandidateVerificationBeginPermit<'permit> {
    fn new(
        key: &'permit ComputePluginCandidateVerificationRecoveryKey,
        authority: &'permit ComputePluginCandidateVerificationAuthorityFacts,
    ) -> Self {
        Self { key, authority }
    }

    pub(in crate::node_agent_compute_plugin_host) fn key(
        &self,
    ) -> &ComputePluginCandidateVerificationRecoveryKey {
        self.key
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority(
        &self,
    ) -> &ComputePluginCandidateVerificationAuthorityFacts {
        self.authority
    }
}

/// Linear hash authorization. No hashing method exists until the purpose-specific binder lands.
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedComputePluginCandidateArtifactSet {
    prepared: ComputePluginPreparedCandidateVerificationFacts,
    recovery_key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: PinnedComputePluginCandidateArtifactSet,
}

impl AuthorizedComputePluginCandidateArtifactSet {
    pub(super) fn into_hash_parts(
        self,
        _permit: super::hash::CandidateVerificationHashPermit,
    ) -> (
        ComputePluginPreparedCandidateVerificationFacts,
        ComputePluginCandidateVerificationRecoveryKey,
        PinnedComputePluginCandidateArtifactSet,
    ) {
        (self.prepared, self.recovery_key, self.pinned)
    }
}

/// Abandon-only custody. A rejected begin must be dropped and a new attempt must re-pin every
/// file under a new verification ID; this type deliberately has no observe/begin/hash conversion.
pub(in crate::node_agent_compute_plugin_host) struct UnclaimedCandidateArtifactSetCustody {
    pinned: PinnedComputePluginCandidateArtifactSet,
}

/// Recovery-only custody. `NotCreated` and terminal outcomes close this attempt; a new begin must
/// re-pin under a new verification ID rather than converting these handles back into Observed.
pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationBeginRecoveryCustody {
    pub(super) key: ComputePluginCandidateVerificationRecoveryKey,
    pub(super) pinned: PinnedComputePluginCandidateArtifactSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationBeginMutationPhase {
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

pub(in crate::node_agent_compute_plugin_host) enum BeginCandidateVerificationFailure {
    RejectedBeforeStoreCall {
        error: Error,
        custody: UnclaimedCandidateArtifactSetCustody,
    },
    OutcomeRecoveryRequired {
        phase: CandidateVerificationBeginMutationPhase,
        error: Error,
        recovery: CandidateVerificationBeginRecoveryCustody,
    },
}

impl fmt::Debug for AuthorizedComputePluginCandidateArtifactSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputePluginCandidateArtifactSet")
            .field("prepared", &self.prepared)
            .field("recovery_key", &self.recovery_key)
            .field("pinned", &self.pinned)
            .finish()
    }
}

impl fmt::Debug for UnclaimedCandidateArtifactSetCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnclaimedCandidateArtifactSetCustody")
            .field("pinned", &self.pinned)
            .finish()
    }
}

impl fmt::Debug for CandidateVerificationBeginRecoveryCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationBeginRecoveryCustody")
            .field("key", &self.key)
            .field("pinned", &self.pinned)
            .finish()
    }
}

impl fmt::Debug for BeginCandidateVerificationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RejectedBeforeStoreCall { error, custody } => formatter
                .debug_struct("RejectedBeforeStoreCall")
                .field("error", error)
                .field("custody", custody)
                .finish(),
            Self::OutcomeRecoveryRequired {
                phase,
                error,
                recovery,
            } => formatter
                .debug_struct("OutcomeRecoveryRequired")
                .field("phase", phase)
                .field("error", error)
                .field("recovery", recovery)
                .finish(),
        }
    }
}

impl fmt::Display for BeginCandidateVerificationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RejectedBeforeStoreCall { error, .. }
            | Self::OutcomeRecoveryRequired { error, .. } => write!(formatter, "{error:#}"),
        }
    }
}

impl StdError for BeginCandidateVerificationFailure {}

impl BeginCandidateVerificationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_required(&self) -> bool {
        matches!(self, Self::OutcomeRecoveryRequired { .. })
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> Option<CandidateVerificationBeginRecoveryCustody> {
        match self {
            Self::OutcomeRecoveryRequired { recovery, .. } => Some(recovery),
            Self::RejectedBeforeStoreCall { .. } => None,
        }
    }
}

pub(in crate::node_agent_compute_plugin_host) fn begin_candidate_verification(
    observed: ObservedComputePluginCandidateArtifactSet<'_>,
) -> std::result::Result<
    AuthorizedComputePluginCandidateArtifactSet,
    BeginCandidateVerificationFailure,
> {
    let ObservedComputePluginCandidateArtifactSet {
        mut pinned,
        current_authority,
        authority_session,
    } = observed;
    if let Err(error) = validate_before_store(&mut pinned, &current_authority, &authority_session) {
        return Err(BeginCandidateVerificationFailure::RejectedBeforeStoreCall {
            error,
            custody: UnclaimedCandidateArtifactSetCustody { pinned },
        });
    }
    let key = match capture_recovery_key(&pinned, &current_authority, &authority_session) {
        Ok(key) => key,
        Err(error) => {
            return Err(BeginCandidateVerificationFailure::RejectedBeforeStoreCall {
                error,
                custody: UnclaimedCandidateArtifactSetCustody { pinned },
            })
        }
    };
    let permit = ValidatedCandidateVerificationBeginPermit::new(&key, &current_authority);
    let store_result = authority_session.begin_validated_candidate_verification(permit);
    let prepared = match store_result {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(BeginCandidateVerificationFailure::OutcomeRecoveryRequired {
                phase: CandidateVerificationBeginMutationPhase::StoreOutcomeUncertain,
                error,
                recovery: CandidateVerificationBeginRecoveryCustody { key, pinned },
            })
        }
    };
    // A successful Store return proves the absence case is permanently unavailable. Clear it
    // before any later check that can fail.
    let key = key.into_run_observed();
    if let Err(error) = validate_after_store(&mut pinned, &key, &prepared) {
        return Err(BeginCandidateVerificationFailure::OutcomeRecoveryRequired {
            phase: CandidateVerificationBeginMutationPhase::StoreReturnedPostconditionFailed,
            error,
            recovery: CandidateVerificationBeginRecoveryCustody { key, pinned },
        });
    }
    Ok(AuthorizedComputePluginCandidateArtifactSet {
        prepared,
        recovery_key: key,
        pinned,
    })
}

fn validate_before_store(
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    current: &ComputePluginCandidateVerificationAuthorityFacts,
    session: &ComputePluginPostPinVerificationAuthoritySession<'_>,
) -> Result<()> {
    session.validate_begin_source(&pinned.cancellation_guard)?;
    pinned.cancellation_guard.ensure_current()?;
    if !pinned.discovery_authority.same_durable_projection(current)
        || current.trusted_now.timestamp_millis() <= current.observed_trusted_time_high_water_ms
        || current.installation_id_digest != pinned.installation_binding_digest
        || session.installation_id_digest() != pinned.installation_binding_digest
        || session.process_owner_epoch() != current.process_owner_epoch
        || !is_identifier(&pinned.verification_id)
        || !is_sha256(&pinned.root_identity_digest)
        || !is_sha256(&pinned.file_set_binding_digest)
        || jcs_sha256_hex(&pinned.candidate_token)? != current.candidate_token_digest
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_BINDING_INVALID");
    }
    revalidate_pinned_files(pinned)?;
    let rebound = compute_file_set_binding_digest(
        &pinned.verification_id,
        current,
        &pinned.installation_binding_digest,
        &pinned.root_identity_digest,
        &pinned.artifacts,
    )?;
    if rebound != pinned.file_set_binding_digest
        || current.recompute_expected_artifact_set_digest()? != current.expected_artifact_set_digest
        || !is_sha256(&current.recompute_durable_candidate_closure_digest()?)
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_PROJECTION_CHANGED");
    }
    Ok(())
}

fn validate_after_store(
    pinned: &mut PinnedComputePluginCandidateArtifactSet,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    prepared: &ComputePluginPreparedCandidateVerificationFacts,
) -> Result<()> {
    pinned.cancellation_guard.ensure_current()?;
    revalidate_pinned_files(pinned)?;
    if !prepared.matches_recovery_key(key) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_RETURN_CHANGED");
    }
    Ok(())
}

fn revalidate_pinned_files(pinned: &mut PinnedComputePluginCandidateArtifactSet) -> Result<()> {
    for artifact in &mut pinned.artifacts {
        pinned.cancellation_guard.ensure_current()?;
        artifact.file.revalidate_exact_len(artifact.expected_len)?;
    }
    pinned.cancellation_guard.ensure_current()
}

fn capture_recovery_key(
    pinned: &PinnedComputePluginCandidateArtifactSet,
    facts: &ComputePluginCandidateVerificationAuthorityFacts,
    session: &ComputePluginPostPinVerificationAuthoritySession<'_>,
) -> Result<ComputePluginCandidateVerificationRecoveryKey> {
    let closure_digest = facts.recompute_durable_candidate_closure_digest()?;
    let initial_absence = ComputePluginCandidateVerificationInitialAbsence {
        authority_state_revision: facts.authority_state_revision,
        inventory_revision: facts.execution_inventory_revision,
        inventory_digest: facts.inventory_digest.clone(),
        trusted_time_high_water_ms: facts.observed_trusted_time_high_water_ms,
        next_verification_generation: facts.next_verification_generation,
        durable_candidate_closure_digest: closure_digest.clone(),
    };
    Ok(ComputePluginCandidateVerificationRecoveryKey {
        authority_instance_binding: session.authority_instance_binding().clone(),
        installation_id_digest: pinned.installation_binding_digest.clone(),
        clock_epoch_digest: session.clock_epoch_digest().to_string(),
        root_identity_digest: pinned.root_identity_digest.clone(),
        verification_id: pinned.verification_id.clone(),
        candidate_token: pinned.candidate_token.clone(),
        candidate_token_digest: facts.candidate_token_digest.clone(),
        owner_plan_id: facts.candidate_owner_plan_id.clone(),
        owner_plan_digest: facts.candidate_owner_plan_digest.clone(),
        verification_generation: facts.next_verification_generation,
        candidate_generation: facts.candidate_generation,
        application_inventory_revision: facts.candidate_application_inventory_revision,
        authority_state_revision: facts.authority_state_revision,
        authority_epoch: facts.authority_epoch,
        process_owner_epoch: facts.process_owner_epoch,
        execution_inventory_revision: facts.execution_inventory_revision,
        inventory_digest: facts.inventory_digest.clone(),
        artifact_count: facts.artifacts.len(),
        artifact_bytes: facts.artifact_bytes,
        expected_artifact_set_digest: facts.expected_artifact_set_digest.clone(),
        durable_candidate_closure_digest: closure_digest,
        file_set_binding_digest: pinned.file_set_binding_digest.clone(),
        prepared_at_ms: facts.trusted_now.timestamp_millis(),
        initial_absence: Some(initial_absence),
    })
}
