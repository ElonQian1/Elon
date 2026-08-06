use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    validate_cleanup_authorization_receipt, AuthorizedCandidateCleanup,
    CandidateCleanupAuthorizationOutcomeUncertainCustody,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginCandidateCleanupRecoveryAuthoritySession,
    ComputePluginCandidateCleanupRecoveryOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupAuthorizationRecoveryAdoptionPhase
{
    OutcomeNotAdoptable,
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedContentChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationRecoveryAdoptionFailure<
    'root,
> {
    phase: CandidateCleanupAuthorizationRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_authorization<
    'root,
>(
    mut recovery: CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
    observed: &ComputePluginCandidateCleanupRecoveryOutcome,
    authority_session: ComputePluginCandidateCleanupRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    AuthorizedCandidateCleanup<'root>,
    CandidateCleanupAuthorizationRecoveryAdoptionFailure<'root>,
> {
    if observed.is_not_created() {
        return Err(adoption_failure(
            CandidateCleanupAuthorizationRecoveryAdoptionPhase::OutcomeNotAdoptable,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ADOPTION_NOT_CREATED"),
            recovery,
        ));
    }
    if let Err(error) = validate_provenance(&recovery, observed, &authority_session) {
        return Err(adoption_failure(
            CandidateCleanupAuthorizationRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match authority_session
        .read_candidate_cleanup_authorization_outcome(&recovery.recovery_key)
    {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateCleanupAuthorizationRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateCleanupAuthorizationRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ADOPTION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    let receipt = match fresh {
        ComputePluginCandidateCleanupRecoveryOutcome::Authorized(receipt) => receipt,
        ComputePluginCandidateCleanupRecoveryOutcome::NotCreated => {
            return Err(adoption_failure(
                CandidateCleanupAuthorizationRecoveryAdoptionPhase::OutcomeChanged,
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ADOPTION_NOT_CREATED"),
                recovery,
            ))
        }
    };
    if let Err(error) = recovery.quarantined.revalidate_retained_content() {
        return Err(adoption_failure(
            CandidateCleanupAuthorizationRecoveryAdoptionPhase::RetainedContentChanged,
            error,
            recovery,
        ));
    }
    if let Err(error) = validate_cleanup_authorization_receipt(&recovery.recovery_key, &receipt) {
        return Err(adoption_failure(
            CandidateCleanupAuthorizationRecoveryAdoptionPhase::OutcomeChanged,
            error,
            recovery,
        ));
    }
    Ok(AuthorizedCandidateCleanup {
        quarantined: recovery.quarantined,
        receipt,
    })
}

fn validate_provenance(
    recovery: &CandidateCleanupAuthorizationOutcomeUncertainCustody<'_>,
    observed: &ComputePluginCandidateCleanupRecoveryOutcome,
    session: &ComputePluginCandidateCleanupRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || observed.authorization_receipt().is_none()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ADOPTION_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn adoption_failure<'root>(
    phase: CandidateCleanupAuthorizationRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
) -> CandidateCleanupAuthorizationRecoveryAdoptionFailure<'root> {
    CandidateCleanupAuthorizationRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl CandidateCleanupAuthorizationRecoveryAdoptionFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupAuthorizationRecoveryAdoptionPhase {
        self.phase
    }
}

impl<'root> CandidateCleanupAuthorizationRecoveryAdoptionFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
    ) {
        (self.error, self.recovery)
    }
}

impl fmt::Display for CandidateCleanupAuthorizationRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupAuthorizationRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupAuthorizationRecoveryAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateCleanupAuthorizationRecoveryAdoptionFailure<'_> {}
