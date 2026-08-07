use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error};

use super::SealedCandidateCleanupTopology;
use crate::node_agent_compute_plugin_host::{
    local_authority::{
        ComputePluginCandidateCleanupDeleteIntentAuthoritySession,
        ComputePluginCandidateCleanupDeleteIntentRecoveryAuthoritySession,
        ComputePluginCandidateCleanupDeleteIntentRecoveryOutcome, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod builder;
mod disposition;
mod disposition_recovery_key;
mod recovery_key;
mod types;

pub(in crate::node_agent_compute_plugin_host) use disposition::{
    adopt_recovered_candidate_cleanup_disposition, prepare_candidate_cleanup_disposition,
    store_candidate_cleanup_disposition, CandidateCleanupDispositionOutcomeUncertainCustody,
    CandidateCleanupDispositionPreparationFailure, CandidateCleanupDispositionRecoveryAdoption,
    CandidateCleanupDispositionRecoveryAdoptionFailure,
    CandidateCleanupDispositionRecoveryAdoptionPhase, CandidateCleanupDispositionStoreFailure,
    CandidateCleanupDispositionStorePhase, DurableCandidateCleanupDisposition,
    PreparedCandidateCleanupDisposition, ValidatedCandidateCleanupDispositionPermit,
};
pub(in crate::node_agent_compute_plugin_host) use disposition_recovery_key::CandidateCleanupDispositionRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use recovery_key::CandidateCleanupDeleteIntentRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use types::{
    restore_hashed_cleanup_step_event, validate_hashed_cleanup_step_event,
    ComputePluginCandidateCleanupStepEvent, HashedComputePluginCandidateCleanupStepEvent,
};

pub(in crate::node_agent_compute_plugin_host) use builder::{
    build_exact_handle_disposition_event, build_initial_delete_intent,
};

#[must_use = "prepared delete intent must be stored or retain the sealed topology"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupDeleteIntent<
    'authority,
> {
    pub(super) sealed: SealedCandidateCleanupTopology,
    pub(super) authority_session:
        ComputePluginCandidateCleanupDeleteIntentAuthoritySession<'authority>,
    pub(super) event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) prepared_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupDeleteIntentPermit<
    'permit,
> {
    prepared: &'permit PreparedCandidateCleanupDeleteIntent<'permit>,
}

#[must_use = "durable intent must be consumed by one exact physical step or retained"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupDeleteIntent {
    sealed: SealedCandidateCleanupTopology,
    event: HashedComputePluginCandidateCleanupStepEvent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeleteIntentPreparationFailure
{
    error: Error,
    sealed: SealedCandidateCleanupTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDeleteIntentStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain intent store retains every pinned object until recovery classification"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeleteIntentOutcomeUncertainCustody
{
    sealed: SealedCandidateCleanupTopology,
    recovery_key: CandidateCleanupDeleteIntentRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeleteIntentStoreFailure {
    phase: CandidateCleanupDeleteIntentStorePhase,
    error: Error,
    recovery: CandidateCleanupDeleteIntentOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDeleteIntentRecoveryAdoption {
    NotCreated(SealedCandidateCleanupTopology),
    Durable(DurableCandidateCleanupDeleteIntent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDeleteIntentRecoveryAdoptionPhase
{
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeleteIntentRecoveryAdoptionFailure
{
    phase: CandidateCleanupDeleteIntentRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupDeleteIntentOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_delete_intent<
    'authority,
>(
    sealed: SealedCandidateCleanupTopology,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> Result<
    PreparedCandidateCleanupDeleteIntent<'authority>,
    CandidateCleanupDeleteIntentPreparationFailure,
> {
    let prepared_at = Instant::now();
    let authority_session = match authority.bind_candidate_cleanup_delete_intent_authority_session(
        process_fence,
        observation,
        prepared_at,
    ) {
        Ok(session) => session,
        Err(error) => return Err(preparation_failure(error, sealed)),
    };
    let event = match build_initial_delete_intent(sealed.plan(), authority_session.trusted_now_ms())
    {
        Ok(event) => event,
        Err(error) => return Err(preparation_failure(error, sealed)),
    };
    if let Err(error) = authority_session.validate_candidate_cleanup_delete_intent(&sealed, &event)
    {
        return Err(preparation_failure(error, sealed));
    }
    Ok(PreparedCandidateCleanupDeleteIntent {
        sealed,
        authority_session,
        event,
        prepared_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_delete_intent(
    prepared: PreparedCandidateCleanupDeleteIntent<'_>,
) -> Result<DurableCandidateCleanupDeleteIntent, CandidateCleanupDeleteIntentStoreFailure> {
    let recovery_key = CandidateCleanupDeleteIntentRecoveryKey::from_prepared(&prepared);
    if let Err(error) = validate_hashed_cleanup_step_event(&prepared.event) {
        return Err(store_failure(
            CandidateCleanupDeleteIntentStorePhase::PreStorePreparation,
            error,
            prepared.sealed,
            recovery_key,
        ));
    }
    let stored = {
        let permit = ValidatedCandidateCleanupDeleteIntentPermit {
            prepared: &prepared,
        };
        prepared
            .authority_session
            .persist_candidate_cleanup_delete_intent(permit)
    };
    let event = match stored {
        Ok(event) => event,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupDeleteIntentStorePhase::StoreOutcomeUncertain,
                error,
                prepared.sealed,
                recovery_key,
            ))
        }
    };
    if event != prepared.event {
        return Err(store_failure(
            CandidateCleanupDeleteIntentStorePhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_POSTCONDITION_CHANGED"),
            prepared.sealed,
            recovery_key,
        ));
    }
    Ok(DurableCandidateCleanupDeleteIntent {
        sealed: prepared.sealed,
        event,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_delete_intent(
    recovery: CandidateCleanupDeleteIntentOutcomeUncertainCustody,
    observed: &ComputePluginCandidateCleanupDeleteIntentRecoveryOutcome,
    session: ComputePluginCandidateCleanupDeleteIntentRecoveryAuthoritySession<'_>,
) -> Result<
    CandidateCleanupDeleteIntentRecoveryAdoption,
    CandidateCleanupDeleteIntentRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &session) {
        return Err(adoption_failure(
            CandidateCleanupDeleteIntentRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match session.read_candidate_cleanup_delete_intent_outcome(&recovery.recovery_key) {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateCleanupDeleteIntentRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateCleanupDeleteIntentRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = session.validate_source(recovery.sealed.state().cancellation_guard()) {
        return Err(adoption_failure(
            CandidateCleanupDeleteIntentRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupDeleteIntentRecoveryOutcome::NotCreated => Ok(
            CandidateCleanupDeleteIntentRecoveryAdoption::NotCreated(recovery.sealed),
        ),
        ComputePluginCandidateCleanupDeleteIntentRecoveryOutcome::Durable(event) => {
            if event != *recovery.recovery_key.event() {
                return Err(adoption_failure(
                    CandidateCleanupDeleteIntentRecoveryAdoptionPhase::OutcomeChanged,
                    anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_EVENT_CHANGED"),
                    recovery,
                ));
            }
            Ok(CandidateCleanupDeleteIntentRecoveryAdoption::Durable(
                DurableCandidateCleanupDeleteIntent {
                    sealed: recovery.sealed,
                    event,
                },
            ))
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupDeleteIntentOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupDeleteIntentRecoveryAuthoritySession<'_>,
) -> anyhow::Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || session.observed_at() <= key.prepared_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(recovery.sealed.state().cancellation_guard())
}

fn preparation_failure(
    error: Error,
    sealed: SealedCandidateCleanupTopology,
) -> CandidateCleanupDeleteIntentPreparationFailure {
    CandidateCleanupDeleteIntentPreparationFailure { error, sealed }
}

fn store_failure(
    phase: CandidateCleanupDeleteIntentStorePhase,
    error: Error,
    sealed: SealedCandidateCleanupTopology,
    recovery_key: CandidateCleanupDeleteIntentRecoveryKey,
) -> CandidateCleanupDeleteIntentStoreFailure {
    CandidateCleanupDeleteIntentStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupDeleteIntentOutcomeUncertainCustody {
            sealed,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupDeleteIntentRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupDeleteIntentOutcomeUncertainCustody,
) -> CandidateCleanupDeleteIntentRecoveryAdoptionFailure {
    CandidateCleanupDeleteIntentRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl ValidatedCandidateCleanupDeleteIntentPermit<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn sealed(
        &self,
    ) -> &SealedCandidateCleanupTopology {
        &self.prepared.sealed
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.prepared.event
    }
}

impl DurableCandidateCleanupDeleteIntent {
    pub(super) fn sealed(&self) -> &SealedCandidateCleanupTopology {
        &self.sealed
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.event
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        SealedCandidateCleanupTopology,
        HashedComputePluginCandidateCleanupStepEvent,
    ) {
        (self.sealed, self.event)
    }
}

impl CandidateCleanupDeleteIntentPreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, SealedCandidateCleanupTopology) {
        (self.error, self.sealed)
    }
}

impl CandidateCleanupDeleteIntentStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupDeleteIntentStorePhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupDeleteIntentOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupDeleteIntentOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupDeleteIntentRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupDeleteIntentRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupDeleteIntentRecoveryAdoptionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupDeleteIntentOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

macro_rules! impl_failure {
    ($failure:ident) => {
        impl fmt::Display for $failure {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:#}", self.error)
            }
        }
        impl fmt::Debug for $failure {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($failure))
                    .field("error", &self.error)
                    .finish_non_exhaustive()
            }
        }
        impl StdError for $failure {}
    };
}

impl_failure!(CandidateCleanupDeleteIntentPreparationFailure);
impl_failure!(CandidateCleanupDeleteIntentStoreFailure);
impl_failure!(CandidateCleanupDeleteIntentRecoveryAdoptionFailure);
