use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        CandidateCleanupCompletionRecoveryKey, DurableCandidateCleanupTerminalJournal,
    },
    manifest_validation::is_sha256,
};

pub(super) fn validate_terminal_journal(
    transaction: &Transaction<'_>,
    terminal: &DurableCandidateCleanupTerminalJournal,
) -> Result<()> {
    let physical = terminal.physical();
    let authorization = physical.authorization_receipt();
    validate_terminal_journal_expectation(
        transaction,
        authorization.receipt().cleanup_id(),
        physical.staging_recovery_key().candidate_token(),
        authorization.receipt_digest(),
        terminal.execution_plan_digest(),
        authorization.receipt().process_owner_epoch(),
        terminal.terminal_journal_digest(),
    )
}

pub(super) fn validate_recovery_terminal_journal(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupCompletionRecoveryKey,
) -> Result<()> {
    let expected = key.receipt_expectation();
    validate_terminal_journal_expectation(
        transaction,
        &expected.cleanup_id,
        key.candidate_token(),
        &expected.authorization_receipt_digest,
        &expected.execution_plan_digest,
        expected.process_owner_epoch,
        &expected.terminal_journal_digest,
    )
}

fn validate_terminal_journal_expectation(
    transaction: &Transaction<'_>,
    cleanup_id: &str,
    candidate_token: &str,
    authorization_receipt_digest: &str,
    execution_plan_digest: &str,
    process_owner_epoch: i64,
    terminal_journal_digest: &str,
) -> Result<()> {
    if !is_sha256(authorization_receipt_digest)
        || !is_sha256(execution_plan_digest)
        || !is_sha256(terminal_journal_digest)
        || process_owner_epoch <= 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_TERMINAL_CHANGED");
    }
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*)
               FROM candidate_cleanup_execution_plan_seals AS seal
               JOIN candidate_cleanup_execution_plans AS plan
                 ON plan.cleanup_id = seal.cleanup_id
                AND plan.candidate_token = seal.candidate_token
                AND plan.plan_digest = seal.plan_digest
               WHERE plan.cleanup_id = ?1 AND plan.candidate_token = ?2
                 AND plan.authorization_receipt_digest = ?3
                 AND plan.plan_digest = ?4
                 AND plan.process_owner_epoch = ?5
                 AND seal.object_count = plan.object_count
                 AND (SELECT COUNT(*) FROM candidate_cleanup_step_events AS event
                      WHERE event.cleanup_id = plan.cleanup_id
                        AND event.event_kind = 'namespace_durable') = plan.object_count
                 AND EXISTS (
                     SELECT 1 FROM candidate_cleanup_step_events AS terminal_event
                     WHERE terminal_event.cleanup_id = plan.cleanup_id
                       AND terminal_event.event_sequence = plan.object_count * 4
                       AND terminal_event.event_kind = 'namespace_durable'
                       AND terminal_event.event_digest = ?6
                 )"#,
            params![
                cleanup_id,
                candidate_token,
                authorization_receipt_digest,
                execution_plan_digest,
                process_owner_epoch,
                terminal_journal_digest,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_TERMINAL_READ")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_TERMINAL_CHANGED");
    }
    Ok(())
}
