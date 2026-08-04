use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::begin::CandidateVerificationBeginRecoveryCustody;
use crate::node_agent_compute_plugin_host::local_authority::ComputePluginCandidateVerificationRecoveryAuthoritySession;

mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginCandidateVerificationInitialAbsence, ComputePluginCandidateVerificationOutcome,
    ComputePluginCandidateVerificationOutcomeKind, ComputePluginCandidateVerificationRecoveryKey,
    ValidatedCandidateVerificationRecoveryAbortPermit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationRecoveryAbortPhase {
    RejectedBeforeStoreCall,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

pub(in crate::node_agent_compute_plugin_host) struct ResolvedCandidateArtifactSetCustody {
    outcome: ComputePluginCandidateVerificationOutcome,
    key: ComputePluginCandidateVerificationRecoveryKey,
    pinned: super::PinnedComputePluginCandidateArtifactSet,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationRecoveryAbortFailure {
    phase: CandidateVerificationRecoveryAbortPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
}

impl fmt::Debug for ResolvedCandidateArtifactSetCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedCandidateArtifactSetCustody")
            .field("outcome", &self.outcome)
            .field("key", &self.key)
            .field("pinned", &self.pinned)
            .finish()
    }
}

impl fmt::Debug for CandidateVerificationRecoveryAbortFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateVerificationRecoveryAbortFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateVerificationRecoveryAbortFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateVerificationRecoveryAbortFailure {}

impl CandidateVerificationRecoveryAbortFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateVerificationRecoveryAbortPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateVerificationBeginRecoveryCustody {
        self.recovery
    }
}

pub(in crate::node_agent_compute_plugin_host) fn inspect_candidate_verification_outcome(
    recovery: &mut CandidateVerificationBeginRecoveryCustody,
    authority_session: &ComputePluginCandidateVerificationRecoveryAuthoritySession<'_>,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    validate_recovery_authority_instance(&recovery.key, authority_session)?;
    let outcome = match authority_session.read_candidate_verification_outcome(&recovery.key) {
        Ok(outcome) => outcome,
        Err(failure) => {
            if failure.run_observed() {
                recovery.key.mark_run_observed();
            }
            return Err(failure.into_error());
        }
    };
    if outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::NotCreated {
        recovery.key.mark_run_observed();
    }
    Ok(outcome)
}

pub(in crate::node_agent_compute_plugin_host) fn abort_recovered_candidate_verification(
    mut recovery: CandidateVerificationBeginRecoveryCustody,
    observed: &ComputePluginCandidateVerificationOutcome,
    authority_session: ComputePluginCandidateVerificationRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    ResolvedCandidateArtifactSetCustody,
    CandidateVerificationRecoveryAbortFailure,
> {
    if let Err(error) = validate_recovery_authority_instance(&recovery.key, &authority_session) {
        return Err(abort_failure(
            CandidateVerificationRecoveryAbortPhase::RejectedBeforeStoreCall,
            error,
            recovery,
        ));
    }
    if observed.kind() != ComputePluginCandidateVerificationOutcomeKind::Prepared {
        return Err(abort_failure(
            CandidateVerificationRecoveryAbortPhase::RejectedBeforeStoreCall,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_NOT_PREPARED"),
            recovery,
        ));
    }
    let fresh = match authority_session.read_candidate_verification_outcome(&recovery.key) {
        Ok(fresh) => fresh,
        Err(failure) => {
            if failure.run_observed() {
                recovery.key.mark_run_observed();
            }
            return Err(abort_failure(
                CandidateVerificationRecoveryAbortPhase::RejectedBeforeStoreCall,
                failure.into_error(),
                recovery,
            ));
        }
    };
    if fresh.kind() != ComputePluginCandidateVerificationOutcomeKind::NotCreated {
        recovery.key.mark_run_observed();
    }
    if &fresh != observed {
        return Err(abort_failure(
            CandidateVerificationRecoveryAbortPhase::RejectedBeforeStoreCall,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    // Seeing the exact row makes a later disappearance corruption, even if the original begin
    // returned an uncertain error while still retaining an absence snapshot.
    let permit = ValidatedCandidateVerificationRecoveryAbortPermit::new(&recovery.key, &fresh);
    let store_result = authority_session.abort_recovered_candidate_verification(permit);
    let outcome = match store_result {
        Ok(outcome) => outcome,
        Err(error) => {
            return Err(abort_failure(
                CandidateVerificationRecoveryAbortPhase::StoreOutcomeUncertain,
                error,
                recovery,
            ))
        }
    };
    if outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Aborted
        || outcome.resolution_reason() != Some("authority_recovery")
    {
        return Err(abort_failure(
            CandidateVerificationRecoveryAbortPhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_RETURN_CHANGED"),
            recovery,
        ));
    }
    Ok(ResolvedCandidateArtifactSetCustody {
        outcome,
        key: recovery.key,
        pinned: recovery.pinned,
    })
}

fn validate_recovery_authority_instance(
    key: &ComputePluginCandidateVerificationRecoveryKey,
    authority_session: &ComputePluginCandidateVerificationRecoveryAuthoritySession<'_>,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(authority_session.authority_instance_binding())
        || key.clock_epoch_digest() != authority_session.clock_epoch_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_AUTHORITY_INSTANCE_CHANGED");
    }
    Ok(())
}

fn abort_failure(
    phase: CandidateVerificationRecoveryAbortPhase,
    error: Error,
    recovery: CandidateVerificationBeginRecoveryCustody,
) -> CandidateVerificationRecoveryAbortFailure {
    CandidateVerificationRecoveryAbortFailure {
        phase,
        error,
        recovery,
    }
}
