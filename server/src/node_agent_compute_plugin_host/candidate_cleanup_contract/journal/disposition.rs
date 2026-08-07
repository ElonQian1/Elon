use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error};

use super::{
    build_exact_handle_disposition_event, validate_hashed_cleanup_step_event,
    CandidateCleanupDispositionRecoveryKey, HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::PhysicallyDisposedCandidateCleanupObject,
    local_authority::{
        ComputePluginCandidateCleanupDispositionAuthoritySession,
        ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession,
        ComputePluginCandidateCleanupDispositionRecoveryOutcome, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[must_use = "prepared disposition event must be stored or retain physical custody"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupDisposition<'authority>
{
    pub(super) physical: PhysicallyDisposedCandidateCleanupObject,
    pub(super) authority_session:
        ComputePluginCandidateCleanupDispositionAuthoritySession<'authority>,
    pub(super) event: HashedComputePluginCandidateCleanupStepEvent,
    pub(super) prepared_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupDispositionPermit<
    'permit,
> {
    prepared: &'permit PreparedCandidateCleanupDisposition<'permit>,
}

#[must_use = "durable disposition must proceed to parent-relative absence or remain retained"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupDisposition {
    physical: PhysicallyDisposedCandidateCleanupObject,
    event: HashedComputePluginCandidateCleanupStepEvent,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionPreparationFailure {
    error: Error,
    physical: PhysicallyDisposedCandidateCleanupObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDispositionStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain disposition store retains the physical capability until classified"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionOutcomeUncertainCustody
{
    physical: PhysicallyDisposedCandidateCleanupObject,
    recovery_key: CandidateCleanupDispositionRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionStoreFailure {
    phase: CandidateCleanupDispositionStorePhase,
    error: Error,
    recovery: CandidateCleanupDispositionOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDispositionRecoveryAdoption {
    NotCreated(PhysicallyDisposedCandidateCleanupObject),
    Durable(DurableCandidateCleanupDisposition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDispositionRecoveryAdoptionPhase
{
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionRecoveryAdoptionFailure
{
    phase: CandidateCleanupDispositionRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupDispositionOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_disposition<
    'authority,
>(
    physical: PhysicallyDisposedCandidateCleanupObject,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> Result<
    PreparedCandidateCleanupDisposition<'authority>,
    CandidateCleanupDispositionPreparationFailure,
> {
    let prepared_at = Instant::now();
    let authority_session = match authority.bind_candidate_cleanup_disposition_authority_session(
        process_fence,
        observation,
        &physical,
        prepared_at,
    ) {
        Ok(session) => session,
        Err(error) => return Err(preparation_failure(error, physical)),
    };
    let event = match build_exact_handle_disposition_event(
        physical.plan(),
        physical.intent_event(),
        physical.object_binding(),
        authority_session.trusted_now_ms(),
    ) {
        Ok(event) => event,
        Err(error) => return Err(preparation_failure(error, physical)),
    };
    if let Err(error) = authority_session.validate_candidate_cleanup_disposition(&physical, &event)
    {
        return Err(preparation_failure(error, physical));
    }
    Ok(PreparedCandidateCleanupDisposition {
        physical,
        authority_session,
        event,
        prepared_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_disposition(
    prepared: PreparedCandidateCleanupDisposition<'_>,
) -> Result<DurableCandidateCleanupDisposition, CandidateCleanupDispositionStoreFailure> {
    let recovery_key = CandidateCleanupDispositionRecoveryKey::from_prepared(&prepared);
    if let Err(error) = validate_hashed_cleanup_step_event(&prepared.event) {
        return Err(store_failure(
            CandidateCleanupDispositionStorePhase::PreStorePreparation,
            error,
            prepared.physical,
            recovery_key,
        ));
    }
    let stored = {
        let permit = ValidatedCandidateCleanupDispositionPermit {
            prepared: &prepared,
        };
        prepared
            .authority_session
            .persist_candidate_cleanup_disposition(permit)
    };
    let event = match stored {
        Ok(event) => event,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupDispositionStorePhase::StoreOutcomeUncertain,
                error,
                prepared.physical,
                recovery_key,
            ))
        }
    };
    if event != prepared.event {
        return Err(store_failure(
            CandidateCleanupDispositionStorePhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_POSTCONDITION_CHANGED"),
            prepared.physical,
            recovery_key,
        ));
    }
    Ok(DurableCandidateCleanupDisposition {
        physical: prepared.physical,
        event,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_disposition(
    recovery: CandidateCleanupDispositionOutcomeUncertainCustody,
    observed: &ComputePluginCandidateCleanupDispositionRecoveryOutcome,
    session: ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_>,
) -> Result<
    CandidateCleanupDispositionRecoveryAdoption,
    CandidateCleanupDispositionRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &session) {
        return Err(adoption_failure(
            CandidateCleanupDispositionRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match session.read_candidate_cleanup_disposition_outcome(&recovery.recovery_key) {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateCleanupDispositionRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateCleanupDispositionRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = session.validate_source(recovery.physical.state().cancellation_guard()) {
        return Err(adoption_failure(
            CandidateCleanupDispositionRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupDispositionRecoveryOutcome::NotCreated => Ok(
            CandidateCleanupDispositionRecoveryAdoption::NotCreated(recovery.physical),
        ),
        ComputePluginCandidateCleanupDispositionRecoveryOutcome::Durable(event) => {
            if event != *recovery.recovery_key.disposition_event() {
                return Err(adoption_failure(
                    CandidateCleanupDispositionRecoveryAdoptionPhase::OutcomeChanged,
                    anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_EVENT_CHANGED"),
                    recovery,
                ));
            }
            Ok(CandidateCleanupDispositionRecoveryAdoption::Durable(
                DurableCandidateCleanupDisposition {
                    physical: recovery.physical,
                    event,
                },
            ))
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupDispositionOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_>,
) -> anyhow::Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || session.observed_at() <= key.prepared_at()
        || session.observed_at() <= recovery.physical.disposition_set_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(recovery.physical.state().cancellation_guard())
}

fn preparation_failure(
    error: Error,
    physical: PhysicallyDisposedCandidateCleanupObject,
) -> CandidateCleanupDispositionPreparationFailure {
    CandidateCleanupDispositionPreparationFailure { error, physical }
}

fn store_failure(
    phase: CandidateCleanupDispositionStorePhase,
    error: Error,
    physical: PhysicallyDisposedCandidateCleanupObject,
    recovery_key: CandidateCleanupDispositionRecoveryKey,
) -> CandidateCleanupDispositionStoreFailure {
    CandidateCleanupDispositionStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupDispositionOutcomeUncertainCustody {
            physical,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupDispositionRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupDispositionOutcomeUncertainCustody,
) -> CandidateCleanupDispositionRecoveryAdoptionFailure {
    CandidateCleanupDispositionRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl ValidatedCandidateCleanupDispositionPermit<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn physical(
        &self,
    ) -> &PhysicallyDisposedCandidateCleanupObject {
        &self.prepared.physical
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.prepared.event
    }
}

impl DurableCandidateCleanupDisposition {
    pub(in crate::node_agent_compute_plugin_host) fn physical(
        &self,
    ) -> &PhysicallyDisposedCandidateCleanupObject {
        &self.physical
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.event
    }
}

impl CandidateCleanupDispositionPreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, PhysicallyDisposedCandidateCleanupObject) {
        (self.error, self.physical)
    }
}

impl CandidateCleanupDispositionStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupDispositionStorePhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupDispositionOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupDispositionOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupDispositionRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupDispositionRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupDispositionRecoveryAdoptionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupDispositionOutcomeUncertainCustody) {
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

impl_failure!(CandidateCleanupDispositionPreparationFailure);
impl_failure!(CandidateCleanupDispositionStoreFailure);
impl_failure!(CandidateCleanupDispositionRecoveryAdoptionFailure);
