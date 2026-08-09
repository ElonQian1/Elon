use anyhow::{bail, Context, Result};
use rusqlite::{named_params, OptionalExtension, Transaction};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct WorkAdmissionHead {
    pub(super) installation_id_digest: String,
    pub(super) plugin_id: String,
    pub(super) generation: i64,
    pub(super) work_admission_id: String,
    pub(super) receipt_digest: String,
    pub(super) previous_id: Option<String>,
    pub(super) previous_receipt_digest: Option<String>,
    pub(super) updated_at_ms: i64,
}

pub(super) fn read_head(
    transaction: &Transaction<'_>,
    plugin_id: &str,
) -> Result<Option<WorkAdmissionHead>> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, plugin_id, work_admission_generation,
                work_admission_id, receipt_digest, previous_work_admission_id,
                previous_work_admission_receipt_digest, updated_at_ms
            FROM compute_plugin_work_admission_heads WHERE plugin_id = ?1"#,
            [plugin_id],
            |row| {
                Ok(WorkAdmissionHead {
                    installation_id_digest: row.get(0)?,
                    plugin_id: row.get(1)?,
                    generation: row.get(2)?,
                    work_admission_id: row.get(3)?,
                    receipt_digest: row.get(4)?,
                    previous_id: row.get(5)?,
                    previous_receipt_digest: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_HEAD_READ")
}

pub(super) fn advance_head(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    plugin_id: &str,
    generation_before: i64,
    generation_after: i64,
    work_admission_id: &str,
    receipt_digest: &str,
    previous_id: Option<&str>,
    previous_receipt_digest: Option<&str>,
    admitted_at_ms: i64,
) -> Result<()> {
    let changed = if generation_before == 0 {
        transaction.execute(
            r#"INSERT INTO compute_plugin_work_admission_heads (
                installation_id_digest, plugin_id, work_admission_generation,
                work_admission_id, receipt_digest, previous_work_admission_id,
                previous_work_admission_receipt_digest, updated_at_ms
            ) VALUES (
                :installation_id_digest, :plugin_id, :generation_after,
                :work_admission_id, :receipt_digest, NULL, NULL, :admitted_at_ms
            )"#,
            named_params! {
                ":installation_id_digest": installation_id_digest,
                ":plugin_id": plugin_id,
                ":generation_after": generation_after,
                ":work_admission_id": work_admission_id,
                ":receipt_digest": receipt_digest,
                ":admitted_at_ms": admitted_at_ms,
            },
        )
    } else {
        transaction.execute(
            r#"UPDATE compute_plugin_work_admission_heads SET
                work_admission_generation = :generation_after,
                work_admission_id = :work_admission_id,
                receipt_digest = :receipt_digest,
                previous_work_admission_id = :previous_id,
                previous_work_admission_receipt_digest = :previous_receipt_digest,
                updated_at_ms = :admitted_at_ms
            WHERE installation_id_digest = :installation_id_digest
              AND plugin_id = :plugin_id
              AND work_admission_generation = :generation_before
              AND work_admission_id = :previous_id
              AND receipt_digest = :previous_receipt_digest
              AND updated_at_ms < :admitted_at_ms"#,
            named_params! {
                ":installation_id_digest": installation_id_digest,
                ":plugin_id": plugin_id,
                ":generation_before": generation_before,
                ":generation_after": generation_after,
                ":work_admission_id": work_admission_id,
                ":receipt_digest": receipt_digest,
                ":previous_id": previous_id,
                ":previous_receipt_digest": previous_receipt_digest,
                ":admitted_at_ms": admitted_at_ms,
            },
        )
    }
    .context("COMPUTE_PLUGIN_WORK_ADMISSION_HEAD_CAS")?;
    if changed != 1 {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_HEAD_CAS_CHANGED");
    }
    Ok(())
}
