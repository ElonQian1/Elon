use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{CandidateStagingOutcomeUncertainCustody, StagedComputePluginCandidateArchive};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginCandidateStagingRecoveryAuthoritySession,
    ComputePluginCandidateStagingRecoveryOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateStagingRecoveryAdoptionPhase {
    OutcomeNotAdoptable,
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedContentChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingRecoveryAdoptionFailure<'root>
{
    phase: CandidateStagingRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateStagingOutcomeUncertainCustody<'root>,
}

impl<'root> CandidateStagingRecoveryAdoptionFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateStagingRecoveryAdoptionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateStagingOutcomeUncertainCustody<'root> {
        self.recovery
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateStagingOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl fmt::Debug for CandidateStagingRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateStagingRecoveryAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateStagingRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateStagingRecoveryAdoptionFailure<'_> {}

/// Reclaims exact staged custody after a Store call returned an uncertain error. The caller must
/// first inspect the recovery key and provide that observed outcome. Adoption consumes the
/// retained handles, performs a fresh authority read, and re-hashes the same files and staging
/// seal before restoring typed staged custody. `NotCreated` is deliberately not a retry permit.
pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_staging<'root>(
    mut recovery: CandidateStagingOutcomeUncertainCustody<'root>,
    observed: &ComputePluginCandidateStagingRecoveryOutcome,
    authority_session: ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    StagedComputePluginCandidateArchive<'root>,
    CandidateStagingRecoveryAdoptionFailure<'root>,
> {
    if observed.is_not_created() {
        return Err(adoption_failure(
            CandidateStagingRecoveryAdoptionPhase::OutcomeNotAdoptable,
            anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_ADOPTION_OUTCOME_NOT_STAGED"),
            recovery,
        ));
    }
    if let Err(error) = validate_adoption_provenance(&recovery, observed, &authority_session) {
        return Err(adoption_failure(
            CandidateStagingRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }

    let fresh = match authority_session.read_candidate_staging_outcome(&recovery.recovery_key) {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateStagingRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateStagingRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_ADOPTION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    let receipt = match fresh {
        ComputePluginCandidateStagingRecoveryOutcome::Staged(receipt) => receipt,
        ComputePluginCandidateStagingRecoveryOutcome::NotCreated => {
            return Err(adoption_failure(
                CandidateStagingRecoveryAdoptionPhase::OutcomeChanged,
                anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_ADOPTION_OUTCOME_NOT_STAGED"),
                recovery,
            ))
        }
    };
    if let Err(error) = validate_retained_content(&mut recovery) {
        return Err(adoption_failure(
            CandidateStagingRecoveryAdoptionPhase::RetainedContentChanged,
            error,
            recovery,
        ));
    }

    Ok(StagedComputePluginCandidateArchive {
        archive: recovery.revalidated.archive,
        receipt,
        recovery_key: recovery.recovery_key,
    })
}

fn validate_adoption_provenance(
    recovery: &CandidateStagingOutcomeUncertainCustody<'_>,
    observed: &ComputePluginCandidateStagingRecoveryOutcome,
    authority_session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(authority_session.authority_instance_binding())
        || key.installation_id_digest() != authority_session.installation_id_digest()
        || key.clock_epoch_digest() != authority_session.clock_epoch_digest()
        || key.process_owner_epoch() != authority_session.process_owner_epoch()
        || observed.staged_receipt().is_none()
    {
        bail!("COMPUTE_PLUGIN_STAGING_ADOPTION_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn validate_retained_content(
    recovery: &mut CandidateStagingOutcomeUncertainCustody<'_>,
) -> Result<()> {
    recovery.revalidated.cancellation_guard().ensure_current()?;
    let previous_barrier = recovery.revalidated.revalidated_at();
    let fresh_barrier = recovery
        .revalidated
        .archive_mut()
        .revalidate_for_staging_store()?;
    recovery.revalidated.cancellation_guard().ensure_current()?;
    if fresh_barrier < previous_barrier {
        bail!("COMPUTE_PLUGIN_STAGING_ADOPTION_MONOTONIC_BARRIER_CHANGED");
    }
    Ok(())
}

fn adoption_failure<'root>(
    phase: CandidateStagingRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateStagingOutcomeUncertainCustody<'root>,
) -> CandidateStagingRecoveryAdoptionFailure<'root> {
    CandidateStagingRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}
