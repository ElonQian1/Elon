use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    validate_store_receipt, CandidateHealthOutcomeUncertainCustody,
    DurableCandidateHealthPublication,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginCandidateHealthRecoveryAuthoritySession,
    ComputePluginCandidateHealthRecoveryOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateHealthRecoveryAdoptionPhase {
    OutcomeNotAdoptable,
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedContentChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthRecoveryAdoptionFailure<'root> {
    phase: CandidateHealthRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateHealthOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_health<'root>(
    mut recovery: CandidateHealthOutcomeUncertainCustody<'root>,
    observed: &ComputePluginCandidateHealthRecoveryOutcome,
    authority_session: ComputePluginCandidateHealthRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    DurableCandidateHealthPublication<'root>,
    CandidateHealthRecoveryAdoptionFailure<'root>,
> {
    if observed.is_not_created() {
        return Err(adoption_failure(
            CandidateHealthRecoveryAdoptionPhase::OutcomeNotAdoptable,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_ADOPTION_OUTCOME_NOT_RECORDED"),
            recovery,
        ));
    }
    if let Err(error) = validate_provenance(&recovery, observed, &authority_session) {
        return Err(adoption_failure(
            CandidateHealthRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match authority_session.read_candidate_health_outcome(&recovery.recovery_key) {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateHealthRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateHealthRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_ADOPTION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    let receipt = match fresh {
        ComputePluginCandidateHealthRecoveryOutcome::Recorded(receipt) => receipt,
        ComputePluginCandidateHealthRecoveryOutcome::NotCreated => {
            return Err(adoption_failure(
                CandidateHealthRecoveryAdoptionPhase::OutcomeChanged,
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_ADOPTION_OUTCOME_NOT_RECORDED"),
                recovery,
            ))
        }
    };
    if let Err(error) = validate_retained_content(&mut recovery) {
        return Err(adoption_failure(
            CandidateHealthRecoveryAdoptionPhase::RetainedContentChanged,
            error,
            recovery,
        ));
    }
    if let Err(error) =
        validate_store_receipt(&recovery.publication, &recovery.recovery_key, &receipt)
    {
        return Err(adoption_failure(
            CandidateHealthRecoveryAdoptionPhase::OutcomeChanged,
            error,
            recovery,
        ));
    }
    let (staged, _, _) = recovery.publication.into_parts();
    Ok(DurableCandidateHealthPublication { staged, receipt })
}

fn validate_provenance(
    recovery: &CandidateHealthOutcomeUncertainCustody<'_>,
    observed: &ComputePluginCandidateHealthRecoveryOutcome,
    session: &ComputePluginCandidateHealthRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || observed.recorded_receipt().is_none()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_ADOPTION_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn validate_retained_content(
    recovery: &mut CandidateHealthOutcomeUncertainCustody<'_>,
) -> Result<()> {
    let staged = &mut recovery.publication.staged;
    staged
        .archive()
        .snapshot_cancellation_guard()
        .ensure_current()?;
    staged.revalidate_retained_content()?;
    staged
        .archive()
        .snapshot_cancellation_guard()
        .ensure_current()
}

fn adoption_failure<'root>(
    phase: CandidateHealthRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateHealthOutcomeUncertainCustody<'root>,
) -> CandidateHealthRecoveryAdoptionFailure<'root> {
    CandidateHealthRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl CandidateHealthRecoveryAdoptionFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateHealthRecoveryAdoptionPhase {
        self.phase
    }
}

impl<'root> CandidateHealthRecoveryAdoptionFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateHealthOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl fmt::Display for CandidateHealthRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateHealthRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthRecoveryAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateHealthRecoveryAdoptionFailure<'_> {}
