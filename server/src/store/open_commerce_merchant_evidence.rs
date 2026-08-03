use anyhow::{anyhow, Context, Result};
use rusqlite::{params, OptionalExtension};

use crate::open_commerce_merchant_evidence_model::MerchantTerminalInvocationRecord;

use super::{
    open_commerce_invocations::{invocation_from_row, INVOCATION_COLUMN_COUNT, INVOCATION_SELECT},
    Store,
};

impl Store {
    pub(crate) fn list_open_commerce_merchant_terminal_invocations(
        &self,
        project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<MerchantTerminalInvocationRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT i.*, e.seq
               FROM open_commerce_invocation_terminal_events e
               JOIN ({INVOCATION_SELECT}) i ON i.id = e.invocation_id
              WHERE i.project_id = ?1 AND i.merchant_id = ?2
              ORDER BY e.seq DESC
              LIMIT ?3"
        ))?;
        let records = stmt
            .query_map(
                params![
                    project_id.trim(),
                    merchant_id.trim(),
                    limit.clamp(1, 200) as i64
                ],
                merchant_record_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub(crate) fn open_commerce_merchant_terminal_invocation(
        &self,
        project_id: &str,
        merchant_id: &str,
        invocation_id: &str,
    ) -> Result<Option<MerchantTerminalInvocationRecord>> {
        self.conn()?
            .query_row(
                &format!(
                    "SELECT i.*, e.seq
                       FROM open_commerce_invocation_terminal_events e
                       JOIN ({INVOCATION_SELECT}) i ON i.id = e.invocation_id
                      WHERE i.project_id = ?1 AND i.merchant_id = ?2 AND i.id = ?3"
                ),
                params![project_id.trim(), merchant_id.trim(), invocation_id.trim()],
                merchant_record_from_row,
            )
            .optional()
            .map_err(|error| anyhow!(error).context("读取商户业务证据失败"))
    }
}

pub(super) fn merchant_record_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MerchantTerminalInvocationRecord> {
    Ok(MerchantTerminalInvocationRecord {
        sequence: row.get(INVOCATION_COLUMN_COUNT)?,
        invocation: invocation_from_row(row)?,
    })
}
