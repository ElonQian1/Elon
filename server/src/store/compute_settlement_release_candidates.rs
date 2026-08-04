use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::{
    compute_attempt_settlement_challenges::{
        settlement_challenge_gate_on, ComputeSettlementChallengeGate,
        COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
    },
    compute_attempt_settlements::compute_attempt_settlement_on,
    Store,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementReleaseCandidatePage {
    pub schema: String,
    pub as_of: String,
    pub limit: usize,
    pub candidates: Vec<ComputeSettlementReleaseCandidate>,
    pub money_effect: String,
    pub external_transfer_effect: String,
}

impl Store {
    pub(crate) fn list_due_compute_settlement_release_candidates(
        &self,
        limit: usize,
    ) -> Result<ComputeSettlementReleaseCandidatePage> {
        let limit = limit.clamp(1, 100);
        let as_of = Utc::now();
        let cutoff = as_of
            .checked_sub_signed(Duration::seconds(
                COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
            ))
            .context("到期结算释放扫描截止时间超出范围")?;
        let conn = self.conn()?;
        let lease_ids = due_lease_ids_on(&conn, &cutoff.to_rfc3339(), limit)?;
        let candidates = lease_ids
            .into_iter()
            .map(|lease_id| candidate_on(&conn, &lease_id, &as_of))
            .collect::<Result<Vec<_>>>()?;
        Ok(ComputeSettlementReleaseCandidatePage {
            schema: "compute_federation.settlement_release_candidate_page.v1".to_string(),
            as_of: as_of.to_rfc3339(),
            limit,
            candidates,
            money_effect: "read_only_no_balance_change".to_string(),
            external_transfer_effect: "none".to_string(),
        })
    }
}

fn due_lease_ids_on(conn: &Connection, cutoff: &str, limit: usize) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT s.lease_id
           FROM compute_attempt_settlements s
           LEFT JOIN compute_settlement_releases r
             ON r.settlement_receipt_id=s.settlement_receipt_id
          WHERE r.release_id IS NULL AND s.settled_at<=?1
          ORDER BY s.settled_at ASC, s.lease_id ASC
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![cutoff, limit as i64], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
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
