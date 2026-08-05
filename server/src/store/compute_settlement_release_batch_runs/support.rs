use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    ComputeSettlementReleaseBatchHistoryItem, ComputeSettlementReleaseBatchReport,
    ComputeSettlementReleaseBatchStart, StartComputeSettlementReleaseBatch,
};
use crate::store::ComputeSettlementReleaseCandidatePage;

const BATCH_HISTORY_CURSOR_VERSION: u8 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BatchHistoryCursor {
    v: u8,
    started_at: String,
    batch_run_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct StoredBatchRun {
    batch_run_id: String,
    requested_by_user_id: String,
    requested_limit: i64,
    cursor_present: i64,
    request_json: String,
    request_digest: String,
    candidate_page_json: String,
    candidate_page_digest: String,
    started_at: String,
    report_json: Option<String>,
    report_digest: Option<String>,
    completed_at: Option<String>,
}

impl StoredBatchRun {
    fn audit(
        &self,
        expected_request_digest: Option<&str>,
    ) -> Result<(
        StartComputeSettlementReleaseBatch,
        ComputeSettlementReleaseCandidatePage,
        Option<ComputeSettlementReleaseBatchReport>,
    )> {
        let request: StartComputeSettlementReleaseBatch =
            serde_json::from_str(&self.request_json).context("批次请求 JSON 无效")?;
        let normalized_request = normalize_start_request(&request)?;
        let actual_request_digest = request_digest(&self.request_json);
        if actual_request_digest != self.request_digest
            || normalized_request != request
            || expected_request_digest.is_some_and(|expected| expected != actual_request_digest)
            || request.requested_by_user_id != self.requested_by_user_id
            || request.requested_limit as i64 != self.requested_limit
            || (if request.requested_cursor.is_some() {
                1_i64
            } else {
                0_i64
            }) != self.cursor_present
        {
            bail!("到期结算释放批次请求审计不一致");
        }
        let page: ComputeSettlementReleaseCandidatePage =
            serde_json::from_str(&self.candidate_page_json).context("批次候选页 JSON 无效")?;
        if candidate_page_digest(&self.candidate_page_json) != self.candidate_page_digest
            || page.limit != request.requested_limit
            || page.candidates.len() > page.limit
            || page.total_due_candidates < page.candidates.len()
            || page.has_more != page.next_cursor.is_some()
        {
            bail!("到期结算释放批次候选页审计不一致");
        }
        DateTime::parse_from_rfc3339(&page.as_of).context("批次候选页时间无效")?;
        DateTime::parse_from_rfc3339(&self.started_at).context("批次开始时间无效")?;
        let report = match (&self.report_json, &self.report_digest, &self.completed_at) {
            (None, None, None) => None,
            (Some(json), Some(digest), Some(completed_at)) => {
                DateTime::parse_from_rfc3339(completed_at).context("批次完成时间无效")?;
                if complete_report_digest(json) != *digest {
                    bail!("到期结算释放批次完成报告摘要不一致");
                }
                let report: ComputeSettlementReleaseBatchReport =
                    serde_json::from_str(json).context("批次完成报告 JSON 无效")?;
                audit_report(&report, Some(&page))?;
                if report.batch_run_id != self.batch_run_id || report.replayed {
                    bail!("到期结算释放批次完成报告身份不一致");
                }
                Some(report)
            }
            _ => bail!("到期结算释放批次完成回执不完整"),
        };
        Ok((request, page, report))
    }

    pub(super) fn into_start(
        self,
        expected_request_digest: Option<&str>,
    ) -> Result<ComputeSettlementReleaseBatchStart> {
        let (_, candidate_page, completed_report) = self.audit(expected_request_digest)?;
        Ok(ComputeSettlementReleaseBatchStart {
            batch_run_id: self.batch_run_id,
            candidate_page,
            completed_report,
        })
    }

    pub(super) fn into_history_item(self) -> Result<ComputeSettlementReleaseBatchHistoryItem> {
        let (_, page, report) = self.audit(None)?;
        let eligible = page
            .candidates
            .iter()
            .filter(|candidate| candidate.eligible)
            .count();
        Ok(ComputeSettlementReleaseBatchHistoryItem {
            batch_run_id: self.batch_run_id,
            requested_by_user_id: self.requested_by_user_id,
            requested_limit: usize::try_from(self.requested_limit)
                .context("批次请求上限超出平台范围")?,
            requested_cursor_present: self.cursor_present != 0,
            total_due_candidates: page.total_due_candidates,
            scanned: page.candidates.len(),
            eligible,
            released: report.as_ref().map(|report| report.released.len()),
            skipped: report.as_ref().map(|report| report.skipped.len()),
            failed: report.as_ref().map(|report| report.failed.len()),
            status: if report.is_some() {
                "completed"
            } else {
                "incomplete"
            }
            .to_string(),
            started_at: self.started_at,
            completed_at: self.completed_at,
            candidate_page_digest: self.candidate_page_digest,
            report_digest: self.report_digest,
            audit_status: "consistent".to_string(),
            transaction_scope: "one_independent_begin_immediate_transaction_per_settlement"
                .to_string(),
            money_effect: "history_only_no_additional_balance_change".to_string(),
            external_transfer_effect: "none".to_string(),
        })
    }
}

pub(super) fn normalize_start_request(
    input: &StartComputeSettlementReleaseBatch,
) -> Result<StartComputeSettlementReleaseBatch> {
    let requested_by_user_id = validate_text("批次操作人", &input.requested_by_user_id, 200)?;
    if !(1..=100).contains(&input.requested_limit) {
        bail!("批次候选上限必须介于 1 至 100");
    }
    let requested_cursor = input
        .requested_cursor
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| validate_text("批次游标", value, 2_000))
        .transpose()?;
    let idempotency_key = validate_text("批次幂等键", &input.idempotency_key, 200)?;
    Ok(StartComputeSettlementReleaseBatch {
        requested_by_user_id,
        requested_limit: input.requested_limit,
        requested_cursor,
        idempotency_key,
    })
}

pub(super) fn audit_report(
    report: &ComputeSettlementReleaseBatchReport,
    page: Option<&ComputeSettlementReleaseCandidatePage>,
) -> Result<()> {
    if report.schema != "compute_federation.settlement_release_batch_report.v2"
        || report.batch_run_id.trim().is_empty()
        || report.replayed
        || report.total_due_candidates < report.scanned
        || report.has_more != report.next_cursor.is_some()
        || report.released.len() + report.skipped.len() + report.failed.len() != report.scanned
        || report.released.len() + report.failed.len() != report.eligible
        || report.transaction_scope != "one_independent_begin_immediate_transaction_per_settlement"
        || report.money_effect != "eligible_provider_and_platform_pending_moved_to_available"
        || report.external_transfer_effect != "none"
    {
        bail!("到期结算释放批次完成报告结构不一致");
    }
    if let Some(page) = page {
        let eligible = page
            .candidates
            .iter()
            .filter(|candidate| candidate.eligible)
            .count();
        if report.scanned != page.candidates.len()
            || report.eligible != eligible
            || report.total_due_candidates != page.total_due_candidates
            || report.has_more != page.has_more
            || report.next_cursor != page.next_cursor
        {
            bail!("到期结算释放批次完成报告与原候选页不一致");
        }
    }
    Ok(())
}

pub(super) fn batch_by_idempotency_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<StoredBatchRun>> {
    conn.query_row(
        &format!(
            "{} WHERE r.idempotency_scope=?1 AND r.idempotency_key=?2",
            batch_select()
        ),
        params![idempotency_scope, idempotency_key],
        stored_batch_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn batch_by_id_on(
    conn: &Connection,
    batch_run_id: &str,
) -> Result<Option<StoredBatchRun>> {
    conn.query_row(
        &format!("{} WHERE r.batch_run_id=?1", batch_select()),
        params![batch_run_id],
        stored_batch_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn history_rows_on(
    conn: &Connection,
    limit: usize,
    cursor: Option<&str>,
) -> Result<(Vec<StoredBatchRun>, bool)> {
    let cursor = decode_cursor(cursor)?;
    let (cursor_started_at, cursor_batch_run_id) = cursor
        .as_ref()
        .map(|cursor| {
            (
                Some(cursor.started_at.as_str()),
                Some(cursor.batch_run_id.as_str()),
            )
        })
        .unwrap_or((None, None));
    let mut stmt = conn.prepare(
        "SELECT r.batch_run_id, r.requested_by_user_id, r.requested_limit,
                r.cursor_present, r.request_json, r.request_digest,
                r.candidate_page_json, r.candidate_page_digest, r.started_at,
                c.report_json, c.report_digest, c.completed_at
           FROM compute_settlement_release_batch_runs r
           LEFT JOIN compute_settlement_release_batch_completions c
             ON c.batch_run_id=r.batch_run_id
          WHERE ?1 IS NULL
             OR r.started_at<?1
             OR (r.started_at=?1 AND r.batch_run_id<?2)
          ORDER BY r.started_at DESC, r.batch_run_id DESC
          LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![
            cursor_started_at,
            cursor_batch_run_id,
            limit.saturating_add(1) as i64
        ],
        stored_batch_from_row,
    )?;
    let mut rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    Ok((rows, has_more))
}

pub(super) fn next_history_cursor(
    item: &ComputeSettlementReleaseBatchHistoryItem,
) -> Result<String> {
    let bytes = serde_json::to_vec(&BatchHistoryCursor {
        v: BATCH_HISTORY_CURSOR_VERSION,
        started_at: item.started_at.clone(),
        batch_run_id: item.batch_run_id.clone(),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(super) fn request_digest(json: &str) -> String {
    digest_bytes(json.as_bytes())
}

pub(super) fn candidate_page_digest(json: &str) -> String {
    digest_bytes(json.as_bytes())
}

pub(super) fn complete_report_digest(json: &str) -> String {
    digest_bytes(json.as_bytes())
}

fn batch_select() -> &'static str {
    "SELECT r.batch_run_id, r.requested_by_user_id, r.requested_limit,
            r.cursor_present, r.request_json, r.request_digest,
            r.candidate_page_json, r.candidate_page_digest, r.started_at,
            c.report_json, c.report_digest, c.completed_at
       FROM compute_settlement_release_batch_runs r
       LEFT JOIN compute_settlement_release_batch_completions c
         ON c.batch_run_id=r.batch_run_id"
}

fn stored_batch_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredBatchRun> {
    Ok(StoredBatchRun {
        batch_run_id: row.get(0)?,
        requested_by_user_id: row.get(1)?,
        requested_limit: row.get(2)?,
        cursor_present: row.get(3)?,
        request_json: row.get(4)?,
        request_digest: row.get(5)?,
        candidate_page_json: row.get(6)?,
        candidate_page_digest: row.get(7)?,
        started_at: row.get(8)?,
        report_json: row.get(9)?,
        report_digest: row.get(10)?,
        completed_at: row.get(11)?,
    })
}

fn decode_cursor(raw: Option<&str>) -> Result<Option<BatchHistoryCursor>> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .map_err(|_| anyhow!("到期结算释放批次历史游标无效"))?;
    let cursor: BatchHistoryCursor =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("到期结算释放批次历史游标无效"))?;
    if cursor.v != BATCH_HISTORY_CURSOR_VERSION
        || cursor.started_at.trim().is_empty()
        || cursor.batch_run_id.trim().is_empty()
    {
        bail!("到期结算释放批次历史游标无效或已过期");
    }
    DateTime::parse_from_rfc3339(&cursor.started_at).context("到期结算释放批次历史游标时间无效")?;
    Ok(Some(cursor))
}

fn validate_text(label: &str, value: &str, max: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        bail!("{label}为空、过长或包含控制字符");
    }
    Ok(value.to_string())
}

fn digest_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
