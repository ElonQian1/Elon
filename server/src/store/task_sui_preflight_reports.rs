use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::task_settlement::sui_preflight_model::{
    CreateSuiPreflightReport, SuiPreflightReport, SUI_PREFLIGHT_REPORT_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn record_task_sui_preflight_report(
        &self,
        input: CreateSuiPreflightReport<'_>,
    ) -> Result<SuiPreflightReport> {
        let conn = self.conn()?;
        insert_or_get_report(&conn, input)
    }

    pub(crate) fn list_task_sui_preflight_reports(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<SuiPreflightReport>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{REPORT_SELECT} WHERE project_id=?1 ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let reports = statement
            .query_map(
                params![project_id.trim(), limit.clamp(1, 500) as i64],
                report_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(reports)
    }
}

pub(super) fn insert_or_get_report(
    conn: &rusqlite::Connection,
    input: CreateSuiPreflightReport<'_>,
) -> Result<SuiPreflightReport> {
    let id = new_id("sui_preflight_report");
    let timestamp = now();
    conn.execute(
        "INSERT OR IGNORE INTO task_sui_preflight_reports (
               id, project_id, adapter_id, credential_version, package_kind,
               projection_package_id, target_network, handoff_digest,
               projection_digest, outcome, summary, tool_version,
               idempotency_key, report_digest, created_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
               ?13, ?14, ?15
             )",
        params![
            id,
            input.project_id.trim(),
            input.adapter_id.trim(),
            input.credential_version,
            input.package_kind.trim(),
            input.projection_package_id.trim(),
            input.target_network.trim(),
            input.handoff_digest.trim(),
            input.projection_digest.trim(),
            input.outcome.trim(),
            input.summary.trim(),
            input.tool_version.trim(),
            input.idempotency_key.trim(),
            input.report_digest.trim(),
            timestamp,
        ],
    )?;
    let report = conn
        .query_row(
            &format!("{REPORT_SELECT} WHERE adapter_id=?1 AND idempotency_key=?2"),
            params![input.adapter_id.trim(), input.idempotency_key.trim()],
            report_from_row,
        )
        .optional()
        .map_err(|error| anyhow!(error).context("读取 Sui 预检报告失败"))?
        .ok_or_else(|| anyhow!("Sui 预检报告写入后无法读取"))?;
    if report.report_digest != input.report_digest.trim() {
        bail!("同一 Sui 预检报告幂等键不能用于不同结果");
    }
    Ok(report)
}

pub(super) fn report_from_row(row: &Row<'_>) -> rusqlite::Result<SuiPreflightReport> {
    Ok(SuiPreflightReport {
        schema: SUI_PREFLIGHT_REPORT_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        adapter_id: row.get(2)?,
        credential_version: row.get(3)?,
        package_kind: row.get(4)?,
        projection_package_id: row.get(5)?,
        target_network: row.get(6)?,
        handoff_digest: row.get(7)?,
        projection_digest: row.get(8)?,
        outcome: row.get(9)?,
        summary: row.get(10)?,
        tool_version: row.get(11)?,
        idempotency_key: row.get(12)?,
        report_digest: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub(super) const REPORT_SELECT: &str =
    "SELECT id, project_id, adapter_id, credential_version, package_kind,
            projection_package_id, target_network, handoff_digest,
            projection_digest, outcome, summary, tool_version,
            idempotency_key, report_digest, created_at
       FROM task_sui_preflight_reports";
