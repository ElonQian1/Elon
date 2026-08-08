use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    projection::validate_removed_candidate_inventory,
    terminal::validate_recovery_terminal_journal,
    types::{
        ComputePluginCandidateCleanupCompletionReceipt,
        HashedComputePluginCandidateCleanupCompletionReceipt,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
    },
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::CandidateCleanupCompletionRecoveryKey,
    local_authority::{
        cleanup_store::binding::validate_failed_candidate_inventory,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
        AuthorizedCandidateCleanupDeletionGuard, ComputePluginAuthorityInstanceBinding,
        ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateCleanupCompletionRecoveryOutcome
{
    NotCreated,
    Completed(HashedComputePluginCandidateCleanupCompletionReceipt),
}

struct StoredCompletionRow {
    completion_id: String,
    cleanup_id: String,
    candidate_token: String,
    authorization_receipt_digest: String,
    execution_plan_digest: String,
    execution_evidence_digest: String,
    terminal_journal_digest: String,
    state_before: i64,
    state_after: i64,
    inventory_before: i64,
    inventory_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    epoch_before: i64,
    epoch_after: i64,
    process_owner_epoch: i64,
    time_before: i64,
    completed_at_ms: i64,
    slot_phase_before: String,
    slot_phase_after: String,
    receipt_json: String,
    receipt_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_completion_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.installation_id_digest())
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }
    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &AuthorizedCandidateCleanupDeletionGuard,
    ) -> Result<()> {
        guard.validate_process_fence(self.process_fence)
    }
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_completion_outcome(
        &self,
        key: &CandidateCleanupCompletionRecoveryKey,
    ) -> Result<ComputePluginCandidateCleanupCompletionRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateCleanupCompletionRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }
    pub(in crate::node_agent_compute_plugin_host) fn completion_receipt(
        &self,
    ) -> Option<&HashedComputePluginCandidateCleanupCompletionReceipt> {
        match self {
            Self::NotCreated => None,
            Self::Completed(receipt) => Some(receipt),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || session.observed_at <= session.process_fence.acquired_observed_at()
        || session.observed_at <= key.physical_completed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<ComputePluginCandidateCleanupCompletionRecoveryOutcome> {
    let stored = read_exact_row(transaction, key)?;
    let identity_matches = count_identity_matches(transaction, key)?;
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let expected = key.receipt_expectation();
    let slot = key.slot_expectation();
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.process_owner_epoch != expected.process_owner_epoch
        || count_exact_authorization(transaction, key)? != 1
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_AUTHORITY_CHANGED");
    }
    validate_recovery_terminal_journal(transaction, key)?;

    match stored {
        Some(row) => {
            if identity_matches != 1
                || count_owner_state(transaction, key, "cleaned", Some(expected.completed_at_ms))?
                    != 1
                || authority.state_revision < expected.authority_state_revision_after
                || authority.inventory.inventory_revision < expected.inventory_revision_after
                || authority.authority_epoch < expected.authority_epoch_after
                || authority.trusted_time_high_water_ms < expected.completed_at_ms
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_RESULT_AMBIGUOUS");
            }
            validate_removed_candidate_inventory(
                &authority.inventory,
                &slot.plugin_id,
                &slot.slot_ref,
            )?;
            Ok(
                ComputePluginCandidateCleanupCompletionRecoveryOutcome::Completed(
                    decode_recorded_row(key, row)?,
                ),
            )
        }
        None => {
            if identity_matches != 0
                || count_owner_state(transaction, key, "cleanup_pending", None)? != 1
                || count_prepared_work(transaction)? != 0
                || authority.state_revision < expected.authority_state_revision_before
                || authority.inventory.inventory_revision < expected.inventory_revision_before
                || authority.authority_epoch < expected.authority_epoch_before
                || authority.trusted_time_high_water_ms < expected.trusted_time_high_water_ms_before
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            validate_failed_candidate_inventory(
                &authority.inventory,
                &slot.plugin_id,
                &slot.slot_ref,
                &slot.release,
            )?;
            Ok(ComputePluginCandidateCleanupCompletionRecoveryOutcome::NotCreated)
        }
    }
}

fn read_exact_row(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<Option<StoredCompletionRow>> {
    transaction
        .query_row(
            r#"SELECT completion_id, cleanup_id, candidate_token,
                authorization_receipt_digest, execution_plan_digest,
                execution_evidence_digest, terminal_journal_digest,
                authority_state_revision_before, authority_state_revision_after,
                inventory_revision_before, inventory_revision_after,
                inventory_digest_before, inventory_digest_after,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_high_water_ms_before, completed_at_ms,
                slot_phase_before, slot_phase_after, receipt_json, receipt_digest
            FROM candidate_cleanup_completions WHERE completion_id = ?1"#,
            params![key.completion_id()],
            |row| {
                Ok(StoredCompletionRow {
                    completion_id: row.get(0)?,
                    cleanup_id: row.get(1)?,
                    candidate_token: row.get(2)?,
                    authorization_receipt_digest: row.get(3)?,
                    execution_plan_digest: row.get(4)?,
                    execution_evidence_digest: row.get(5)?,
                    terminal_journal_digest: row.get(6)?,
                    state_before: row.get(7)?,
                    state_after: row.get(8)?,
                    inventory_before: row.get(9)?,
                    inventory_after: row.get(10)?,
                    inventory_digest_before: row.get(11)?,
                    inventory_digest_after: row.get(12)?,
                    epoch_before: row.get(13)?,
                    epoch_after: row.get(14)?,
                    process_owner_epoch: row.get(15)?,
                    time_before: row.get(16)?,
                    completed_at_ms: row.get(17)?,
                    slot_phase_before: row.get(18)?,
                    slot_phase_after: row.get(19)?,
                    receipt_json: row.get(20)?,
                    receipt_digest: row.get(21)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_ROW_READ")
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<i64> {
    let expected = key.receipt_expectation();
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_completions
               WHERE completion_id = ?1 OR cleanup_id = ?2 OR candidate_token = ?3
                  OR authorization_receipt_digest = ?4 OR execution_plan_digest = ?5
                  OR execution_evidence_digest = ?6 OR terminal_journal_digest = ?7"#,
            params![
                key.completion_id(),
                expected.cleanup_id,
                key.candidate_token(),
                expected.authorization_receipt_digest,
                expected.execution_plan_digest,
                expected.execution_evidence_digest,
                expected.terminal_journal_digest,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_IDENTITY_READ")
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<i64> {
    let expected = key.receipt_expectation();
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 AND candidate_token = ?2
                 AND candidate_token_digest = ?3 AND receipt_digest = ?4
                 AND process_owner_epoch = ?5 AND authorized_at_ms <= ?6
                 AND slot_phase_before = 'failed'"#,
            params![
                expected.cleanup_id,
                key.candidate_token(),
                expected.candidate_token_digest,
                expected.authorization_receipt_digest,
                expected.process_owner_epoch,
                expected.trusted_time_high_water_ms_before,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_AUTHORIZATION_READ")
}

fn count_prepared_work(transaction: &Transaction<'_>) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT
                (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared')
              + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')"#,
            [],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_CONFLICT_READ")
}

fn count_owner_state(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
    state: &str,
    closed_at_ms: Option<i64>,
) -> Result<i64> {
    let slot = key.slot_expectation();
    let release_json = serde_json::to_string(&slot.release)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_RELEASE")?;
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND plugin_id = ?2 AND slot_ref = ?3
                 AND candidate_generation = ?4 AND release_json = ?5
                 AND owner_plan_id = ?6 AND owner_plan_digest = ?7
                 AND application_inventory_revision = ?8 AND state = ?9
                 AND closed_at_ms IS ?10 AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL
                 AND close_reason IS ?11"#,
            params![
                key.candidate_token(),
                slot.plugin_id.as_str(),
                slot.slot_ref.as_str(),
                key.candidate_generation(),
                release_json,
                key.owner_plan_id(),
                key.owner_plan_digest(),
                key.application_inventory_revision(),
                state,
                closed_at_ms,
                closed_at_ms.map(|_| "candidate_cleanup_completed"),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_OWNER_READ")
}

fn decode_recorded_row(
    key: &CandidateCleanupCompletionRecoveryKey,
    row: StoredCompletionRow,
) -> Result<HashedComputePluginCandidateCleanupCompletionReceipt> {
    let receipt: ComputePluginCandidateCleanupCompletionReceipt =
        serde_json::from_str(&row.receipt_json)
            .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_RECEIPT_PARSE")?;
    let expected = key.receipt_expectation();
    if serde_json::to_string(&receipt)? != row.receipt_json
        || receipt.schema != CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA
        || row.completion_id != key.completion_id()
        || row.cleanup_id != expected.cleanup_id
        || row.candidate_token != key.candidate_token()
        || row.authorization_receipt_digest != expected.authorization_receipt_digest
        || row.execution_plan_digest != expected.execution_plan_digest
        || row.execution_evidence_digest != expected.execution_evidence_digest
        || row.terminal_journal_digest != expected.terminal_journal_digest
        || row.state_before != expected.authority_state_revision_before
        || row.state_after != expected.authority_state_revision_after
        || row.inventory_before != expected.inventory_revision_before
        || row.inventory_after != expected.inventory_revision_after
        || row.inventory_digest_before != expected.inventory_digest_before
        || row.inventory_digest_after != expected.inventory_digest_after
        || row.epoch_before != expected.authority_epoch_before
        || row.epoch_after != expected.authority_epoch_after
        || row.process_owner_epoch != expected.process_owner_epoch
        || row.time_before != expected.trusted_time_high_water_ms_before
        || row.completed_at_ms != expected.completed_at_ms
        || row.slot_phase_before != "failed"
        || row.slot_phase_after != "removed"
        || receipt.completion_id != row.completion_id
        || receipt.cleanup_id != row.cleanup_id
        || receipt.candidate_token_digest != expected.candidate_token_digest
        || receipt.authorization_receipt_digest != row.authorization_receipt_digest
        || receipt.execution_plan_digest != row.execution_plan_digest
        || receipt.execution_evidence_digest != row.execution_evidence_digest
        || receipt.terminal_journal_digest != row.terminal_journal_digest
        || receipt.authority_state_revision_before != row.state_before
        || receipt.authority_state_revision_after != row.state_after
        || receipt.inventory_revision_before != row.inventory_before
        || receipt.inventory_revision_after != row.inventory_after
        || receipt.inventory_digest_before != row.inventory_digest_before
        || receipt.inventory_digest_after != row.inventory_digest_after
        || receipt.authority_epoch_before != row.epoch_before
        || receipt.authority_epoch_after != row.epoch_after
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before != row.time_before
        || receipt.completed_at_ms != row.completed_at_ms
        || receipt.slot_phase_before != row.slot_phase_before
        || receipt.slot_phase_after != row.slot_phase_after
        || !is_sha256(&row.receipt_digest)
        || jcs_sha256_hex(&receipt)? != row.receipt_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECOVERY_ROW_CHANGED");
    }
    Ok(HashedComputePluginCandidateCleanupCompletionReceipt {
        schema: HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA.to_string(),
        receipt,
        canonicalization: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest,
    })
}
