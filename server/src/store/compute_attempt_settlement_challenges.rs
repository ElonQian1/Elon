use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::{
    compute_attempt_settlement_challenge_resolutions::settlement_challenge_resolution_by_challenge_on,
    compute_attempt_settlement_corrections::compute_settlement_correction_by_resolution_on, Store,
};

mod pending_candidate;
mod pending_queue;
mod support;

use pending_candidate::build_pending_challenge_candidate_on;
use pending_queue::list_pending_challenge_lease_ids_on;
use support::{
    challenge_by_idempotency_on, challenge_by_lease_on, challenge_by_settlement_on,
    normalize_challenge_request, persist_challenge_on, settlement_challenge_request_digest,
};

pub(crate) const COMPUTE_SETTLEMENT_CHALLENGE_SCHEMA: &str =
    "compute_federation.settlement_challenge.v1";
pub(crate) const COMPUTE_SETTLEMENT_CHALLENGE_POLICY_ID: &str = "consumer_challenge_72h_v1";
pub(crate) const COMPUTE_SETTLEMENT_CHALLENGE_POLICY_VERSION: i64 = 1;
pub(crate) const COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS: i64 = 72 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpenComputeSettlementChallengeRequest {
    pub lease_id: String,
    pub expected_settlement_receipt_id: String,
    pub expected_settlement_event_digest: String,
    pub expected_posting_id: String,
    pub expected_posting_digest: String,
    pub reason_code: String,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub opened_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementChallengeReceipt {
    pub schema: String,
    pub challenge_id: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub posting_id: String,
    pub posting_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub challenge_deadline: String,
    pub status: String,
    pub reason_code: String,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub opened_by_user_id: String,
    pub opened_at: String,
    pub balance_effect: String,
    pub settlement_release_effect: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementChallengeGate {
    pub status: String,
    pub blocked: bool,
    pub correction_required: bool,
    pub challenge_id: Option<String>,
    pub challenge_event_digest: Option<String>,
    pub resolution_id: Option<String>,
    pub resolution_event_digest: Option<String>,
    pub correction_id: Option<String>,
    pub correction_event_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputePendingSettlementChallengeCandidate {
    pub settlement: super::compute_attempt_settlements::ComputeAttemptSettlementReceipt,
    pub challenge_deadline: String,
    pub balance_effect: &'static str,
    pub settlement_release_effect: &'static str,
    pub external_payment_effect: &'static str,
}

impl Store {
    pub(crate) fn open_compute_settlement_challenge(
        &self,
        input: &OpenComputeSettlementChallengeRequest,
    ) -> Result<ComputeSettlementChallengeReceipt> {
        let input = normalize_challenge_request(input)?;
        let request_digest = settlement_challenge_request_digest(&input)?;
        let idempotency_scope = format!("compute_settlement_challenge:{}", input.opened_by_user_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            challenge_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同算力结算挑战幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = support::open_challenge_on(&tx, &input, &request_digest)?;
        if let Some(stored) = challenge_by_settlement_on(&tx, &receipt.settlement_receipt_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Settlement Receipt 已有另一份消费者挑战");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        persist_challenge_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = challenge_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("算力结算挑战写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_challenge(
        &self,
        lease_id: &str,
    ) -> Result<ComputeSettlementChallengeReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_settlement_challenge_on(&*conn, lease_id)
    }

    pub(crate) fn list_pending_compute_settlement_challenges(
        &self,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputePendingSettlementChallengeCandidate>> {
        support::validate_exact("消费者用户 ID", consumer_user_id, 240)?;
        let limit = limit.clamp(1, 100);
        let as_of = chrono::Utc::now();
        let cutoff = as_of
            .checked_sub_signed(chrono::Duration::seconds(
                COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
            ))
            .ok_or_else(|| anyhow::anyhow!("结算挑战候选扫描截止时间超出范围"))?;
        let conn = self.conn()?;
        list_pending_challenge_lease_ids_on(
            &conn,
            consumer_user_id,
            &cutoff.to_rfc3339(),
            &as_of.to_rfc3339(),
            limit,
        )?
        .into_iter()
        .map(|lease_id| {
            build_pending_challenge_candidate_on(&conn, &lease_id, consumer_user_id, &as_of)
        })
        .collect()
    }
}

pub(super) fn compute_settlement_challenge_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = challenge_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 结算尚无消费者挑战"))?;
    stored.into_receipt(conn, false)
}

pub(super) fn settlement_has_open_challenge_on(
    conn: &Connection,
    settlement_receipt_id: &str,
) -> Result<bool> {
    Ok(settlement_challenge_gate_on(conn, settlement_receipt_id)?.blocked)
}

pub(super) fn settlement_challenge_gate_on(
    conn: &Connection,
    settlement_receipt_id: &str,
) -> Result<ComputeSettlementChallengeGate> {
    support::validate_exact("Settlement Receipt ID", settlement_receipt_id, 240)?;
    let Some(stored) = challenge_by_settlement_on(conn, settlement_receipt_id)? else {
        return Ok(ComputeSettlementChallengeGate {
            status: "none".to_string(),
            blocked: false,
            correction_required: false,
            challenge_id: None,
            challenge_event_digest: None,
            resolution_id: None,
            resolution_event_digest: None,
            correction_id: None,
            correction_event_digest: None,
        });
    };
    let challenge = stored.into_receipt(conn, false)?;
    let Some(resolution) =
        settlement_challenge_resolution_by_challenge_on(conn, &challenge.challenge_id)?
    else {
        return Ok(ComputeSettlementChallengeGate {
            status: "open".to_string(),
            blocked: true,
            correction_required: false,
            challenge_id: Some(challenge.challenge_id),
            challenge_event_digest: Some(challenge.event_digest),
            resolution_id: None,
            resolution_event_digest: None,
            correction_id: None,
            correction_event_digest: None,
        });
    };
    if resolution.action == "accepted" {
        if let Some(correction) =
            compute_settlement_correction_by_resolution_on(conn, &resolution.resolution_id)?
        {
            return Ok(ComputeSettlementChallengeGate {
                status: "accepted_corrected".to_string(),
                blocked: false,
                correction_required: false,
                challenge_id: Some(challenge.challenge_id),
                challenge_event_digest: Some(challenge.event_digest),
                resolution_id: Some(resolution.resolution_id),
                resolution_event_digest: Some(resolution.event_digest),
                correction_id: Some(correction.correction_id),
                correction_event_digest: Some(correction.event_digest),
            });
        }
    }
    let (blocked, correction_required) = match resolution.action.as_str() {
        "accepted" => (true, true),
        "rejected" | "withdrawn" => (false, false),
        _ => bail!("算力结算挑战决议包含未知终态"),
    };
    Ok(ComputeSettlementChallengeGate {
        status: resolution.action,
        blocked,
        correction_required,
        challenge_id: Some(challenge.challenge_id),
        challenge_event_digest: Some(challenge.event_digest),
        resolution_id: Some(resolution.resolution_id),
        resolution_event_digest: Some(resolution.event_digest),
        correction_id: None,
        correction_event_digest: None,
    })
}
