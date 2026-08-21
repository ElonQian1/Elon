use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::super::super::compute_attempt_settlement_challenges::{
    compute_attempt_historical_settlement_challenge_by_lease_on, compute_settlement_challenge_on,
};
use super::super::{
    ComputeSettlementChallengeResolutionReceipt, COMPUTE_SETTLEMENT_CHALLENGE_RESOLUTION_SCHEMA,
};
use super::{
    normalize_resolution_request, resolution_event_digest, resolution_request_digest,
    StoredResolution,
};

pub(super) fn audited_resolution_on(
    conn: &Connection,
    stored: &StoredResolution,
    replayed: bool,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    audited_resolution_with_head_policy_on(conn, stored, replayed, true)
}

pub(super) fn audited_historical_resolution_on(
    conn: &Connection,
    stored: &StoredResolution,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    audited_resolution_with_head_policy_on(conn, stored, false, false)
}

fn audited_resolution_with_head_policy_on(
    conn: &Connection,
    stored: &StoredResolution,
    replayed: bool,
    require_current_heads: bool,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    let request = normalize_resolution_request(&serde_json::from_str(&stored.request_json)?)?;
    let mut receipt: ComputeSettlementChallengeResolutionReceipt =
        serde_json::from_str(&stored.receipt_json)?;
    if stored.request_json != serde_json::to_string(&request)?
        || stored.receipt_json != serde_json::to_string(&receipt)?
        || request.lease_id != stored.lease_id
        || request.expected_challenge_id != stored.challenge_id
        || request.expected_challenge_event_digest != stored.challenge_event_digest
        || request.action != stored.action
        || request.statement != stored.statement
        || request.actor_user_id != stored.actor_user_id
        || request.actor_role != stored.actor_role
        || request.idempotency_key != stored.idempotency_key
        || resolution_request_digest(&request)? != stored.request_digest
        || stored.idempotency_scope
            != format!(
                "compute_settlement_challenge_resolution:{}:{}",
                stored.actor_role, stored.actor_user_id
            )
        || receipt.schema != COMPUTE_SETTLEMENT_CHALLENGE_RESOLUTION_SCHEMA
        || receipt.resolution_id != stored.resolution_id
        || receipt.challenge_id != stored.challenge_id
        || receipt.challenge_event_digest != stored.challenge_event_digest
        || receipt.settlement_receipt_id != stored.settlement_receipt_id
        || receipt.settlement_event_digest != stored.settlement_event_digest
        || receipt.lease_id != stored.lease_id
        || receipt.consumer_account_id != stored.consumer_account_id
        || receipt.provider_account_id != stored.provider_account_id
        || receipt.action != stored.action
        || receipt.statement != stored.statement
        || receipt.actor_user_id != stored.actor_user_id
        || receipt.actor_role != stored.actor_role
        || receipt.request_digest != stored.request_digest
        || receipt.event_digest != stored.event_digest
        || receipt.resolved_at != stored.resolved_at
        || receipt.replayed
    {
        bail!("算力结算挑战决议数据库列、请求或回执 JSON 不一致");
    }
    if resolution_event_digest(&receipt)? != stored.event_digest {
        bail!("算力结算挑战决议事件摘要审计失败");
    }

    let challenge = if require_current_heads {
        compute_settlement_challenge_on(conn, &stored.lease_id)?
    } else {
        compute_attempt_historical_settlement_challenge_by_lease_on(conn, &stored.lease_id)?
            .ok_or_else(|| anyhow::anyhow!("算力结算挑战决议引用的 v196 历史回执不存在"))?
    };
    if challenge.challenge_id != stored.challenge_id
        || challenge.event_digest != stored.challenge_event_digest
        || challenge.settlement_receipt_id != stored.settlement_receipt_id
        || challenge.settlement_event_digest != stored.settlement_event_digest
        || challenge.consumer_account_id != stored.consumer_account_id
        || challenge.provider_account_id != stored.provider_account_id
    {
        bail!("算力结算挑战决议与 v196 挑战不一致");
    }
    if stored.actor_role == "consumer" && stored.actor_user_id != stored.consumer_account_id {
        bail!("结算挑战撤回操作人不是原 Job 消费者");
    }
    let expected_correction_required = stored.action == "accepted";
    let expected_release_effect = if expected_correction_required {
        "blocked_by_accepted_challenge"
    } else {
        "challenge_no_longer_blocks_release"
    };
    let resolved_at = DateTime::parse_from_rfc3339(&stored.resolved_at)?.with_timezone(&Utc);
    let opened_at = DateTime::parse_from_rfc3339(&challenge.opened_at)?.with_timezone(&Utc);
    if resolved_at < opened_at
        || receipt.challenge_status != stored.action
        || receipt.correction_required != expected_correction_required
        || receipt.balance_effect != "consumer_provider_and_platform_balances_unchanged"
        || receipt.settlement_release_effect != expected_release_effect
    {
        bail!("算力结算挑战决议时间、状态或效果字段审计失败");
    }
    receipt.replayed = replayed;
    Ok(receipt)
}
