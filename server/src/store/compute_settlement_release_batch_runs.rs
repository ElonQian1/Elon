use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ComputeSettlementReleaseCandidate, ComputeSettlementReleaseCandidatePage,
    ComputeSettlementReleaseReceipt, Store,
};

mod support;

use support::{
    batch_by_id_on, batch_by_idempotency_on, candidate_page_digest, complete_report_digest,
    history_rows_on, next_history_cursor, normalize_start_request, request_digest,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartComputeSettlementReleaseBatch {
    pub requested_by_user_id: String,
    pub requested_limit: usize,
    pub requested_cursor: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchFailure {
    pub lease_id: String,
    pub settlement_receipt_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchReport {
    pub schema: String,
    pub batch_run_id: String,
    pub replayed: bool,
    pub scanned: usize,
    pub eligible: usize,
    pub total_due_candidates: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub released: Vec<ComputeSettlementReleaseReceipt>,
    pub skipped: Vec<ComputeSettlementReleaseCandidate>,
    pub failed: Vec<ComputeSettlementReleaseBatchFailure>,
    pub transaction_scope: String,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ComputeSettlementReleaseBatchStart {
    pub batch_run_id: String,
    pub candidate_page: ComputeSettlementReleaseCandidatePage,
    pub completed_report: Option<ComputeSettlementReleaseBatchReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchHistoryItem {
    pub batch_run_id: String,
    pub requested_by_user_id: String,
    pub requested_limit: usize,
    pub requested_cursor_present: bool,
    pub total_due_candidates: usize,
    pub scanned: usize,
    pub eligible: usize,
    pub released: Option<usize>,
    pub skipped: Option<usize>,
    pub failed: Option<usize>,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub candidate_page_digest: String,
    pub report_digest: Option<String>,
    pub audit_status: String,
    pub transaction_scope: String,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementReleaseBatchHistoryPage {
    pub schema: String,
    pub as_of: String,
    pub limit: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub items: Vec<ComputeSettlementReleaseBatchHistoryItem>,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

impl Store {
    pub(crate) fn start_or_resume_compute_settlement_release_batch(
        &self,
        request: &StartComputeSettlementReleaseBatch,
        current_page: &ComputeSettlementReleaseCandidatePage,
    ) -> Result<ComputeSettlementReleaseBatchStart> {
        let request = normalize_start_request(request)?;
        let request_json = serde_json::to_string(&request)?;
        let request_digest = request_digest(&request_json);
        let idempotency_scope = format!(
            "compute_settlement_release_batch:{}",
            request.requested_by_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            batch_by_idempotency_on(&tx, &idempotency_scope, &request.idempotency_key)?
        {
            let start = stored.into_start(Some(&request_digest))?;
            tx.commit()?;
            return Ok(start);
        }

        let candidate_page_json = serde_json::to_string(current_page)?;
        let candidate_page_digest = candidate_page_digest(&candidate_page_json);
        let batch_run_id = format!("csrb_{}", Uuid::new_v4().simple());
        let started_at = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO compute_settlement_release_batch_runs (
               batch_run_id, requested_by_user_id, requested_limit, cursor_present,
               request_json, request_digest, candidate_page_json, candidate_page_digest,
               idempotency_scope, idempotency_key, started_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                batch_run_id,
                request.requested_by_user_id,
                request.requested_limit as i64,
                if request.requested_cursor.is_some() {
                    1_i64
                } else {
                    0_i64
                },
                request_json,
                request_digest,
                candidate_page_json,
                candidate_page_digest,
                idempotency_scope,
                request.idempotency_key,
                started_at,
            ],
        )?;
        let stored = batch_by_idempotency_on(&tx, &idempotency_scope, &request.idempotency_key)?
            .ok_or_else(|| anyhow!("到期结算释放批次意图写入后不可见"))?;
        let start = stored.into_start(Some(&request_digest))?;
        tx.commit()?;
        Ok(start)
    }

    pub(crate) fn complete_compute_settlement_release_batch(
        &self,
        batch_run_id: &str,
        report: &ComputeSettlementReleaseBatchReport,
    ) -> Result<(ComputeSettlementReleaseBatchReport, bool)> {
        if batch_run_id.trim().is_empty() || report.batch_run_id != batch_run_id {
            bail!("批次完成报告与运行 ID 不一致");
        }
        let mut stored_report = report.clone();
        stored_report.replayed = false;
        support::audit_report(&stored_report, None)?;
        let report_json = serde_json::to_string(&stored_report)?;
        let report_digest = complete_report_digest(&report_json);
        let completed_at = Utc::now().to_rfc3339();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = batch_by_id_on(&tx, batch_run_id)?
            .ok_or_else(|| anyhow!("到期结算释放批次意图不存在"))?;
        let audited = existing.into_start(None)?;
        if let Some(existing_report) = audited.completed_report {
            tx.commit()?;
            return Ok((existing_report, true));
        }
        tx.execute(
            "INSERT INTO compute_settlement_release_batch_completions (
               batch_run_id, report_json, report_digest, completed_at
             ) VALUES (?1, ?2, ?3, ?4)",
            params![batch_run_id, report_json, report_digest, completed_at],
        )?;
        let completed = batch_by_id_on(&tx, batch_run_id)?
            .ok_or_else(|| anyhow!("到期结算释放批次完成回执写入后不可见"))?
            .into_start(None)?
            .completed_report
            .ok_or_else(|| anyhow!("到期结算释放批次完成回执写入后不可见"))?;
        tx.commit()?;
        Ok((completed, false))
    }

    pub(crate) fn list_compute_settlement_release_batch_history(
        &self,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<ComputeSettlementReleaseBatchHistoryPage> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;
        let (stored, has_more) = history_rows_on(&conn, limit, cursor)?;
        let items = stored
            .into_iter()
            .map(|stored| stored.into_history_item())
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if has_more {
            items.last().map(next_history_cursor).transpose()?
        } else {
            None
        };
        Ok(ComputeSettlementReleaseBatchHistoryPage {
            schema: "compute_federation.settlement_release_batch_history_page.v1".to_string(),
            as_of: Utc::now().to_rfc3339(),
            limit,
            has_more,
            next_cursor,
            items,
            money_effect: "read_only_no_balance_change".to_string(),
            external_transfer_effect: "none".to_string(),
        })
    }
}
