use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    validate_quarantine_receipt, CandidateHealthQuarantineOutcomeUncertainCustody,
    DurableCandidateHealthQuarantine,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession,
    ComputePluginCandidateHealthQuarantineRecoveryOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateHealthQuarantineRecoveryAdoptionPhase {
    OutcomeNotAdoptable,
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedContentChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineRecoveryAdoptionFailure<
    'root,
> {
    phase: CandidateHealthQuarantineRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_health_quarantine<
    'root,
>(
    mut recovery: CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
    observed: &ComputePluginCandidateHealthQuarantineRecoveryOutcome,
    authority_session: ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    DurableCandidateHealthQuarantine<'root>,
    CandidateHealthQuarantineRecoveryAdoptionFailure<'root>,
> {
    if observed.is_not_created() {
        return Err(adoption_failure(
            CandidateHealthQuarantineRecoveryAdoptionPhase::OutcomeNotAdoptable,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_ADOPTION_OUTCOME_NOT_CREATED"),
            recovery,
        ));
    }
    if let Err(error) = validate_provenance(&recovery, observed, &authority_session) {
        return Err(adoption_failure(
            CandidateHealthQuarantineRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh =
        match authority_session.read_candidate_health_quarantine_outcome(&recovery.recovery_key) {
            Ok(fresh) => fresh,
            Err(error) => {
                return Err(adoption_failure(
                    CandidateHealthQuarantineRecoveryAdoptionPhase::OutcomeReadFailed,
                    error,
                    recovery,
                ))
            }
        };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateHealthQuarantineRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_ADOPTION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    let receipt = match fresh {
        ComputePluginCandidateHealthQuarantineRecoveryOutcome::Quarantined(receipt) => receipt,
        ComputePluginCandidateHealthQuarantineRecoveryOutcome::NotCreated => {
            return Err(adoption_failure(
                CandidateHealthQuarantineRecoveryAdoptionPhase::OutcomeChanged,
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_ADOPTION_OUTCOME_NOT_CREATED"),
                recovery,
            ))
        }
    };
    if let Err(error) = validate_retained_content(&mut recovery) {
        return Err(adoption_failure(
            CandidateHealthQuarantineRecoveryAdoptionPhase::RetainedContentChanged,
            error,
            recovery,
        ));
    }
    if let Err(error) =
        validate_quarantine_receipt(&recovery.publication, &recovery.recovery_key, &receipt)
    {
        return Err(adoption_failure(
            CandidateHealthQuarantineRecoveryAdoptionPhase::OutcomeChanged,
            error,
            recovery,
        ));
    }
    let (staged, _, _) = recovery.publication.into_parts();
    Ok(DurableCandidateHealthQuarantine { staged, receipt })
}

fn validate_provenance(
    recovery: &CandidateHealthQuarantineOutcomeUncertainCustody<'_>,
    observed: &ComputePluginCandidateHealthQuarantineRecoveryOutcome,
    session: &ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || observed.quarantined_receipt().is_none()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_ADOPTION_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn validate_retained_content(
    recovery: &mut CandidateHealthQuarantineOutcomeUncertainCustody<'_>,
) -> Result<()> {
    let staged = recovery.publication.staged_mut();
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
    phase: CandidateHealthQuarantineRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
) -> CandidateHealthQuarantineRecoveryAdoptionFailure<'root> {
    CandidateHealthQuarantineRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl CandidateHealthQuarantineRecoveryAdoptionFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateHealthQuarantineRecoveryAdoptionPhase {
        self.phase
    }
}

impl<'root> CandidateHealthQuarantineRecoveryAdoptionFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
    ) {
        (self.error, self.recovery)
    }
}

impl fmt::Display for CandidateHealthQuarantineRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateHealthQuarantineRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthQuarantineRecoveryAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateHealthQuarantineRecoveryAdoptionFailure<'_> {}
