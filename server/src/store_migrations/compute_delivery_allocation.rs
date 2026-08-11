use anyhow::{bail, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

mod grant_guards;
mod tables;
mod terminal_guards;

const LEGACY_COLUMNS: [&str; 22] = [
    "commitment_id",
    "commitment_digest",
    "owner_account_id",
    "provider_id",
    "offer_id",
    "offer_version",
    "pool_id",
    "capacity_epoch",
    "delivery_window_id",
    "price_snapshot_id",
    "reference_binding_id",
    "claim_id",
    "current_revision",
    "current_status",
    "current_claim_revision",
    "current_claim_digest",
    "terminal_receipt_id",
    "terminal_receipt_digest",
    "created_at",
    "expires_at",
    "terminal_occurred_at",
    "terminal_recorded_at",
];

pub(crate) fn migration_v228(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    preflight_legacy_view(&transaction)?;
    tables::create(&transaction)?;
    grant_guards::install(&transaction)?;
    terminal_guards::install(&transaction)?;
    replace_current_view(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn preflight_legacy_view(conn: &Connection) -> Result<()> {
    let object = conn
        .query_row(
            "SELECT type, sql FROM sqlite_master
              WHERE name='compute_capacity_commitment_current'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((object_type, sql)) = object else {
        bail!("v228 requires the exact v225 commitment current view");
    };
    let normalized_sql = sql
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if object_type != "view"
        || !normalized_sql.contains("from compute_capacity_commitments commitment")
        || !normalized_sql
            .contains("left join compute_capacity_commitment_terminal_receipts terminal")
        || normalized_sql.contains("compute_delivery_allocation")
        || normalized_sql.contains(" union ")
    {
        bail!("v228 refuses an unknown commitment current object");
    }

    let mut statement = conn.prepare("PRAGMA table_info(compute_capacity_commitment_current)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if columns.len() != LEGACY_COLUMNS.len()
        || columns
            .iter()
            .zip(LEGACY_COLUMNS)
            .any(|(actual, expected)| actual.as_str() != expected)
    {
        bail!("v228 refuses a commitment current view with unknown columns");
    }

    let (view_rows, commitment_rows): (i64, i64) = conn.query_row(
        "SELECT (SELECT COUNT(*) FROM compute_capacity_commitment_current),
                (SELECT COUNT(*) FROM compute_capacity_commitments)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mismatches: i64 = conn.query_row(
        r#"
        WITH expected AS (
            SELECT commitment.commitment_id, commitment.commitment_digest,
                   commitment.owner_account_id, commitment.provider_id,
                   commitment.offer_id, commitment.offer_version,
                   commitment.pool_id, commitment.capacity_epoch,
                   commitment.delivery_window_id, commitment.price_snapshot_id,
                   commitment.reference_binding_id, commitment.claim_id,
                   COALESCE(terminal.terminal_revision, commitment.commitment_revision)
                        AS current_revision,
                   COALESCE(terminal.terminal_status, commitment.commitment_status)
                        AS current_status,
                   COALESCE(terminal.result_claim_revision, commitment.claim_revision)
                        AS current_claim_revision,
                   COALESCE(terminal.result_claim_digest, commitment.claim_digest)
                        AS current_claim_digest,
                   terminal.terminal_receipt_id, terminal.terminal_receipt_digest,
                   commitment.created_at, commitment.expires_at,
                   terminal.occurred_at AS terminal_occurred_at,
                   terminal.recorded_at AS terminal_recorded_at
              FROM compute_capacity_commitments commitment
              LEFT JOIN compute_capacity_commitment_terminal_receipts terminal
                ON terminal.commitment_id=commitment.commitment_id
        )
        SELECT
          (SELECT COUNT(*) FROM (
              SELECT * FROM compute_capacity_commitment_current
              EXCEPT SELECT * FROM expected
          )) +
          (SELECT COUNT(*) FROM (
              SELECT * FROM expected
              EXCEPT SELECT * FROM compute_capacity_commitment_current
          ))
        "#,
        [],
        |row| row.get(0),
    )?;
    if view_rows != commitment_rows || mismatches != 0 {
        bail!("v228 refuses a commitment current view with drifted rows");
    }
    Ok(())
}

fn replace_current_view(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW compute_capacity_commitment_current;
        CREATE VIEW compute_capacity_commitment_current AS
        SELECT commitment.commitment_id,
               commitment.commitment_digest,
               commitment.owner_account_id,
               commitment.provider_id,
               commitment.offer_id,
               commitment.offer_version,
               commitment.pool_id,
               commitment.capacity_epoch,
               commitment.delivery_window_id,
               commitment.price_snapshot_id,
               commitment.reference_binding_id,
               commitment.claim_id,
               CASE WHEN allocation_terminal.terminal_status='exercised'
                    THEN allocation_terminal.terminal_revision
                    ELSE COALESCE(terminal.terminal_revision,
                                      commitment.commitment_revision) END AS current_revision,
               CASE WHEN allocation_terminal.terminal_status='exercised' THEN 'allocated'
                    ELSE COALESCE(terminal.terminal_status,
                                      commitment.commitment_status) END AS current_status,
               CASE WHEN allocation_terminal.terminal_status='exercised'
                    THEN allocation_terminal.parent_result_claim_revision
                    ELSE COALESCE(terminal.result_claim_revision,
                                      commitment.claim_revision) END AS current_claim_revision,
               CASE WHEN allocation_terminal.terminal_status='exercised'
                    THEN allocation_terminal.parent_result_claim_digest
                    ELSE COALESCE(terminal.result_claim_digest,
                                      commitment.claim_digest) END AS current_claim_digest,
               terminal.terminal_receipt_id,
               terminal.terminal_receipt_digest,
               commitment.created_at,
               commitment.expires_at,
               terminal.occurred_at AS terminal_occurred_at,
               terminal.recorded_at AS terminal_recorded_at,
               allocation_grant.grant_id AS delivery_allocation_grant_id,
               allocation_grant.grant_digest AS delivery_allocation_grant_digest,
               allocation_terminal.terminal_receipt_id
                    AS delivery_allocation_terminal_receipt_id,
               allocation_terminal.terminal_receipt_digest
                    AS delivery_allocation_terminal_receipt_digest,
               allocation_terminal.terminal_status
                    AS delivery_allocation_terminal_status,
               allocation_terminal.occurred_at
                    AS delivery_allocation_terminal_occurred_at,
               allocation_terminal.recorded_at
                    AS delivery_allocation_terminal_recorded_at
          FROM compute_capacity_commitments commitment
          LEFT JOIN compute_capacity_commitment_terminal_receipts terminal
            ON terminal.commitment_id=commitment.commitment_id
          LEFT JOIN compute_delivery_allocation_grants allocation_grant
            ON allocation_grant.commitment_id=commitment.commitment_id
          LEFT JOIN compute_delivery_allocation_terminal_receipts allocation_terminal
            ON allocation_terminal.grant_id=allocation_grant.grant_id;
        "#,
    )?;
    Ok(())
}
