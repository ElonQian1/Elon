use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn audit_existing_candidates(conn: &Connection) -> Result<()> {
    let drifted_candidates: i64 = conn.query_row(
        r#"
        SELECT COUNT(*)
          FROM compute_attempt_terminal_candidates candidate
         WHERE NOT EXISTS (
            SELECT 1
              FROM compute_attempt_usage_declarations usage
             WHERE usage.lease_id=candidate.lease_id
               AND usage.sequence_no=(
                    SELECT MAX(head.sequence_no)
                      FROM compute_attempt_usage_declarations head
                     WHERE head.lease_id=candidate.lease_id)
               AND usage.snapshot_id=candidate.final_usage_snapshot_id
               AND usage.sequence_no=candidate.final_usage_sequence_no
               AND usage.cumulative_usage_digest=candidate.final_cumulative_usage_digest
               AND usage.provider_id=candidate.provider_id
               AND usage.consumer_account_id=candidate.consumer_account_id
               AND usage.source_lease_revision=candidate.source_lease_revision
               AND usage.source_lease_digest=candidate.source_lease_digest
               AND usage.source_lease_status=candidate.source_lease_status
               AND usage.fencing_generation=candidate.fencing_generation
               AND usage.job_id=candidate.job_id
               AND usage.job_revision=candidate.job_revision
               AND usage.job_digest=candidate.job_digest
               AND usage.reservation_id=candidate.reservation_id
               AND usage.reservation_revision=candidate.reservation_revision
               AND usage.reservation_digest=candidate.reservation_digest
               AND usage.capacity_claim_id=candidate.capacity_claim_id
               AND usage.capacity_claim_revision=candidate.capacity_claim_revision
               AND usage.capacity_claim_digest=candidate.capacity_claim_digest
         )
        "#,
        [],
        |row| row.get(0),
    )?;
    if drifted_candidates != 0 {
        bail!(
            "cannot install Attempt final-usage fence: {drifted_candidates} terminal candidate(s) do not bind the exact current usage head"
        );
    }
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_terminal_candidates_final_usage_head
        BEFORE INSERT ON compute_attempt_terminal_candidates
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_usage_declarations usage
             WHERE usage.lease_id=NEW.lease_id
               AND usage.sequence_no=(
                    SELECT MAX(head.sequence_no)
                      FROM compute_attempt_usage_declarations head
                     WHERE head.lease_id=NEW.lease_id)
               AND usage.snapshot_id=NEW.final_usage_snapshot_id
               AND usage.sequence_no=NEW.final_usage_sequence_no
               AND usage.cumulative_usage_digest=NEW.final_cumulative_usage_digest
               AND usage.provider_id=NEW.provider_id
               AND usage.consumer_account_id=NEW.consumer_account_id
               AND usage.source_lease_revision=NEW.source_lease_revision
               AND usage.source_lease_digest=NEW.source_lease_digest
               AND usage.source_lease_status=NEW.source_lease_status
               AND usage.fencing_generation=NEW.fencing_generation
               AND usage.job_id=NEW.job_id
               AND usage.job_revision=NEW.job_revision
               AND usage.job_digest=NEW.job_digest
               AND usage.reservation_id=NEW.reservation_id
               AND usage.reservation_revision=NEW.reservation_revision
               AND usage.reservation_digest=NEW.reservation_digest
               AND usage.capacity_claim_id=NEW.capacity_claim_id
               AND usage.capacity_claim_revision=NEW.capacity_claim_revision
               AND usage.capacity_claim_digest=NEW.capacity_claim_digest
        )
        BEGIN
            SELECT RAISE(ABORT,
                'compute attempt terminal candidate must bind the exact current usage head');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_usage_declarations_terminal_seal
        BEFORE INSERT ON compute_attempt_usage_declarations
        WHEN EXISTS (
            SELECT 1
              FROM compute_attempt_terminal_candidates candidate
             WHERE candidate.lease_id=NEW.lease_id
        )
        BEGIN
            SELECT RAISE(ABORT,
                'compute attempt usage declaration stream is sealed by terminal candidate');
        END;
        "#,
    )?;
    Ok(())
}
