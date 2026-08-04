use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{
        compute_attempt_settlement_challenges::{
            settlement_challenge_gate_on, ComputeSettlementChallengeGate,
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        },
        compute_attempt_settlements::compute_attempt_settlement_on,
        new_id,
    },
    ComputeSettlementReleaseReceipt, ReleaseComputeAttemptSettlementRequest,
    COMPUTE_SETTLEMENT_RELEASE_POLICY_ID, COMPUTE_SETTLEMENT_RELEASE_POLICY_VERSION,
    COMPUTE_SETTLEMENT_RELEASE_SCHEMA,
};

mod audit;
mod money;

#[derive(Debug, Clone)]
pub(super) struct StoredRelease {
    pub release_id: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub source_posting_id: String,
    pub source_posting_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub provider_released_micros: i64,
    pub platform_released_micros: i64,
    pub challenge_deadline: String,
    pub challenge_gate_json: String,
    pub challenge_gate_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub release_posting_id: String,
    pub release_posting_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub released_by_user_id: String,
    pub released_at: String,
}

impl StoredRelease {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementReleaseReceipt> {
        audit::audited_release_on(conn, self, replayed)
    }
}

pub(super) fn normalize_release_request(
    input: &ReleaseComputeAttemptSettlementRequest,
) -> Result<ReleaseComputeAttemptSettlementRequest> {
    let mut normalized = input.clone();
    for (label, value) in [
        ("Attempt Lease ID", &mut normalized.lease_id),
        (
            "Settlement Receipt ID",
            &mut normalized.expected_settlement_receipt_id,
        ),
        ("Settlement Posting ID", &mut normalized.expected_posting_id),
        ("幂等键", &mut normalized.idempotency_key),
        ("释放操作用户 ID", &mut normalized.released_by_user_id),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, 240)?;
    }
    for (label, value) in [
        (
            "Settlement 事件摘要",
            &mut normalized.expected_settlement_event_digest,
        ),
        (
            "Settlement Posting 摘要",
            &mut normalized.expected_posting_digest,
        ),
    ] {
        *value = value.trim().to_ascii_lowercase();
        validate_digest(label, value)?;
    }
    Ok(normalized)
}

pub(super) fn release_settlement_on(
    tx: &Transaction<'_>,
    request: &ReleaseComputeAttemptSettlementRequest,
    request_digest: &str,
) -> Result<ComputeSettlementReleaseReceipt> {
    let settlement = compute_attempt_settlement_on(tx, &request.lease_id)?;
    if settlement.settlement.settlement_receipt_id != request.expected_settlement_receipt_id
        || settlement.event_digest != request.expected_settlement_event_digest
        || settlement.posting_id != request.expected_posting_id
        || settlement.posting_digest != request.expected_posting_digest
        || settlement.settlement.balance_state != "pending"
    {
        bail!("待结算释放引用的 Settlement Receipt 或 Posting 不匹配");
    }
    let settled_at = parse_time("Settlement 结算时间", &settlement.settled_at)?;
    let challenge_deadline = settled_at
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("待结算释放截止时间超出范围")?;
    let released_at = Utc::now();
    if released_at < challenge_deadline {
        bail!("Settlement Receipt 尚未度过 72 小时消费者挑战窗口");
    }
    let challenge_gate =
        settlement_challenge_gate_on(tx, &settlement.settlement.settlement_receipt_id)?;
    if challenge_gate.blocked {
        bail!(
            "Settlement Receipt 被 {} 挑战状态阻止释放",
            challenge_gate.status
        );
    }

    let release_id = new_id("compute_settlement_release");
    let money = money::post_release_money_on(
        tx,
        money::PostReleaseMoneyInput {
            release_id: &release_id,
            settlement_receipt_id: &settlement.settlement.settlement_receipt_id,
            provider_account_id: &settlement.settlement.provider_account_id,
            provider_released_micros: settlement.settlement.amounts.provider_payable_micros,
            platform_released_micros: settlement.settlement.amounts.platform_margin_micros,
            released_at: &released_at.to_rfc3339(),
        },
    )?;
    let challenge_gate_digest = challenge_gate_digest(&challenge_gate)?;
    let mut receipt = ComputeSettlementReleaseReceipt {
        schema: COMPUTE_SETTLEMENT_RELEASE_SCHEMA.to_string(),
        release_id,
        settlement_receipt_id: settlement.settlement.settlement_receipt_id,
        settlement_event_digest: settlement.event_digest,
        source_posting_id: settlement.posting_id,
        source_posting_digest: settlement.posting_digest,
        lease_id: request.lease_id.clone(),
        consumer_account_id: settlement.settlement.consumer_account_id,
        provider_account_id: settlement.settlement.provider_account_id,
        platform_account_id: money::PLATFORM_ACCOUNT_ID.to_string(),
        currency: "CNY".to_string(),
        provider_released_micros: settlement.settlement.amounts.provider_payable_micros,
        platform_released_micros: settlement.settlement.amounts.platform_margin_micros,
        provider_pending_balance_after_micros: money.provider.pending_after_micros,
        provider_available_balance_after_micros: money.provider.available_after_micros,
        provider_account_revision_after: money.provider.revision_after,
        platform_pending_balance_after_micros: money.platform.pending_after_micros,
        platform_available_balance_after_micros: money.platform.available_after_micros,
        platform_account_revision_after: money.platform.revision_after,
        challenge_deadline: challenge_deadline.to_rfc3339(),
        challenge_gate,
        challenge_gate_digest,
        policy_id: COMPUTE_SETTLEMENT_RELEASE_POLICY_ID.to_string(),
        policy_version: COMPUTE_SETTLEMENT_RELEASE_POLICY_VERSION,
        release_posting_id: money.posting_id,
        release_posting_digest: money.posting_digest,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        released_by_user_id: request.released_by_user_id.clone(),
        released_at: released_at.to_rfc3339(),
        balance_effect: "provider_and_platform_pending_moved_to_available".to_string(),
        withdrawal_effect: "no_external_transfer_or_withdrawal".to_string(),
        replayed: false,
    };
    receipt.event_digest = release_event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn release_request_digest(
    input: &ReleaseComputeAttemptSettlementRequest,
) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn challenge_gate_digest(gate: &ComputeSettlementChallengeGate) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(gate)?)))
}

pub(super) fn release_event_digest(receipt: &ComputeSettlementReleaseReceipt) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_release_on(
    conn: &Connection,
    request: &ReleaseComputeAttemptSettlementRequest,
    receipt: &ComputeSettlementReleaseReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_releases (
           release_id, settlement_receipt_id, settlement_event_digest,
           source_posting_id, source_posting_digest, lease_id,
           consumer_account_id, provider_account_id,
           provider_released_micros, platform_released_micros,
           challenge_deadline, challenge_gate_json, challenge_gate_digest,
           policy_id, policy_version, release_posting_id, release_posting_digest,
           request_json, request_digest, receipt_json, event_digest,
           idempotency_scope, idempotency_key, released_by_user_id,
           released_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,
                   ?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?25)",
        params![
            receipt.release_id,
            receipt.settlement_receipt_id,
            receipt.settlement_event_digest,
            receipt.source_posting_id,
            receipt.source_posting_digest,
            receipt.lease_id,
            receipt.consumer_account_id,
            receipt.provider_account_id,
            receipt.provider_released_micros,
            receipt.platform_released_micros,
            receipt.challenge_deadline,
            serde_json::to_string(&receipt.challenge_gate)?,
            receipt.challenge_gate_digest,
            receipt.policy_id,
            receipt.policy_version,
            receipt.release_posting_id,
            receipt.release_posting_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.released_by_user_id,
            receipt.released_at,
        ],
    )?;
    Ok(())
}

pub(super) fn release_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRelease>> {
    release_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn release_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredRelease>> {
    release_query(conn, "WHERE lease_id=?1", params![lease_id])
}

fn release_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredRelease>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!(
            "SELECT release_id, settlement_receipt_id, settlement_event_digest,
                    source_posting_id, source_posting_digest, lease_id,
                    consumer_account_id, provider_account_id,
                    provider_released_micros, platform_released_micros,
                    challenge_deadline, challenge_gate_json, challenge_gate_digest,
                    policy_id, policy_version, release_posting_id,
                    release_posting_digest, request_json, request_digest,
                    receipt_json, event_digest, idempotency_scope,
                    idempotency_key, released_by_user_id, released_at
               FROM compute_settlement_releases {where_clause}"
        ),
        values,
        stored_release_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_release_from_row(row: &Row<'_>) -> rusqlite::Result<StoredRelease> {
    Ok(StoredRelease {
        release_id: row.get(0)?,
        settlement_receipt_id: row.get(1)?,
        settlement_event_digest: row.get(2)?,
        source_posting_id: row.get(3)?,
        source_posting_digest: row.get(4)?,
        lease_id: row.get(5)?,
        consumer_account_id: row.get(6)?,
        provider_account_id: row.get(7)?,
        provider_released_micros: row.get(8)?,
        platform_released_micros: row.get(9)?,
        challenge_deadline: row.get(10)?,
        challenge_gate_json: row.get(11)?,
        challenge_gate_digest: row.get(12)?,
        policy_id: row.get(13)?,
        policy_version: row.get(14)?,
        release_posting_id: row.get(15)?,
        release_posting_digest: row.get(16)?,
        request_json: row.get(17)?,
        request_digest: row.get(18)?,
        receipt_json: row.get(19)?,
        event_digest: row.get(20)?,
        idempotency_scope: row.get(21)?,
        idempotency_key: row.get(22)?,
        released_by_user_id: row.get(23)?,
        released_at: row.get(24)?,
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max_len
        || value != value.trim()
        || value.chars().any(char::is_control)
    {
        bail!("{label}无效");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    validate_exact(label, value, 64)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}

pub(super) fn parse_time(label: &str, value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label}不是 RFC3339 时间"))
        .map(|value| value.with_timezone(&Utc))
}
