use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};
use uuid::Uuid;

use super::{CandidateCleanupCompletionRecoveryKey, DurableCandidateCleanupTerminalJournal};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    local_authority::{
        ComputePluginCandidateCleanupCompletionAuthorityFacts,
        ComputePluginCandidateCleanupCompletionAuthoritySession,
        ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession,
        ComputePluginCandidateCleanupCompletionRecoveryOutcome, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority, HashedComputePluginCandidateCleanupCompletionReceipt,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[must_use = "prepared cleanup completion must be stored or returned with physical custody"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupCompletion<'authority>
{
    terminal: DurableCandidateCleanupTerminalJournal,
    authority_session: ComputePluginCandidateCleanupCompletionAuthoritySession<'authority>,
    facts: ComputePluginCandidateCleanupCompletionAuthorityFacts,
    completion_id: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateCleanupCompletionPermit<
    'permit,
    'authority,
> {
    prepared: &'permit PreparedCandidateCleanupCompletion<'authority>,
}

#[must_use = "durable cleanup completion retains the root lock until Host adopts final state"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupCompletion {
    terminal: DurableCandidateCleanupTerminalJournal,
    receipt: HashedComputePluginCandidateCleanupCompletionReceipt,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionPreparationFailure {
    error: Error,
    terminal: DurableCandidateCleanupTerminalJournal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupCompletionStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain completion must be classified through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionOutcomeUncertainCustody
{
    terminal: DurableCandidateCleanupTerminalJournal,
    recovery_key: CandidateCleanupCompletionRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionStoreFailure {
    phase: CandidateCleanupCompletionStorePhase,
    error: Error,
    recovery: CandidateCleanupCompletionOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupCompletionRecoveryAdoption {
    NotCreated(DurableCandidateCleanupTerminalJournal),
    Completed(DurableCandidateCleanupCompletion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupCompletionRecoveryAdoptionPhase {
    RejectedBeforeOutcomeRead,
    OutcomeReadFailed,
    OutcomeChanged,
    RetainedCustodyChanged,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupCompletionRecoveryAdoptionFailure
{
    phase: CandidateCleanupCompletionRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupCompletionOutcomeUncertainCustody,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_completion<
    'authority,
>(
    terminal: DurableCandidateCleanupTerminalJournal,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    PreparedCandidateCleanupCompletion<'authority>,
    CandidateCleanupCompletionPreparationFailure,
> {
    let authority_session = match authority.bind_candidate_cleanup_completion_authority_session(
        process_fence,
        observation,
        terminal.physical().physical_completed_at(),
    ) {
        Ok(session) => session,
        Err(error) => return Err(CandidateCleanupCompletionPreparationFailure { error, terminal }),
    };
    let facts = match authority_session.read_candidate_cleanup_completion_binding(&terminal) {
        Ok(facts) => facts,
        Err(error) => return Err(CandidateCleanupCompletionPreparationFailure { error, terminal }),
    };
    let completion_id = format!("ccc_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    if !is_identifier(&completion_id) {
        return Err(CandidateCleanupCompletionPreparationFailure {
            error: anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_ID_INVALID"),
            terminal,
        });
    }
    Ok(PreparedCandidateCleanupCompletion {
        terminal,
        authority_session,
        facts,
        completion_id,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn store_candidate_cleanup_completion(
    prepared: PreparedCandidateCleanupCompletion<'_>,
) -> std::result::Result<DurableCandidateCleanupCompletion, CandidateCleanupCompletionStoreFailure>
{
    let recovery_key = CandidateCleanupCompletionRecoveryKey::from_prepared(&prepared);
    if !is_identifier(prepared.completion_id()) {
        return Err(store_failure(
            CandidateCleanupCompletionStorePhase::PreStorePreparation,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_ID_INVALID"),
            prepared.terminal,
            recovery_key,
        ));
    }
    let result = {
        let permit = ValidatedCandidateCleanupCompletionPermit::new(&prepared);
        prepared
            .authority_session
            .persist_candidate_cleanup_completion(permit)
    };
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(store_failure(
                CandidateCleanupCompletionStorePhase::StoreOutcomeUncertain,
                error,
                prepared.terminal,
                recovery_key,
            ))
        }
    };
    if let Err(error) = validate_cleanup_completion_receipt(&recovery_key, &receipt) {
        return Err(store_failure(
            CandidateCleanupCompletionStorePhase::StoreReturnedPostconditionFailed,
            error,
            prepared.terminal,
            recovery_key,
        ));
    }
    Ok(DurableCandidateCleanupCompletion {
        terminal: prepared.terminal,
        receipt,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_cleanup_completion(
    recovery: CandidateCleanupCompletionOutcomeUncertainCustody,
    observed: &ComputePluginCandidateCleanupCompletionRecoveryOutcome,
    authority_session: ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    CandidateCleanupCompletionRecoveryAdoption,
    CandidateCleanupCompletionRecoveryAdoptionFailure,
> {
    if let Err(error) = validate_recovery_provenance(&recovery, &authority_session) {
        return Err(adoption_failure(
            CandidateCleanupCompletionRecoveryAdoptionPhase::RejectedBeforeOutcomeRead,
            error,
            recovery,
        ));
    }
    let fresh =
        match authority_session.read_candidate_cleanup_completion_outcome(&recovery.recovery_key) {
            Ok(fresh) => fresh,
            Err(error) => {
                return Err(adoption_failure(
                    CandidateCleanupCompletionRecoveryAdoptionPhase::OutcomeReadFailed,
                    error,
                    recovery,
                ))
            }
        };
    if &fresh != observed {
        return Err(adoption_failure(
            CandidateCleanupCompletionRecoveryAdoptionPhase::OutcomeChanged,
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OUTCOME_CHANGED"),
            recovery,
        ));
    }
    if let Err(error) =
        authority_session.validate_source(recovery.terminal.physical().deletion_guard())
    {
        return Err(adoption_failure(
            CandidateCleanupCompletionRecoveryAdoptionPhase::RetainedCustodyChanged,
            error,
            recovery,
        ));
    }
    match fresh {
        ComputePluginCandidateCleanupCompletionRecoveryOutcome::NotCreated => Ok(
            CandidateCleanupCompletionRecoveryAdoption::NotCreated(recovery.terminal),
        ),
        ComputePluginCandidateCleanupCompletionRecoveryOutcome::Completed(receipt) => {
            if let Err(error) =
                validate_cleanup_completion_receipt(&recovery.recovery_key, &receipt)
            {
                return Err(adoption_failure(
                    CandidateCleanupCompletionRecoveryAdoptionPhase::OutcomeChanged,
                    error,
                    recovery,
                ));
            }
            Ok(CandidateCleanupCompletionRecoveryAdoption::Completed(
                DurableCandidateCleanupCompletion {
                    terminal: recovery.terminal,
                    receipt,
                },
            ))
        }
    }
}

fn validate_recovery_provenance(
    recovery: &CandidateCleanupCompletionOutcomeUncertainCustody,
    session: &ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'_>,
) -> Result<()> {
    let key = &recovery.recovery_key;
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || session.observed_at() <= key.physical_completed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_PROVENANCE_CHANGED");
    }
    session.validate_source(recovery.terminal.physical().deletion_guard())
}

fn validate_cleanup_completion_receipt(
    key: &CandidateCleanupCompletionRecoveryKey,
    hashed: &HashedComputePluginCandidateCleanupCompletionReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let expected = key.receipt_expectation();
    if hashed.schema() != HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA
        || receipt.schema() != CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA
        || hashed.canonicalization() != CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION
        || hashed.digest_algorithm() != CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM
        || receipt.completion_id() != key.completion_id()
        || receipt.cleanup_id() != expected.cleanup_id
        || receipt.candidate_token_digest() != expected.candidate_token_digest
        || receipt.authorization_receipt_digest() != expected.authorization_receipt_digest
        || receipt.execution_plan_digest() != expected.execution_plan_digest
        || receipt.execution_evidence_digest() != expected.execution_evidence_digest
        || receipt.terminal_journal_digest() != expected.terminal_journal_digest
        || receipt.authority_state_revision_before() != expected.authority_state_revision_before
        || receipt.authority_state_revision_after() != expected.authority_state_revision_after
        || receipt.inventory_revision_before() != expected.inventory_revision_before
        || receipt.inventory_revision_after() != expected.inventory_revision_after
        || receipt.inventory_digest_before() != expected.inventory_digest_before
        || receipt.inventory_digest_after() != expected.inventory_digest_after
        || receipt.authority_epoch_before() != expected.authority_epoch_before
        || receipt.authority_epoch_after() != expected.authority_epoch_after
        || receipt.process_owner_epoch() != expected.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before() != expected.trusted_time_high_water_ms_before
        || receipt.completed_at_ms() != expected.completed_at_ms
        || receipt.slot_phase_before() != "failed"
        || receipt.slot_phase_after() != "removed"
        || !is_sha256(hashed.receipt_digest())
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_POSTCONDITION_CHANGED");
    }
    Ok(())
}

fn store_failure(
    phase: CandidateCleanupCompletionStorePhase,
    error: Error,
    terminal: DurableCandidateCleanupTerminalJournal,
    recovery_key: CandidateCleanupCompletionRecoveryKey,
) -> CandidateCleanupCompletionStoreFailure {
    CandidateCleanupCompletionStoreFailure {
        phase,
        error,
        recovery: CandidateCleanupCompletionOutcomeUncertainCustody {
            terminal,
            recovery_key,
        },
    }
}

fn adoption_failure(
    phase: CandidateCleanupCompletionRecoveryAdoptionPhase,
    error: Error,
    recovery: CandidateCleanupCompletionOutcomeUncertainCustody,
) -> CandidateCleanupCompletionRecoveryAdoptionFailure {
    CandidateCleanupCompletionRecoveryAdoptionFailure {
        phase,
        error,
        recovery,
    }
}

impl<'permit, 'authority> ValidatedCandidateCleanupCompletionPermit<'permit, 'authority> {
    fn new(prepared: &'permit PreparedCandidateCleanupCompletion<'authority>) -> Self {
        Self { prepared }
    }
    pub(in crate::node_agent_compute_plugin_host) fn terminal(
        &self,
    ) -> &DurableCandidateCleanupTerminalJournal {
        &self.prepared.terminal
    }
    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginCandidateCleanupCompletionAuthorityFacts {
        &self.prepared.facts
    }
    pub(in crate::node_agent_compute_plugin_host) fn completion_id(&self) -> &str {
        &self.prepared.completion_id
    }
}

impl PreparedCandidateCleanupCompletion<'_> {
    pub(super) fn terminal(&self) -> &DurableCandidateCleanupTerminalJournal {
        &self.terminal
    }
    pub(super) fn authority_session(
        &self,
    ) -> &ComputePluginCandidateCleanupCompletionAuthoritySession<'_> {
        &self.authority_session
    }
    pub(super) fn facts(&self) -> &ComputePluginCandidateCleanupCompletionAuthorityFacts {
        &self.facts
    }
    pub(super) fn completion_id(&self) -> &str {
        &self.completion_id
    }
}

impl DurableCandidateCleanupCompletion {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupCompletionReceipt {
        &self.receipt
    }
    pub(in crate::node_agent_compute_plugin_host) fn terminal(
        &self,
    ) -> &DurableCandidateCleanupTerminalJournal {
        &self.terminal
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        DurableCandidateCleanupTerminalJournal,
        HashedComputePluginCandidateCleanupCompletionReceipt,
    ) {
        (self.terminal, self.receipt)
    }
}

impl CandidateCleanupCompletionPreparationFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, DurableCandidateCleanupTerminalJournal) {
        (self.error, self.terminal)
    }
}

impl CandidateCleanupCompletionStoreFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupCompletionStorePhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupCompletionOutcomeUncertainCustody) {
        (self.error, self.recovery)
    }
}

impl CandidateCleanupCompletionOutcomeUncertainCustody {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateCleanupCompletionRecoveryKey {
        &self.recovery_key
    }
}

impl CandidateCleanupCompletionRecoveryAdoptionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupCompletionRecoveryAdoptionPhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupCompletionOutcomeUncertainCustody) {
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

impl_failure!(CandidateCleanupCompletionPreparationFailure);
impl_failure!(CandidateCleanupCompletionStoreFailure);
impl_failure!(CandidateCleanupCompletionRecoveryAdoptionFailure);

impl fmt::Debug for PreparedCandidateCleanupCompletion<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedCandidateCleanupCompletion")
            .field("completion_id", &"<redacted>")
            .field("physical", &"<retained-custody>")
            .finish()
    }
}

impl fmt::Debug for DurableCandidateCleanupCompletion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableCandidateCleanupCompletion")
            .field("receipt", &self.receipt)
            .field("physical", &"<retained-custody>")
            .finish()
    }
}

impl fmt::Debug for CandidateCleanupCompletionOutcomeUncertainCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupCompletionOutcomeUncertainCustody")
            .field("recovery_key", &self.recovery_key)
            .field("physical", &"<retained-custody>")
            .finish()
    }
}
