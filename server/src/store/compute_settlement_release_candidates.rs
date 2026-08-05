use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::{
    compute_attempt_settlement_challenges::{
        settlement_challenge_gate_on, ComputeSettlementChallengeGate,
        COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
    },
    compute_attempt_settlements::compute_attempt_settlement_on,
    Store,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementReleaseCandidate {
    pub lease_id: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub settlement_posting_id: String,
    pub settlement_posting_digest: String,
    pub settled_at: String,
    pub challenge_deadline: String,
    pub challenge_gate: ComputeSettlementChallengeGate,
    pub eligible: bool,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementReleaseCandidatePage {
    pub schema: String,
    pub as_of: String,
    pub limit: usize,
    pub total_due_candidates: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub candidates: Vec<ComputeSettlementReleaseCandidate>,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ComputeSettlementReleaseCandidateCursor {
    v: u8,
    settled_at: String,
    lease_id: String,
}

const RELEASE_CANDIDATE_CURSOR_VERSION: u8 = 1;

impl Store {
    pub(crate) fn list_due_compute_settlement_release_candidates(
        &self,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<ComputeSettlementReleaseCandidatePage> {
        let limit = limit.clamp(1, 100);
        let cursor = decode_cursor(cursor)?;
        let as_of = Utc::now();
        let cutoff = as_of
            .checked_sub_signed(Duration::seconds(
                COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
            ))
            .context("到期结算释放扫描截止时间超出范围")?;
        let conn = self.conn()?;
        let total_due_candidates = total_due_candidates_on(&conn, &cutoff.to_rfc3339())?;
        let lease_ids = due_lease_ids_on(
            &conn,
            &cutoff.to_rfc3339(),
            cursor.as_ref(),
            limit.saturating_add(1),
        )?;
        let has_more = lease_ids.len() > limit;
        let candidates = lease_ids
            .into_iter()
            .take(limit)
            .map(|lease_id| candidate_on(&conn, &lease_id, &as_of))
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if has_more {
            candidates.last().map(encode_cursor).transpose()?
        } else {
            None
        };
        Ok(ComputeSettlementReleaseCandidatePage {
            schema: "compute_federation.settlement_release_candidate_page.v1".to_string(),
            as_of: as_of.to_rfc3339(),
            limit,
            total_due_candidates,
            has_more,
            next_cursor,
            candidates,
            money_effect: "read_only_no_balance_change".to_string(),
            external_transfer_effect: "none".to_string(),
        })
    }
}

fn total_due_candidates_on(conn: &Connection, cutoff: &str) -> Result<usize> {
    let count = conn.query_row(
        "SELECT COUNT(*)
           FROM compute_attempt_settlements s
           LEFT JOIN compute_settlement_releases r
             ON r.settlement_receipt_id=s.settlement_receipt_id
          WHERE r.release_id IS NULL AND s.settled_at<=?1",
        params![cutoff],
        |row| row.get::<_, i64>(0),
    )?;
    usize::try_from(count).context("到期结算释放候选总数超出平台范围")
}

fn due_lease_ids_on(
    conn: &Connection,
    cutoff: &str,
    cursor: Option<&ComputeSettlementReleaseCandidateCursor>,
    limit: usize,
) -> Result<Vec<String>> {
    let (cursor_settled_at, cursor_lease_id) = cursor
        .map(|cursor| {
            (
                Some(cursor.settled_at.as_str()),
                Some(cursor.lease_id.as_str()),
            )
        })
        .unwrap_or((None, None));
    let mut stmt = conn.prepare(
        "SELECT s.lease_id
           FROM compute_attempt_settlements s
           LEFT JOIN compute_settlement_releases r
             ON r.settlement_receipt_id=s.settlement_receipt_id
          WHERE r.release_id IS NULL
            AND s.settled_at<=?1
            AND (
              ?2 IS NULL
              OR s.settled_at>?2
              OR (s.settled_at=?2 AND s.lease_id>?3)
            )
          ORDER BY s.settled_at ASC, s.lease_id ASC
          LIMIT ?4",
    )?;
    let rows = stmt.query_map(
        params![cutoff, cursor_settled_at, cursor_lease_id, limit as i64],
        |row| row.get(0),
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn decode_cursor(raw: Option<&str>) -> Result<Option<ComputeSettlementReleaseCandidateCursor>> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .map_err(|_| anyhow!("到期结算释放候选游标无效"))?;
    let cursor: ComputeSettlementReleaseCandidateCursor =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("到期结算释放候选游标无效"))?;
    if cursor.v != RELEASE_CANDIDATE_CURSOR_VERSION || cursor.lease_id.trim().is_empty() {
        bail!("到期结算释放候选游标无效或已过期");
    }
    DateTime::parse_from_rfc3339(&cursor.settled_at)
        .context("到期结算释放候选游标的结算时间无效")?;
    Ok(Some(cursor))
}

fn encode_cursor(candidate: &ComputeSettlementReleaseCandidate) -> Result<String> {
    let bytes = serde_json::to_vec(&ComputeSettlementReleaseCandidateCursor {
        v: RELEASE_CANDIDATE_CURSOR_VERSION,
        settled_at: candidate.settled_at.clone(),
        lease_id: candidate.lease_id.clone(),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn candidate_on(
    conn: &Connection,
    lease_id: &str,
    as_of: &DateTime<Utc>,
) -> Result<ComputeSettlementReleaseCandidate> {
    let settlement = compute_attempt_settlement_on(conn, lease_id)?;
    let settled_at = DateTime::parse_from_rfc3339(&settlement.settled_at)
        .context("Settlement 结算时间不是 RFC3339")?
        .with_timezone(&Utc);
    let challenge_deadline = settled_at
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("到期结算释放挑战截止时间超出范围")?;
    if as_of < &challenge_deadline {
        bail!("到期候选查询返回了尚未度过挑战窗口的 Settlement");
    }
    let gate = settlement_challenge_gate_on(conn, &settlement.settlement.settlement_receipt_id)?;
    let eligible = !gate.blocked && !gate.correction_required;
    let blocked_reason = if eligible {
        None
    } else {
        Some(format!("challenge_gate:{}", gate.status))
    };
    Ok(ComputeSettlementReleaseCandidate {
        lease_id: settlement.lease_id,
        settlement_receipt_id: settlement.settlement.settlement_receipt_id,
        settlement_event_digest: settlement.event_digest,
        settlement_posting_id: settlement.posting_id,
        settlement_posting_digest: settlement.posting_digest,
        settled_at: settlement.settled_at,
        challenge_deadline: challenge_deadline.to_rfc3339(),
        challenge_gate: gate,
        eligible,
        blocked_reason,
    })
}
