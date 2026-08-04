use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{compute_attempt_settlement_challenges::compute_settlement_challenge_on, new_id},
    ComputeSettlementChallengeResolutionReceipt, ResolveComputeSettlementChallengeRequest,
    COMPUTE_SETTLEMENT_CHALLENGE_RESOLUTION_SCHEMA,
};

mod audit;

#[derive(Debug, Clone)]
pub(super) struct StoredResolution {
    pub resolution_id: String,
    pub challenge_id: String,
    pub challenge_event_digest: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub action: String,
    pub statement: String,
    pub actor_user_id: String,
    pub actor_role: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub resolved_at: String,
}

impl StoredResolution {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementChallengeResolutionReceipt> {
        audit::audited_resolution_on(conn, self, replayed)
    }
}

pub(super) fn normalize_resolution_request(
    input: &ResolveComputeSettlementChallengeRequest,
) -> Result<ResolveComputeSettlementChallengeRequest> {
    let mut normalized = input.clone();
    for (label, value) in [
        ("Attempt Lease ID", &mut normalized.lease_id),
        ("结算挑战 ID", &mut normalized.expected_challenge_id),
        ("决议操作用户 ID", &mut normalized.actor_user_id),
        ("幂等键", &mut normalized.idempotency_key),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, 240)?;
    }
    normalized.expected_challenge_event_digest = normalized
        .expected_challenge_event_digest
        .trim()
        .to_ascii_lowercase();
    validate_digest(
        "结算挑战事件摘要",
        &normalized.expected_challenge_event_digest,
    )?;
    normalized.action = normalize_action(&normalized.action)?.to_string();
    normalized.actor_role = normalize_actor_role(&normalized.actor_role)?.to_string();
    normalized.statement = normalized.statement.trim().to_string();
    let statement_len = normalized.statement.chars().count();
    if statement_len < 8
        || statement_len > 1000
        || normalized.statement.chars().any(char::is_control)
    {
        bail!("结算挑战决议说明长度必须为 8 至 1000 个字符且不能包含控制字符");
    }
    match (normalized.actor_role.as_str(), normalized.action.as_str()) {
        ("consumer", "withdrawn") => {}
        ("platform_admin", "accepted" | "rejected") => {}
        _ => bail!("结算挑战决议操作与操作人角色不匹配"),
    }
    Ok(normalized)
}

pub(super) fn resolve_challenge_on(
    tx: &Transaction<'_>,
    request: &ResolveComputeSettlementChallengeRequest,
    request_digest: &str,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    let challenge = compute_settlement_challenge_on(tx, &request.lease_id)?;
    if challenge.challenge_id != request.expected_challenge_id
        || challenge.event_digest != request.expected_challenge_event_digest
    {
        bail!("结算挑战决议引用的挑战 ID 或事件摘要不匹配");
    }
    if request.actor_role == "consumer" && request.actor_user_id != challenge.consumer_account_id {
        bail!("只有原 Job 消费者可以撤回结算挑战");
    }
    let correction_required = request.action == "accepted";
    let settlement_release_effect = if correction_required {
        "blocked_by_accepted_challenge"
    } else {
        "challenge_no_longer_blocks_release"
    };
    let mut receipt = ComputeSettlementChallengeResolutionReceipt {
        schema: COMPUTE_SETTLEMENT_CHALLENGE_RESOLUTION_SCHEMA.to_string(),
        resolution_id: new_id("compute_settlement_challenge_resolution"),
        challenge_id: challenge.challenge_id,
        challenge_event_digest: challenge.event_digest,
        settlement_receipt_id: challenge.settlement_receipt_id,
        settlement_event_digest: challenge.settlement_event_digest,
        lease_id: request.lease_id.clone(),
        consumer_account_id: challenge.consumer_account_id,
        provider_account_id: challenge.provider_account_id,
        action: request.action.clone(),
        statement: request.statement.clone(),
        actor_user_id: request.actor_user_id.clone(),
        actor_role: request.actor_role.clone(),
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        resolved_at: Utc::now().to_rfc3339(),
        challenge_status: request.action.clone(),
        correction_required,
        balance_effect: "consumer_provider_and_platform_balances_unchanged".to_string(),
        settlement_release_effect: settlement_release_effect.to_string(),
        replayed: false,
    };
    receipt.event_digest = resolution_event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn resolution_request_digest(
    input: &ResolveComputeSettlementChallengeRequest,
) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn resolution_event_digest(
    receipt: &ComputeSettlementChallengeResolutionReceipt,
) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_resolution_on(
    conn: &Connection,
    request: &ResolveComputeSettlementChallengeRequest,
    receipt: &ComputeSettlementChallengeResolutionReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_challenge_resolutions (
           resolution_id, challenge_id, challenge_event_digest,
           settlement_receipt_id, settlement_event_digest, lease_id,
           consumer_account_id, provider_account_id, action, statement,
           actor_user_id, actor_role, request_json, request_digest,
           receipt_json, event_digest, idempotency_scope, idempotency_key,
           resolved_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,
                   ?15,?16,?17,?18,?19,?19)",
        params![
            receipt.resolution_id,
            receipt.challenge_id,
            receipt.challenge_event_digest,
            receipt.settlement_receipt_id,
            receipt.settlement_event_digest,
            receipt.lease_id,
            receipt.consumer_account_id,
            receipt.provider_account_id,
            receipt.action,
            receipt.statement,
            receipt.actor_user_id,
            receipt.actor_role,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.resolved_at,
        ],
    )?;
    Ok(())
}

pub(super) fn resolution_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredResolution>> {
    resolution_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn resolution_by_challenge_on(
    conn: &Connection,
    challenge_id: &str,
) -> Result<Option<StoredResolution>> {
    resolution_query(conn, "WHERE challenge_id=?1", params![challenge_id])
}

pub(super) fn resolution_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredResolution>> {
    resolution_query(conn, "WHERE lease_id=?1", params![lease_id])
}

fn resolution_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredResolution>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!(
            "SELECT resolution_id, challenge_id, challenge_event_digest,
                    settlement_receipt_id, settlement_event_digest, lease_id,
                    consumer_account_id, provider_account_id, action, statement,
                    actor_user_id, actor_role, request_json, request_digest,
                    receipt_json, event_digest, idempotency_scope,
                    idempotency_key, resolved_at
               FROM compute_settlement_challenge_resolutions {where_clause}"
        ),
        values,
        stored_resolution_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_resolution_from_row(row: &Row<'_>) -> rusqlite::Result<StoredResolution> {
    Ok(StoredResolution {
        resolution_id: row.get(0)?,
        challenge_id: row.get(1)?,
        challenge_event_digest: row.get(2)?,
        settlement_receipt_id: row.get(3)?,
        settlement_event_digest: row.get(4)?,
        lease_id: row.get(5)?,
        consumer_account_id: row.get(6)?,
        provider_account_id: row.get(7)?,
        action: row.get(8)?,
        statement: row.get(9)?,
        actor_user_id: row.get(10)?,
        actor_role: row.get(11)?,
        request_json: row.get(12)?,
        request_digest: row.get(13)?,
        receipt_json: row.get(14)?,
        event_digest: row.get(15)?,
        idempotency_scope: row.get(16)?,
        idempotency_key: row.get(17)?,
        resolved_at: row.get(18)?,
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

fn normalize_action(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "withdrawn" => Ok("withdrawn"),
        "accepted" => Ok("accepted"),
        "rejected" => Ok("rejected"),
        _ => bail!("结算挑战决议操作不受支持"),
    }
}

fn normalize_actor_role(value: &str) -> Result<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "consumer" => Ok("consumer"),
        "platform_admin" => Ok("platform_admin"),
        _ => bail!("结算挑战决议操作人角色不受支持"),
    }
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    validate_exact(label, value, 64)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}
