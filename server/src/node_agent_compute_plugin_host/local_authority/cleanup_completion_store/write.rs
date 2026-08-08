use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    binding::read_candidate_cleanup_completion_binding,
    meta::update_cleanup_completion_authority_meta,
    projection::{project_candidate_cleanup_completion, validate_removed_candidate_inventory},
    types::{
        ComputePluginCandidateCleanupCompletionReceipt,
        HashedComputePluginCandidateCleanupCompletionReceipt,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
    },
    ComputePluginCandidateCleanupCompletionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::ValidatedCandidateCleanupCompletionPermit,
    local_authority::{
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
        plan_application::{
            read_authority_plan_application_state,
            read_authority_plan_application_state_at_or_before_observation,
        },
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn persist_candidate_cleanup_completion(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupCompletionAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupCompletionPermit<'_, '_>,
) -> Result<HashedComputePluginCandidateCleanupCompletionReceipt> {
    let terminal = permit.terminal();
    let physical = terminal.physical();
    session.validate_source(physical.deletion_guard())?;
    let current = read_candidate_cleanup_completion_binding(transaction, session, terminal)?;
    if &current != permit.facts() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let slot = physical.staging_recovery_key().slot_expectation();
    let projection = project_candidate_cleanup_completion(
        &authority,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
        &session.trusted_now,
    )?;
    if projection.state_revision != current.authority_state_revision_after()
        || projection.inventory.inventory_revision != current.inventory_revision_after()
        || projection.inventory_digest != current.inventory_digest_after()
        || projection.inventory_json != current.inventory_json_after()
        || &projection.inventory != current.inventory_after()
        || projection.authority_epoch != current.authority_epoch_after()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_PROJECTION_CHANGED");
    }

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != current.authority_state_revision_before()
        || time_state.authority_epoch != current.authority_epoch_before()
        || time_state.trusted_time_high_water_ms
            != Some(current.trusted_time_high_water_ms_before())
        || time_state.clock_status != "trusted"
        || current.completed_at_ms() <= current.trusted_time_high_water_ms_before()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, current.completed_at_ms())?;
    update_cleanup_completion_authority_meta(
        transaction,
        &authority,
        &projection,
        current.completed_at_ms(),
    )?;
    session.validate_source(physical.deletion_guard())?;

    let receipt = ComputePluginCandidateCleanupCompletionReceipt {
        schema: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA.to_string(),
        completion_id: permit.completion_id().to_string(),
        cleanup_id: current.cleanup_id().to_string(),
        candidate_token_digest: current.candidate_token_digest().to_string(),
        authorization_receipt_digest: current.authorization_receipt_digest().to_string(),
        execution_plan_digest: current.execution_plan_digest().to_string(),
        execution_evidence_digest: current.execution_evidence_digest().to_string(),
        terminal_journal_digest: current.terminal_journal_digest().to_string(),
        authority_state_revision_before: current.authority_state_revision_before(),
        authority_state_revision_after: current.authority_state_revision_after(),
        inventory_revision_before: current.inventory_revision_before(),
        inventory_revision_after: current.inventory_revision_after(),
        inventory_digest_before: current.inventory_digest_before().to_string(),
        inventory_digest_after: current.inventory_digest_after().to_string(),
        authority_epoch_before: current.authority_epoch_before(),
        authority_epoch_after: current.authority_epoch_after(),
        process_owner_epoch: current.process_owner_epoch(),
        trusted_time_high_water_ms_before: current.trusted_time_high_water_ms_before(),
        completed_at_ms: current.completed_at_ms(),
        slot_phase_before: "failed".to_string(),
        slot_phase_after: "removed".to_string(),
    };
    let hashed = HashedComputePluginCandidateCleanupCompletionReceipt {
        schema: HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA.to_string(),
        receipt_digest: jcs_sha256_hex(&receipt)?,
        receipt,
        canonicalization: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM.to_string(),
    };
    insert_completion(transaction, &permit, &hashed)?;
    mark_owner_cleaned(transaction, &permit, &hashed)?;
    validate_readback(transaction, session, &permit, &hashed)?;
    session.validate_source(physical.deletion_guard())?;
    Ok(hashed)
}

fn insert_completion(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateCleanupCompletionPermit<'_, '_>,
    hashed: &HashedComputePluginCandidateCleanupCompletionReceipt,
) -> Result<()> {
    let candidate_token = permit
        .terminal()
        .physical()
        .staging_recovery_key()
        .candidate_token();
    let receipt = hashed.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_cleanup_completions (
                completion_id, cleanup_id, candidate_token,
                authorization_receipt_digest, execution_plan_digest,
                execution_evidence_digest, terminal_journal_digest,
                authority_state_revision_before, authority_state_revision_after,
                inventory_revision_before, inventory_revision_after,
                inventory_digest_before, inventory_digest_after,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_high_water_ms_before, completed_at_ms,
                slot_phase_before, slot_phase_after, receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22
            )"#,
            params![
                receipt.completion_id(),
                receipt.cleanup_id(),
                candidate_token,
                receipt.authorization_receipt_digest(),
                receipt.execution_plan_digest(),
                receipt.execution_evidence_digest(),
                receipt.terminal_journal_digest(),
                receipt.authority_state_revision_before(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_before(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_before(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_before(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.trusted_time_high_water_ms_before(),
                receipt.completed_at_ms(),
                receipt.slot_phase_before(),
                receipt.slot_phase_after(),
                receipt_json,
                hashed.receipt_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_INSERT")?;
    Ok(())
}

fn mark_owner_cleaned(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateCleanupCompletionPermit<'_, '_>,
    hashed: &HashedComputePluginCandidateCleanupCompletionReceipt,
) -> Result<()> {
    let candidate_token = permit
        .terminal()
        .physical()
        .staging_recovery_key()
        .candidate_token();
    let receipt = hashed.receipt();
    let updated = transaction
        .execute(
            r#"UPDATE candidate_owners SET
                    state = 'cleaned', closed_at_ms = ?4,
                    closed_by_plan_id = NULL, closed_by_plan_digest = NULL,
                    close_reason = 'candidate_cleanup_completed'
               WHERE candidate_token = ?1 AND state = 'cleanup_pending'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL
                 AND EXISTS (
                     SELECT 1 FROM candidate_cleanup_completions
                     WHERE candidate_token = ?1 AND completion_id = ?2
                       AND receipt_digest = ?3
                 )"#,
            params![
                candidate_token,
                receipt.completion_id(),
                hashed.receipt_digest(),
                receipt.completed_at_ms(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_CAS");
    }
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupCompletionAuthoritySession<'_>,
    permit: &ValidatedCandidateCleanupCompletionPermit<'_, '_>,
    hashed: &HashedComputePluginCandidateCleanupCompletionReceipt,
) -> Result<()> {
    let physical = permit.terminal().physical();
    let staging = physical.staging_recovery_key();
    let slot = staging.slot_expectation();
    let expected = staging.receipt_expectation();
    let receipt = hashed.receipt();
    let release_json = serde_json::to_string(&slot.release)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_READBACK_RELEASE")?;
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_READBACK_SERIALIZE")?;
    let row_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_completions
               WHERE completion_id = ?1 AND cleanup_id = ?2 AND candidate_token = ?3
                  AND authorization_receipt_digest = ?4
                  AND execution_plan_digest = ?5
                  AND execution_evidence_digest = ?6
                  AND terminal_journal_digest = ?7
                  AND authority_state_revision_before = ?8
                  AND authority_state_revision_after = ?9
                  AND inventory_revision_before = ?10
                  AND inventory_revision_after = ?11
                  AND inventory_digest_before = ?12 AND inventory_digest_after = ?13
                  AND authority_epoch_before = ?14 AND authority_epoch_after = ?15
                  AND process_owner_epoch = ?16
                  AND trusted_time_high_water_ms_before = ?17
                  AND completed_at_ms = ?18 AND slot_phase_before = 'failed'
                  AND slot_phase_after = 'removed' AND receipt_json = ?19
                  AND receipt_digest = ?20"#,
            params![
                receipt.completion_id(),
                receipt.cleanup_id(),
                staging.candidate_token(),
                receipt.authorization_receipt_digest(),
                receipt.execution_plan_digest(),
                receipt.execution_evidence_digest(),
                receipt.terminal_journal_digest(),
                receipt.authority_state_revision_before(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_before(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_before(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_before(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.trusted_time_high_water_ms_before(),
                receipt.completed_at_ms(),
                receipt_json,
                hashed.receipt_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_READBACK")?;
    let owner_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND plugin_id = ?2 AND slot_ref = ?3
                 AND candidate_generation = ?4 AND release_json = ?5
                 AND owner_plan_id = ?6 AND owner_plan_digest = ?7
                 AND application_inventory_revision = ?8 AND state = 'cleaned'
                 AND closed_at_ms = ?9 AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL
                 AND close_reason = 'candidate_cleanup_completed'"#,
            params![
                staging.candidate_token(),
                slot.plugin_id.as_str(),
                slot.slot_ref.as_str(),
                expected.candidate_generation,
                release_json,
                expected.owner_plan_id.as_str(),
                expected.owner_plan_digest.as_str(),
                expected.application_inventory_revision,
                receipt.completed_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_READBACK")?;
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    if row_count != 1
        || owner_count != 1
        || authority.state_revision != receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision != receipt.inventory_revision_after()
        || authority.inventory_digest != receipt.inventory_digest_after()
        || authority.inventory_json != permit.facts().inventory_json_after()
        || authority.authority_epoch != receipt.authority_epoch_after()
        || authority.process_owner_epoch != receipt.process_owner_epoch()
        || authority.trusted_time_high_water_ms != receipt.completed_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_READBACK_CHANGED");
    }
    validate_removed_candidate_inventory(&authority.inventory, &slot.plugin_id, &slot.slot_ref)
}
