use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::open_commerce_data_erasure_evidence_model::{
    OpenCommerceDataErasureEvidence, ERASURE_EVIDENCE_SOURCE_AUTHORITY,
};

use super::{new_id, now, Store};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_open_commerce_data_erasure_evidence(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        request_id: &str,
        submitted_by_user_id: &str,
        evidence_kind: &str,
        external_system: &str,
        reference_id: &str,
        receipt_sha256: &str,
        summary: &str,
    ) -> Result<(OpenCommerceDataErasureEvidence, bool)> {
        self.open_commerce_merchant_for_project(merchant_project_id, merchant_id)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let request_status = tx
            .query_row(
                "SELECT status FROM open_commerce_consumer_data_requests
                  WHERE id=?1 AND merchant_project_id=?2 AND merchant_id=?3",
                params![
                    request_id.trim(),
                    merchant_project_id.trim(),
                    merchant_id.trim()
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者数据删除请求不存在"))?;
        if request_status != "completed" {
            bail!("只有商户已声明完成的删除请求可以附加外部证明");
        }
        if let Some(existing) = tx
            .query_row(
                &format!(
                    "{ERASURE_EVIDENCE_SELECT}
                      WHERE data_request_id=?1 AND external_system=?2 AND receipt_sha256=?3"
                ),
                params![
                    request_id.trim(),
                    external_system.trim(),
                    receipt_sha256.trim()
                ],
                erasure_evidence_from_row,
            )
            .optional()?
        {
            tx.commit()?;
            return Ok((existing, false));
        }
        let id = new_id("erasure_evidence");
        let timestamp = now();
        tx.execute(
            "INSERT INTO open_commerce_data_erasure_evidence (
               id, data_request_id, merchant_project_id, merchant_id, evidence_kind,
               external_system, reference_id, receipt_sha256, summary,
               submitted_by_user_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                request_id.trim(),
                merchant_project_id.trim(),
                merchant_id.trim(),
                evidence_kind.trim(),
                external_system.trim(),
                reference_id.trim(),
                receipt_sha256.trim(),
                summary.trim(),
                submitted_by_user_id.trim(),
                timestamp,
            ],
        )?;
        let evidence = tx.query_row(
            &format!("{ERASURE_EVIDENCE_SELECT} WHERE id=?1"),
            params![id],
            erasure_evidence_from_row,
        )?;
        tx.commit()?;
        Ok((evidence, true))
    }

    pub(crate) fn list_open_commerce_consumer_data_erasure_evidence(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceDataErasureEvidence>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{ERASURE_EVIDENCE_SELECT}
              JOIN open_commerce_consumer_data_requests r ON r.id=e.data_request_id
             WHERE r.consumer_project_id=?1 AND r.consumer_user_id=?2
             ORDER BY e.created_at DESC, e.rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 500) as i64
            ],
            erasure_evidence_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn list_open_commerce_merchant_data_erasure_evidence(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceDataErasureEvidence>> {
        self.open_commerce_merchant_for_project(merchant_project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{ERASURE_EVIDENCE_SELECT}
             WHERE e.merchant_project_id=?1 AND e.merchant_id=?2
             ORDER BY e.created_at DESC, e.rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                merchant_project_id.trim(),
                merchant_id.trim(),
                limit.clamp(1, 500) as i64
            ],
            erasure_evidence_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

pub(super) fn erasure_evidence_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<OpenCommerceDataErasureEvidence> {
    Ok(OpenCommerceDataErasureEvidence {
        id: row.get(0)?,
        data_request_id: row.get(1)?,
        merchant_id: row.get(2)?,
        evidence_kind: row.get(3)?,
        external_system: row.get(4)?,
        reference_id: row.get(5)?,
        receipt_sha256: row.get(6)?,
        summary: row.get(7)?,
        source_authority: ERASURE_EVIDENCE_SOURCE_AUTHORITY,
        platform_verified: false,
        created_at: row.get(8)?,
    })
}

pub(super) const ERASURE_EVIDENCE_SELECT: &str = "SELECT e.id, e.data_request_id, e.merchant_id,
       e.evidence_kind, e.external_system, e.reference_id, e.receipt_sha256,
       e.summary, e.created_at
  FROM open_commerce_data_erasure_evidence e";
