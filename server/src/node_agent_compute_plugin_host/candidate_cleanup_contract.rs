use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};
use uuid::Uuid;

use super::{
    candidate_health_contract::DurableCandidateHealthQuarantine,
    install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_FAILED,
    local_authority::{
        ComputePluginCandidateCleanupAuthorityFacts, ComputePluginCandidateCleanupAuthoritySession,
        ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod adoption;
mod completion;
mod completion_recovery_key;
mod execution;
mod recovery_key;
mod terminal_journal;
mod topology;

pub(in crate::node_agent_compute_plugin_host) use adoption::{
    adopt_recovered_candidate_cleanup_authorization,
    CandidateCleanupAuthorizationRecoveryAdoptionFailure,
    CandidateCleanupAuthorizationRecoveryAdoptionPhase,
};
pub(in crate::node_agent_compute_plugin_host) use completion::{
    adopt_recovered_candidate_cleanup_completion, prepare_candidate_cleanup_completion,
    store_candidate_cleanup_completion, CandidateCleanupCompletionOutcomeUncertainCustody,
    CandidateCleanupCompletionPreparationFailure, CandidateCleanupCompletionRecoveryAdoption,
    CandidateCleanupCompletionRecoveryAdoptionFailure,
    CandidateCleanupCompletionRecoveryAdoptionPhase, CandidateCleanupCompletionStoreFailure,
    CandidateCleanupCompletionStorePhase, DurableCandidateCleanupCompletion,
    PreparedCandidateCleanupCompletion, ValidatedCandidateCleanupCompletionPermit,
};
pub(in crate::node_agent_compute_plugin_host) use completion_recovery_key::{
    CandidateCleanupCompletionReceiptExpectation, CandidateCleanupCompletionRecoveryKey,
};
pub(in crate::node_agent_compute_plugin_host) use execution::validate_hashed_execution_evidence;
pub(in crate::node_agent_compute_plugin_host) use execution::{
    prepare_candidate_cleanup_execution, resume_candidate_cleanup_execution,
    CandidateCleanupExecutionFailure, CandidateCleanupExecutionState,
    ComputePluginCandidateCleanupExecutionEvidence,
    HashedComputePluginCandidateCleanupExecutionEvidence, PhysicallyExecutedCandidateCleanup,
};
pub(in crate::node_agent_compute_plugin_host) use recovery_key::{
    CandidateCleanupAuthorizationReceiptExpectation, CandidateCleanupAuthorizationRecoveryKey,
    CandidateCleanupSlotExpectation,
};
pub(in crate::node_agent_compute_plugin_host) use terminal_journal::DurableCandidateCleanupTerminalJournal;
pub(in crate::node_agent_compute_plugin_host) use topology::{
    adopt_recovered_candidate_cleanup_topology, prepare_candidate_cleanup_topology,
    prepare_pinned_candidate_cleanup_topology, restore_hashed_execution_plan,
    restore_hashed_expected_object, store_candidate_cleanup_topology,
    validate_hashed_execution_plan, CandidateCleanupPinnedTopologyPreparationFailure,
    CandidateCleanupTopologyOutcomeUncertainCustody, CandidateCleanupTopologyPreparationCustody,
    CandidateCleanupTopologyPreparationFailure, CandidateCleanupTopologyRecoveryAdoption,
    CandidateCleanupTopologyRecoveryAdoptionFailure, CandidateCleanupTopologyRecoveryAdoptionPhase,
    CandidateCleanupTopologyRecoveryKey, CandidateCleanupTopologyStoreFailure,
    CandidateCleanupTopologyStorePhase, ComputePluginCandidateCleanupExecutionPlan,
    ComputePluginCandidateCleanupExpectedObject, HashedCandidateCleanupExpectedObject,
    HashedComputePluginCandidateCleanupExecutionPlan, PreparedCandidateCleanupTopology,
    SealedCandidateCleanupTopology, ValidatedCandidateCleanupTopologyPermit,
};

#[must_use = "prepared cleanup authorization must be stored or returned with candidate custody"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupAuthorization<
    'root,
    'authority,
> {
    quarantined: DurableCandidateHealthQuarantine<'root>,
    authority_session: ComputePluginCandidateCleanupAuthoritySession<'authority>,
    facts: ComputePluginCandidateCleanupAuthorityFacts,
    cleanup_id: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupAuthorizationPermit<
    'permit,
    'root,
> {
    prepared: &'permit PreparedCandidateCleanupAuthorization<'root, 'permit>,
}

#[must_use = "authorized cleanup must be executed or retained for recovery"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedCandidateCleanup<'root> {
    quarantined: DurableCandidateHealthQuarantine<'root>,
    receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationPreparationFailure<
    'root,
> {
    error: Error,
    quarantined: DurableCandidateHealthQuarantine<'root>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupAuthorizationStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain cleanup authorization must be inspected through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationOutcomeUncertainCustody<
    'root,
> {
    quarantined: DurableCandidateHealthQuarantine<'root>,
    recovery_key: CandidateCleanupAuthorizationRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupAuthorizationStoreFailure<
    'root,
> {
    phase: CandidateCleanupAuthorizationStorePhase,
    error: Error,
    recovery: CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_authorization<
    'root,
    'authority,
>(
    mut quarantined: DurableCandidateHealthQuarantine<'root>,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    PreparedCandidateCleanupAuthorization<'root, 'authority>,
    CandidateCleanupAuthorizationPreparationFailure<'root>,
> {
    if let Err(error) = quarantined.revalidate_retained_content() {
        return Err(CandidateCleanupAuthorizationPreparationFailure { error, quarantined });
    }
    let authority_session =
        match authority.bind_candidate_cleanup_authority_session(process_fence, observation) {
            Ok(session) => session,
            Err(error) => {
                return Err(CandidateCleanupAuthorizationPreparationFailure { error, quarantined })
            }
        };
    let facts = match authority_session.read_candidate_cleanup_binding(&quarantined) {
        Ok(facts) => facts,
        Err(error) => {
            return Err(CandidateCleanupAuthorizationPreparationFailure { error, quarantined })
        }
    };
    let cleanup_id = format!("cca_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    if !is_identifier(&cleanup_id) {
        return Err(CandidateCleanupAuthorizationPreparationFailure {
            error: anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ID_INVALID"),
            quarantined,
        });
    }
    Ok(PreparedCandidateCleanupAuthorization {
        quarantined,
        authority_session,
        facts,
        cleanup_id,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_authorization<'root>(
    prepared: PreparedCandidateCleanupAuthorization<'root, '_>,
) -> std::result::Result<
    AuthorizedCandidateCleanup<'root>,
    CandidateCleanupAuthorizationStoreFailure<'root>,
> {
    let recovery_key = CandidateCleanupAuthorizationRecoveryKey::from_prepared(&prepared);
    if !is_identifier(prepared.cleanup_id.as_str()) {
        return Err(store_failure(
            CandidateCleanupAuthorizationStorePhase::PreStorePreparation,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ID_INVALID"),
            prepared.quarantined,
            recovery_key,
        ));
    }
    let result = {
        let permit = ValidatedCandidateCleanupAuthorizationPermit::new(&prepared);
        prepared
            .authority_session
            .persist_candidate_cleanup_authorization(permit)
    };
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupAuthorizationStorePhase::StoreOutcomeUncertain,
                error,
                prepared.quarantined,
                recovery_key,
            ))
        }
    };
    if let Err(error) = validate_cleanup_authorization_receipt(&recovery_key, &receipt) {
        return Err(store_failure(
            CandidateCleanupAuthorizationStorePhase::StoreReturnedPostconditionFailed,
            error,
            prepared.quarantined,
            recovery_key,
        ));
    }
    Ok(AuthorizedCandidateCleanup {
        quarantined: prepared.quarantined,
        receipt,
    })
}

pub(super) fn validate_cleanup_authorization_receipt(
    key: &CandidateCleanupAuthorizationRecoveryKey,
    hashed: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let expected = key.receipt_expectation();
    if hashed.schema() != HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA
        || receipt.schema() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA
        || hashed.canonicalization() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION
        || hashed.digest_algorithm() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM
        || receipt.cleanup_id() != key.cleanup_id()
        || receipt.candidate_token_digest() != expected.candidate_token_digest
        || receipt.quarantine_id() != expected.quarantine_id
        || receipt.quarantine_receipt_digest() != expected.quarantine_receipt_digest
        || receipt.staging_id() != expected.staging_id
        || receipt.staging_run_digest() != expected.staging_run_digest
        || receipt.authority_state_revision_before() != expected.authority_state_revision_before
        || receipt.authority_state_revision_after() != expected.authority_state_revision_after
        || receipt.inventory_revision() != expected.inventory_revision
        || receipt.inventory_digest() != expected.inventory_digest
        || receipt.authority_epoch_before() != expected.authority_epoch_before
        || receipt.authority_epoch_after() != expected.authority_epoch_after
        || receipt.process_owner_epoch() != expected.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before() != expected.trusted_time_high_water_ms_before
        || receipt.authorized_at_ms() != expected.authorized_at_ms
        || receipt.slot_phase_before() != SLOT_FAILED
        || !is_sha256(hashed.receipt_digest())
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORIZATION_POSTCONDITION_CHANGED");
    }
    Ok(())
}

fn store_failure<'root>(
    phase: CandidateCleanupAuthorizationStorePhase,
    error: Error,
    quarantined: DurableCandidateHealthQuarantine<'root>,
    recovery_key: CandidateCleanupAuthorizationRecoveryKey,
) -> CandidateCleanupAuthorizationStoreFailure<'root> {
    CandidateCleanupAuthorizationStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupAuthorizationOutcomeUncertainCustody {
            quarantined,
            recovery_key,
        },
    }
}

impl<'permit, 'root> ValidatedCandidateCleanupAuthorizationPermit<'permit, 'root> {
    fn new(prepared: &'permit PreparedCandidateCleanupAuthorization<'root, 'permit>) -> Self {
        Self { prepared }
    }
    pub(in crate::node_agent_compute_plugin_host) fn quarantined(
        &self,
    ) -> &DurableCandidateHealthQuarantine<'root> {
        &self.prepared.quarantined
    }
    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginCandidateCleanupAuthorityFacts {
        &self.prepared.facts
    }
    pub(in crate::node_agent_compute_plugin_host) fn cleanup_id(&self) -> &str {
        &self.prepared.cleanup_id
    }
}

impl AuthorizedCandidateCleanup<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn quarantined(
        &self,
    ) -> &DurableCandidateHealthQuarantine<'_> {
        &self.quarantined
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupAuthorizationReceipt {
        &self.receipt
    }
}

impl<'root> AuthorizedCandidateCleanup<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        DurableCandidateHealthQuarantine<'root>,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
    ) {
        (self.quarantined, self.receipt)
    }
}

impl CandidateCleanupAuthorizationStoreFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupAuthorizationStorePhase {
        self.phase
    }
}

impl<'root> CandidateCleanupAuthorizationStoreFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateCleanupAuthorizationOutcomeUncertainCustody<'root>,
    ) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupAuthorizationOutcomeUncertainCustody<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupAuthorizationRecoveryKey {
        &self.recovery_key
    }
}

impl<'root> CandidateCleanupAuthorizationPreparationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, DurableCandidateHealthQuarantine<'root>) {
        (self.error, self.quarantined)
    }
}

macro_rules! impl_failure {
    ($failure:ident) => {
        impl fmt::Display for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:#}", self.error)
            }
        }
        impl fmt::Debug for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($failure))
                    .field("error", &self.error)
                    .finish_non_exhaustive()
            }
        }
        impl StdError for $failure<'_> {}
    };
}

impl_failure!(CandidateCleanupAuthorizationPreparationFailure);
impl_failure!(CandidateCleanupAuthorizationStoreFailure);

impl fmt::Debug for CandidateCleanupAuthorizationOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupAuthorizationOutcomeUncertainCustody")
            .field("recovery_key", &self.recovery_key)
            .field("quarantined", &"<retained-handles>")
            .finish()
    }
}

impl fmt::Debug for AuthorizedCandidateCleanup<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedCandidateCleanup")
            .field("receipt", &self.receipt)
            .field("quarantined", &"<retained-handles>")
            .finish()
    }
}
