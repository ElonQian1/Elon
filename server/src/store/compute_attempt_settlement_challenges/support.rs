use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{compute_attempt_settlements::compute_attempt_settlement_on, new_id},
    ComputeSettlementChallengeReceipt, OpenComputeSettlementChallengeRequest,
    COMPUTE_SETTLEMENT_CHALLENGE_POLICY_ID, COMPUTE_SETTLEMENT_CHALLENGE_POLICY_VERSION,
    COMPUTE_SETTLEMENT_CHALLENGE_SCHEMA, COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
};

mod audit;

#[derive(Debug, Clone)]
pub(super) struct StoredChallenge {
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
    pub reason_code: String,
    pub summary: String,
    pub evidence_refs_json: String,
    pub evidence_refs_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub opened_by_user_id: String,
    pub opened_at: String,
}

impl StoredChallenge {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementChallengeReceipt> {
        audit::audited_challenge_on(conn, self, replayed)
    }
}

pub(super) fn normalize_challenge_request(
    input: &OpenComputeSettlementChallengeRequest,
) -> Result<OpenComputeSettlementChallengeRequest> {
    let mut normalized = input.clone();
    for (label, value) in [
        ("Attempt Lease ID", &mut normalized.lease_id),
        (
            "Settlement Receipt ID",
            &mut normalized.expected_settlement_receipt_id,
        ),
        ("Settlement Posting ID", &mut normalized.expected_posting_id),
        ("幂等键", &mut normalized.idempotency_key),
        ("挑战发起用户 ID", &mut normalized.opened_by_user_id),
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
    normalized.reason_code = normalize_reason_code(&normalized.reason_code)?.to_string();
    normalized.summary = normalized.summary.trim().to_string();
    let summary_len = normalized.summary.chars().count();
    if summary_len < 8 || summary_len > 1000 || normalized.summary.chars().any(char::is_control) {
        bail!("结算挑战摘要长度必须为 8 至 1000 个字符且不能包含控制字符");
    }
    if normalized.evidence_refs.len() > 16 {
        bail!("结算挑战证据引用不能超过 16 条");
    }
    let mut evidence_refs = Vec::with_capacity(normalized.evidence_refs.len());
    for value in &normalized.evidence_refs {
        let value = value.trim();
        validate_exact("结算挑战证据引用", value, 512)?;
        evidence_refs.push(value.to_string());
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    normalized.evidence_refs = evidence_refs;
    Ok(normalized)
}

pub(super) fn open_challenge_on(
    tx: &Transaction<'_>,
    request: &OpenComputeSettlementChallengeRequest,
    request_digest: &str,
) -> Result<ComputeSettlementChallengeReceipt> {
    let settlement = compute_attempt_settlement_on(tx, &request.lease_id)?;
    if settlement.settlement.settlement_receipt_id != request.expected_settlement_receipt_id
        || settlement.event_digest != request.expected_settlement_event_digest
        || settlement.posting_id != request.expected_posting_id
        || settlement.posting_digest != request.expected_posting_digest
        || settlement.settlement.balance_state != "pending"
        || settlement.settlement.consumer_account_id != request.opened_by_user_id
    {
        bail!("结算挑战引用的 Settlement Receipt、Posting 或消费者身份不匹配");
    }
    let already_released = tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM compute_settlement_releases
            WHERE settlement_receipt_id=?1
         )",
        params![settlement.settlement.settlement_receipt_id],
        |row| row.get::<_, bool>(0),
    )?;
    if already_released {
        bail!("Settlement Receipt 已经释放到 available，不能再创建消费者挑战");
    }
    let settled_at = parse_time("Settlement 结算时间", &settlement.settled_at)?;
    let challenge_deadline = settled_at
        .checked_add_signed(Duration::seconds(
            COMPUTE_SETTLEMENT_CHALLENGE_WINDOW_SECONDS,
        ))
        .context("结算挑战截止时间超出范围")?;
    let opened_at = std::cmp::max(Utc::now(), settled_at);
    if opened_at > challenge_deadline {
        bail!("Settlement Receipt 的消费者挑战窗口已经结束");
    }
    let evidence_refs_digest = evidence_refs_digest(&request.evidence_refs)?;
    let mut receipt = ComputeSettlementChallengeReceipt {
        schema: COMPUTE_SETTLEMENT_CHALLENGE_SCHEMA.to_string(),
        challenge_id: new_id("compute_settlement_challenge"),
        settlement_receipt_id: settlement.settlement.settlement_receipt_id,
        settlement_event_digest: settlement.event_digest,
        lease_id: request.lease_id.clone(),
        consumer_account_id: settlement.settlement.consumer_account_id,
        provider_account_id: settlement.settlement.provider_account_id,
        posting_id: settlement.posting_id,
        posting_digest: settlement.posting_digest,
        policy_id: COMPUTE_SETTLEMENT_CHALLENGE_POLICY_ID.to_string(),
        policy_version: COMPUTE_SETTLEMENT_CHALLENGE_POLICY_VERSION,
        challenge_deadline: challenge_deadline.to_rfc3339(),
        status: "open".to_string(),
        reason_code: request.reason_code.clone(),
        summary: request.summary.clone(),
        evidence_refs: request.evidence_refs.clone(),
        evidence_refs_digest,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        opened_by_user_id: request.opened_by_user_id.clone(),
        opened_at: opened_at.to_rfc3339(),
        balance_effect: "provider_and_platform_pending_unchanged".to_string(),
        settlement_release_effect: "blocked_by_open_challenge".to_string(),
        replayed: false,
    };
    receipt.event_digest = settlement_challenge_event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn settlement_challenge_request_digest(
    input: &OpenComputeSettlementChallengeRequest,
) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn evidence_refs_digest(values: &[String]) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(values)?)))
}

pub(super) fn settlement_challenge_event_digest(
    receipt: &ComputeSettlementChallengeReceipt,
) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_challenge_on(
    conn: &Connection,
    request: &OpenComputeSettlementChallengeRequest,
    receipt: &ComputeSettlementChallengeReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_challenges (
           challenge_id, settlement_receipt_id, settlement_event_digest,
           lease_id, consumer_account_id, provider_account_id,
           posting_id, posting_digest, policy_id, policy_version,
           challenge_deadline, reason_code, summary, evidence_refs_json,
           evidence_refs_digest, request_json, request_digest, receipt_json,
           event_digest, idempotency_scope, idempotency_key,
           opened_by_user_id, opened_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,
                   ?15,?16,?17,?18,?19,?20,?21,?22,?23,?23)",
        params![
            receipt.challenge_id,
            receipt.settlement_receipt_id,
            receipt.settlement_event_digest,
            receipt.lease_id,
            receipt.consumer_account_id,
            receipt.provider_account_id,
            receipt.posting_id,
            receipt.posting_digest,
            receipt.policy_id,
            receipt.policy_version,
            receipt.challenge_deadline,
            receipt.reason_code,
            receipt.summary,
            serde_json::to_string(&receipt.evidence_refs)?,
            receipt.evidence_refs_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.opened_by_user_id,
            receipt.opened_at,
        ],
    )?;
    Ok(())
}

pub(super) fn challenge_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredChallenge>> {
    challenge_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn challenge_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredChallenge>> {
    challenge_query(conn, "WHERE lease_id=?1", params![lease_id])
}

pub(super) fn challenge_by_settlement_on(
    conn: &Connection,
    settlement_receipt_id: &str,
) -> Result<Option<StoredChallenge>> {
    challenge_query(
        conn,
        "WHERE settlement_receipt_id=?1",
        params![settlement_receipt_id],
    )
}

fn challenge_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredChallenge>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!(
            "SELECT challenge_id, settlement_receipt_id, settlement_event_digest,
                    lease_id, consumer_account_id, provider_account_id,
                    posting_id, posting_digest, policy_id, policy_version,
                    challenge_deadline, reason_code, summary, evidence_refs_json,
                    evidence_refs_digest, request_json, request_digest,
                    receipt_json, event_digest, idempotency_scope,
                    idempotency_key, opened_by_user_id, opened_at
               FROM compute_settlement_challenges {where_clause}"
        ),
        values,
        stored_challenge_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_challenge_from_row(row: &Row<'_>) -> rusqlite::Result<StoredChallenge> {
    Ok(StoredChallenge {
        challenge_id: row.get(0)?,
        settlement_receipt_id: row.get(1)?,
        settlement_event_digest: row.get(2)?,
        lease_id: row.get(3)?,
        consumer_account_id: row.get(4)?,
        provider_account_id: row.get(5)?,
        posting_id: row.get(6)?,
        posting_digest: row.get(7)?,
        policy_id: row.get(8)?,
        policy_version: row.get(9)?,
        challenge_deadline: row.get(10)?,
        reason_code: row.get(11)?,
        summary: row.get(12)?,
        evidence_refs_json: row.get(13)?,
        evidence_refs_digest: row.get(14)?,
        request_json: row.get(15)?,
        request_digest: row.get(16)?,
        receipt_json: row.get(17)?,
        event_digest: row.get(18)?,
        idempotency_scope: row.get(19)?,
        idempotency_key: row.get(20)?,
        opened_by_user_id: row.get(21)?,
        opened_at: row.get(22)?,
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

pub(super) fn parse_time(label: &str, value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label}不是 RFC3339"))
        .map(|value| value.with_timezone(&Utc))
}

fn normalize_reason_code(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "amount" => Ok("amount"),
        "metering" => Ok("metering"),
        "price_snapshot" => Ok("price_snapshot"),
        "execution_evidence" => Ok("execution_evidence"),
        "provider_identity" => Ok("provider_identity"),
        "other" => Ok("other"),
        _ => bail!("结算挑战原因代码不受支持"),
    }
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    validate_exact(label, value, 64)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}
