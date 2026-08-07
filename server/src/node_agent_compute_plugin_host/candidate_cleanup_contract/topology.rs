use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::{execution::prepare_candidate_cleanup_execution_state, AuthorizedCandidateCleanup};
use crate::node_agent_compute_plugin_host::{
    local_authority::{
        ComputePluginCandidateCleanupTopologyAuthoritySession,
        ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession,
        ComputePluginCandidateCleanupTopologyRecoveryOutcome, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) mod builder;
mod recovery_key;
mod types;

pub(in crate::node_agent_compute_plugin_host) use recovery_key::CandidateCleanupTopologyRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use types::{
    restore_hashed_execution_plan, restore_hashed_expected_object, validate_hashed_execution_plan,
    ComputePluginCandidateCleanupExecutionPlan, ComputePluginCandidateCleanupExpectedObject,
    HashedCandidateCleanupExpectedObject, HashedComputePluginCandidateCleanupExecutionPlan,
};

use builder::{build_execution_plan, CandidateCleanupTopologyPlanInput};

#[must_use = "prepared cleanup topology must be stored or retained with all handles"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupTopology<'authority> {
    pub(super) state: super::CandidateCleanupExecutionState,
    pub(super) authority_session: ComputePluginCandidateCleanupTopologyAuthoritySession<'authority>,
    pub(super) plan: HashedComputePluginCandidateCleanupExecutionPlan,
    pub(super) prepared_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupTopologyPermit<
    'permit,
> {
    prepared: &'permit PreparedCandidateCleanupTopology<'permit>,
}

#[must_use = "sealed topology must be consumed by physical cleanup or retained for recovery"]
pub(in crate::node_agent_compute_plugin_host) struct SealedCandidateCleanupTopology {
    state: super::CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupTopologyPreparationCustody<'root>
{
    Authorized(AuthorizedCandidateCleanup<'root>),
    Pinned(super::CandidateCleanupExecutionState),
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyPreparationFailure<
    'root,
> {
    error: Error,
    custody: CandidateCleanupTopologyPreparationCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupPinnedTopologyPreparationFailure
{
    error: Error,
    state: super::CandidateCleanupExecutionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupTopologyStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain topology store retains every pinned object until recovery classification"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyOutcomeUncertainCustody
{
    state: super::CandidateCleanupExecutionState,
    recovery_key: CandidateCleanupTopologyRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyStoreFailure {
    phase: CandidateCleanupTopologyStorePhase,
    error: Error,
    recovery: CandidateCleanupTopologyOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupTopologyRecoveryAdoption {
    NotCreated(super::CandidateCleanupExecutionState),
    Sealed(SealedCandidateCleanupTopology),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupTopologyRecoveryAdoptionPhase {
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupTopologyRecoveryAdoptionFailure
{
    phase: CandidateCleanupTopologyRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupTopologyOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_topology<
    'root,
    'authority,
>(
    authorized: AuthorizedCandidateCleanup<'root>,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    PreparedCandidateCleanupTopology<'authority>,
    CandidateCleanupTopologyPreparationFailure<'root>,
> {
    let state = match prepare_candidate_cleanup_execution_state(authorized) {
        Ok(state) => state,
        Err((error, authorized)) => {
            return Err(CandidateCleanupTopologyPreparationFailure {
                error,
                custody: CandidateCleanupTopologyPreparationCustody::Authorized(authorized),
            })
        }
    };
    prepare_pinned_candidate_cleanup_topology(state, authority, process_fence, observation).map_err(
        |failure| CandidateCleanupTopologyPreparationFailure {
            error: failure.error,
            custody: CandidateCleanupTopologyPreparationCustody::Pinned(failure.state),
        },
    )
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_pinned_candidate_cleanup_topology<
    'authority,
>(
    state: super::CandidateCleanupExecutionState,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    PreparedCandidateCleanupTopology<'authority>,
    CandidateCleanupPinnedTopologyPreparationFailure,
> {
    let prepared_at = Instant::now();
    let authority_session = match authority.bind_candidate_cleanup_topology_authority_session(
        process_fence,
        observation,
        prepared_at,
    ) {
        Ok(session) => session,
        Err(error) => return Err(pinned_preparation_failure(error, state)),
    };
    let candidate_parent_anchor_identity_digest =
        match state.candidate_parent_anchor_identity_digest() {
            Ok(digest) => digest.to_string(),
            Err(error) => return Err(pinned_preparation_failure(error, state)),
        };
    let objects = match state.topology_objects() {
        Ok(objects) => objects,
        Err(error) => return Err(pinned_preparation_failure(error, state)),
    };
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let recovery = state.staging_recovery_key();
    let plan = match build_execution_plan(CandidateCleanupTopologyPlanInput {
        cleanup_id: receipt.cleanup_id().to_string(),
        candidate_token_digest: receipt.candidate_token_digest().to_string(),
        authorization_receipt_digest: authorization.receipt_digest().to_string(),
        installation_id_digest: authority_session.installation_id_digest().to_string(),
        root_identity_digest: recovery.root_identity_digest().to_string(),
        candidate_parent_anchor_identity_digest,
        process_owner_epoch: authority_session.process_owner_epoch(),
        planned_at_ms: authority_session.trusted_now_ms(),
        objects,
    }) {
        Ok(plan) => plan,
        Err(error) => return Err(pinned_preparation_failure(error, state)),
    };
    if let Err(error) = authority_session.validate_candidate_cleanup_topology(&state, &plan) {
        return Err(pinned_preparation_failure(error, state));
    }
    Ok(PreparedCandidateCleanupTopology {
        state,
        authority_session,
        plan,
        prepared_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_topology(
    prepared: PreparedCandidateCleanupTopology<'_>,
) -> std::result::Result<SealedCandidateCleanupTopology, CandidateCleanupTopologyStoreFailure> {
    let recovery_key = CandidateCleanupTopologyRecoveryKey::from_prepared(&prepared);
    if let Err(error) = validate_hashed_execution_plan(&prepared.plan) {
        return Err(topology_store_failure(
            CandidateCleanupTopologyStorePhase::PreStorePreparation,
            error,
            prepared.state,
            recovery_key,
        ));
    }
    let stored = {
        let permit = ValidatedCandidateCleanupTopologyPermit {
            prepared: &prepared,
        };
        prepared
            .authority_session
            .persist_candidate_cleanup_topology(permit)
    };
    let plan = match stored {
        Ok(plan) => plan,
        Err(error) => {
            return Err(topology_store_failure(
                CandidateCleanupTopologyStorePhase::StoreOutcomeUncertain,
                error,
                prepared.state,
                recovery_key,
            ))
        }
    };
    if plan != prepared.plan {
        return Err(topology_store_failure(
            CandidateCleanupTopologyStorePhase::StoreReturnedPostconditionFailed,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_POSTCONDITION_CHANGED"),
            prepared.state,
            recovery_key,
        ));
    }
    Ok(SealedCandidateCleanupTopology {
        state: prepared.state,
        plan,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_topology(
    recovery: CandidateCleanupTopologyOutcomeUncertainCustody,
    observed: &ComputePluginCandidateCleanupTopologyRecoveryOutcome,
    session: ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    CandidateCleanupTopologyRecoveryAdoption,
    CandidateCleanupTopologyRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &session) {
        return Err(adoption_failure(
            CandidateCleanupTopologyRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh = match session.read_candidate_cleanup_topology_outcome(&recovery.recovery_key) {
        Ok(fresh) => fresh,
        Err(error) => {
            return Err(adoption_failure(
                CandidateCleanupTopologyRecoveryAdoptionPhase::OutcomeReadFailed,
                error,
                recovery,
            ))
        }
    };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateCleanupTopologyRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) = session.validate_source(recovery.state.cancellation_guard()) {
        return Err(adoption_failure(
            CandidateCleanupTopologyRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupTopologyRecoveryOutcome::NotCreated => Ok(
            CandidateCleanupTopologyRecoveryAdoption::NotCreated(recovery.state),
        ),
        ComputePluginCandidateCleanupTopologyRecoveryOutcome::Sealed(plan) => {
            if plan != *recovery.recovery_key.plan() {
                return Err(adoption_failure(
                    CandidateCleanupTopologyRecoveryAdoptionPhase::OutcomeChanged,
                    anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_CHANGED"),
                    recovery,
                ));
            }
            Ok(CandidateCleanupTopologyRecoveryAdoption::Sealed(
                SealedCandidateCleanupTopology {
                    state: recovery.state,
                    plan,
                },
            ))
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupTopologyOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || session.observed_at() <= key.prepared_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(recovery.state.cancellation_guard())
}

fn pinned_preparation_failure(
    error: Error,
    state: super::CandidateCleanupExecutionState,
) -> CandidateCleanupPinnedTopologyPreparationFailure {
    CandidateCleanupPinnedTopologyPreparationFailure { error, state }
}

fn topology_store_failure(
    phase: CandidateCleanupTopologyStorePhase,
    error: Error,
    state: super::CandidateCleanupExecutionState,
    recovery_key: CandidateCleanupTopologyRecoveryKey,
) -> CandidateCleanupTopologyStoreFailure {
    CandidateCleanupTopologyStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupTopologyOutcomeUncertainCustody {
            state,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupTopologyRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupTopologyOutcomeUncertainCustody,
) -> CandidateCleanupTopologyRecoveryAdoptionFailure {
    CandidateCleanupTopologyRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl ValidatedCandidateCleanupTopologyPermit<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn state(
        &self,
    ) -> &super::CandidateCleanupExecutionState {
        &self.prepared.state
    }
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionPlan {
        &self.prepared.plan
    }
}

impl SealedCandidateCleanupTopology {
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionPlan {
        &self.plan
    }
    pub(super) fn into_parts(
        self,
    ) -> (
        super::CandidateCleanupExecutionState,
        HashedComputePluginCandidateCleanupExecutionPlan,
    ) {
        (self.state, self.plan)
    }
}

impl<'root> CandidateCleanupTopologyPreparationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupTopologyPreparationCustody<'root>) {
        (self.error, self.custody)
    }
}

impl CandidateCleanupPinnedTopologyPreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, super::CandidateCleanupExecutionState) {
        (self.error, self.state)
    }
}

impl CandidateCleanupTopologyStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupTopologyStorePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupTopologyOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupTopologyOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupTopologyRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupTopologyRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupTopologyRecoveryAdoptionPhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupTopologyOutcomeUncertainCustody) {
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

impl fmt::Display for CandidateCleanupTopologyPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}
impl fmt::Debug for CandidateCleanupTopologyPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupTopologyPreparationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl StdError for CandidateCleanupTopologyPreparationFailure<'_> {}
impl_failure!(CandidateCleanupTopologyStoreFailure);
impl_failure!(CandidateCleanupPinnedTopologyPreparationFailure);
impl_failure!(CandidateCleanupTopologyRecoveryAdoptionFailure);
