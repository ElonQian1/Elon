use anyhow::{bail, Context, Result};
use chrono::Duration;
use rusqlite::Connection;

use super::super::super::compute_attempt_settlements::compute_attempt_settlement_on;
use super::super::{
    ComputeSettlementChallengeReceipt, COMPUTE_SETTLEMENT_CHALLENGE_POLICY_ID,
    COMPUTE_SETTLEMENT_CHALLENGE_POLICY_VERSION, COMPUTE_SETTLEMENT_CHALLENGE_SCHEMA,
    COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
};
use super::{
    evidence_refs_digest, normalize_challenge_request, parse_time,
    settlement_challenge_event_digest, settlement_challenge_request_digest, StoredChallenge,
};

pub(super) fn audited_challenge_on(
    conn: &Connection,
    stored: &StoredChallenge,
    replayed: bool,
) -> Result<ComputeSettlementChallengeReceipt> {
    let request = normalize_challenge_request(&serde_json::from_str(&stored.request_json)?)?;
    let evidence_refs: Vec<String> = serde_json::from_str(&stored.evidence_refs_json)?;
    let mut receipt: ComputeSettlementChallengeReceipt =
        serde_json::from_str(&stored.receipt_json)?;
    if request.lease_id != stored.lease_id
        || request.expected_settlement_receipt_id != stored.settlement_receipt_id
        || request.expected_settlement_event_digest != stored.settlement_event_digest
        || request.expected_posting_id != stored.posting_id
        || request.expected_posting_digest != stored.posting_digest
        || request.reason_code != stored.reason_code
        || request.summary != stored.summary
        || request.evidence_refs != evidence_refs
        || request.idempotency_key != stored.idempotency_key
        || request.opened_by_user_id != stored.opened_by_user_id
        || settlement_challenge_request_digest(&request)? != stored.request_digest
        || evidence_refs_digest(&evidence_refs)? != stored.evidence_refs_digest
        || stored.idempotency_scope
            != format!("compute_settlement_challenge:{}", stored.opened_by_user_id)
        || receipt.schema != COMPUTE_SETTLEMENT_CHALLENGE_SCHEMA
        || receipt.challenge_id != stored.challenge_id
        || receipt.settlement_receipt_id != stored.settlement_receipt_id
        || receipt.settlement_event_digest != stored.settlement_event_digest
        || receipt.lease_id != stored.lease_id
        || receipt.consumer_account_id != stored.consumer_account_id
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.posting_id != stored.posting_id
        || receipt.posting_digest != stored.posting_digest
        || receipt.policy_id != stored.policy_id
        || receipt.policy_version != stored.policy_version
        || receipt.challenge_deadline != stored.challenge_deadline
        || receipt.reason_code != stored.reason_code
        || receipt.summary != stored.summary
        || receipt.evidence_refs != evidence_refs
        || receipt.evidence_refs_digest != stored.evidence_refs_digest
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.opened_by_user_id != stored.opened_by_user_id
        || receipt.opened_at != stored.opened_at
        || receipt.replayed
    {
        bail!("算力结算挑战数据库列、请求或回执 JSON 不一致");
    }
    if settlement_challenge_event_digest(&receipt)? != stored.event_digest {
        bail!("算力结算挑战事件摘要审计失败");
    }

    let settlement = compute_attempt_settlement_on(conn, &stored.lease_id)?;
    if settlement.settlement.settlement_receipt_id != stored.settlement_receipt_id
        || settlement.event_digest != stored.settlement_event_digest
        || settlement.posting_id != stored.posting_id
        || settlement.posting_digest != stored.posting_digest
        || settlement.settlement.consumer_account_id != stored.consumer_account_id
        || settlement.settlement.provider_account_id != stored.provider_account_id
        || settlement.settlement.balance_state != "pending"
        || settlement.settlement.consumer_account_id != stored.opened_by_user_id
    {
        bail!("算力结算挑战与 v195 Settlement Receipt 不一致");
    }
    let settled_at = parse_time("Settlement 结算时间", &settlement.settled_at)?;
    let expected_deadline = settled_at
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("结算挑战截止时间超出范围")?;
    let opened_at = parse_time("挑战发起时间", &stored.opened_at)?;
    let deadline = parse_time("挑战截止时间", &stored.challenge_deadline)?;
    if stored.policy_id != COMPUTE_SETTLEMENT_CHALLENGE_POLICY_ID
        || stored.policy_version != COMPUTE_SETTLEMENT_CHALLENGE_POLICY_VERSION
        || deadline != expected_deadline
        || opened_at < settled_at
        || opened_at > deadline
        || receipt.status != "open"
        || receipt.balance_effect != "provider_and_platform_pending_unchanged"
        || receipt.settlement_release_effect != "blocked_by_open_challenge"
    {
        bail!("算力结算挑战政策、时间或效果字段审计失败");
    }
    receipt.replayed = replayed;
    Ok(receipt)
}
