use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_release_admission_current;
        CREATE VIEW compute_external_pool_adapter_release_admission_current AS
        SELECT admission.admission_id,
               admission.admission_digest,
               admission.adapter_id,
               admission.release_version,
               admission.applied_at,
               admission.status AS admission_status,
               COALESCE(terminal.terminal_status, admission.status) AS current_status,
               terminal.terminal_receipt_id,
               terminal.terminal_receipt_digest,
               terminal.occurred_at AS terminal_occurred_at,
               terminal.successor_admission_id,
               terminal.successor_admission_digest,
               terminal.successor_release_version
          FROM compute_external_pool_adapter_release_admissions admission
          LEFT JOIN compute_external_pool_adapter_release_admission_terminal_receipts terminal
            ON terminal.admission_id=admission.admission_id;
        "#,
    )?;
    Ok(())
}
