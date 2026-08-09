use anyhow::{Context, Result};
use rusqlite::{params, Transaction};

use super::super::head::WorkAdmissionHead;

pub(super) fn count_chain_membership(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    plugin_id: &str,
    head: &WorkAdmissionHead,
    expected_id: &str,
    expected_digest: &str,
    expected_generation: i64,
) -> Result<i64> {
    transaction
        .query_row(
            r#"WITH RECURSIVE chain(
                work_admission_id, receipt_digest, generation,
                previous_work_admission_id, previous_receipt_digest
            ) AS (
                SELECT receipt.work_admission_id, receipt.receipt_digest,
                    receipt.work_admission_generation_after,
                    receipt.previous_work_admission_id,
                    receipt.previous_work_admission_receipt_digest
                FROM compute_plugin_work_admission_receipts AS receipt
                WHERE receipt.work_admission_id = ?1 AND receipt.receipt_digest = ?2
                  AND receipt.plugin_id = ?3 AND receipt.installation_id_digest = ?4
                UNION ALL
                SELECT previous.work_admission_id, previous.receipt_digest,
                    previous.work_admission_generation_after,
                    previous.previous_work_admission_id,
                    previous.previous_work_admission_receipt_digest
                FROM compute_plugin_work_admission_receipts AS previous
                JOIN chain AS successor
                  ON previous.work_admission_id = successor.previous_work_admission_id
                 AND previous.receipt_digest = successor.previous_receipt_digest
                 AND previous.plugin_id = ?3
                 AND previous.installation_id_digest = ?4
            )
            SELECT COUNT(*) FROM chain
            WHERE work_admission_id = ?5 AND receipt_digest = ?6 AND generation = ?7"#,
            params![
                head.work_admission_id,
                head.receipt_digest,
                plugin_id,
                installation_id_digest,
                expected_id,
                expected_digest,
                expected_generation,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_CHAIN_READ")
}
