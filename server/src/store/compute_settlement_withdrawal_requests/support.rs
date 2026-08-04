use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{common::new_id, compute_provider_registry::current_registered_provider_on},
    ComputeSettlementWithdrawalRequestReceipt, CreateComputeSettlementWithdrawalRequest,
    COMPUTE_SETTLEMENT_WITHDRAWAL_REQUEST_SCHEMA,
};

mod audit;
mod money;

#[derive(Debug, Clone)]
pub(super) struct StoredWithdrawalRequest {
    pub withdrawal_id: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub amount_micros: i64,
    pub destination_kind: String,
    pub destination_ref: String,
    pub request_posting_id: String,
    pub request_posting_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub requested_at: String,
}

impl StoredWithdrawalRequest {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
        audit::audited_request_on(conn, self, replayed)
    }
}

pub(super) fn normalize_request(
    input: &CreateComputeSettlementWithdrawalRequest,
) -> Result<CreateComputeSettlementWithdrawalRequest> {
    let mut normalized = input.clone();
    for (label, value, max_len) in [
        ("Provider ID", &mut normalized.provider_id, 160),
        (
            "Provider Settlement Account ID",
            &mut normalized.provider_account_id,
            240,
        ),
        ("Provider 所有者", &mut normalized.owner_user_id, 240),
        ("提现目标类型", &mut normalized.destination_kind, 80),
        ("提现目标引用", &mut normalized.destination_ref, 512),
        ("幂等键", &mut normalized.idempotency_key, 240),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, max_len)?;
    }
    normalized.expected_provider_digest = normalized
        .expected_provider_digest
        .trim()
        .to_ascii_lowercase();
    validate_digest("Provider 摘要", &normalized.expected_provider_digest)?;
    if normalized.expected_provider_policy_revision <= 0 {
        bail!("Provider 策略版本必须大于 0");
    }
    if normalized.amount_micros <= 0 {
        bail!("提现申请金额必须大于 0");
    }
    if !matches!(
        normalized.destination_kind.as_str(),
        "bank_account_vault_ref"
            | "digital_wallet_vault_ref"
            | "sui_address_ref"
            | "other_vault_ref"
    ) {
        bail!("提现目标类型不受支持");
    }
    Ok(normalized)
}

pub(super) fn request_withdrawal_on(
    tx: &Transaction<'_>,
    request: &CreateComputeSettlementWithdrawalRequest,
    request_digest: &str,
) -> Result<ComputeSettlementWithdrawalRequestReceipt> {
    let provider = current_registered_provider_on(tx, &request.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("算力 Provider 不存在"))?;
    let account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str());
    if provider.provider.owner_account_id != request.owner_user_id
        || account_id != request.provider_account_id
        || provider.provider.policy_revision != request.expected_provider_policy_revision
        || provider.provider_digest != request.expected_provider_digest
    {
        bail!("提现申请与当前 Provider 所有权、结算账户或策略版本不一致");
    }

    let requested_at = Utc::now().to_rfc3339();
    let withdrawal_id = new_id("compute_settlement_withdrawal");
    let money = money::reserve_withdrawal_on(
        tx,
        &withdrawal_id,
        &request.provider_account_id,
        request.amount_micros,
        &requested_at,
    )?;
    let mut receipt = ComputeSettlementWithdrawalRequestReceipt {
        schema: COMPUTE_SETTLEMENT_WITHDRAWAL_REQUEST_SCHEMA.to_string(),
        withdrawal_id,
        provider_id: request.provider_id.clone(),
        provider_policy_revision: request.expected_provider_policy_revision,
        provider_digest: request.expected_provider_digest.clone(),
        provider_account_id: request.provider_account_id.clone(),
        owner_user_id: request.owner_user_id.clone(),
        currency: "CNY".to_string(),
        amount_micros: request.amount_micros,
        destination_kind: request.destination_kind.clone(),
        destination_ref: request.destination_ref.clone(),
        available_balance_after_micros: money.available_after_micros,
        withdrawn_balance_after_micros: money.withdrawn_after_micros,
        account_revision_after: money.revision_after,
        request_posting_id: money.posting_id,
        request_posting_digest: money.posting_digest,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        requested_at,
        fund_effect: "provider_available_moved_to_withdrawn_reserve".to_string(),
        external_transfer_effect: "not_executed".to_string(),
        replayed: false,
    };
    receipt.event_digest = event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn persist_request_on(
    conn: &Connection,
    request: &CreateComputeSettlementWithdrawalRequest,
    receipt: &ComputeSettlementWithdrawalRequestReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_withdrawal_requests (
           withdrawal_id, provider_id, provider_policy_revision, provider_digest,
           provider_account_id, owner_user_id, currency, amount_micros,
           destination_kind, destination_ref, request_posting_id,
           request_posting_digest, request_json, request_digest, receipt_json,
           event_digest, idempotency_scope, idempotency_key, requested_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,'CNY',?7,?8,?9,?10,?11,?12,?13,?14,
                   ?15,?16,?17,?18,?18)",
        params![
            receipt.withdrawal_id,
            receipt.provider_id,
            receipt.provider_policy_revision,
            receipt.provider_digest,
            receipt.provider_account_id,
            receipt.owner_user_id,
            receipt.amount_micros,
            receipt.destination_kind,
            receipt.destination_ref,
            receipt.request_posting_id,
            receipt.request_posting_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.requested_at,
        ],
    )?;
    Ok(())
}

pub(super) fn request_digest(input: &CreateComputeSettlementWithdrawalRequest) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn event_digest(receipt: &ComputeSettlementWithdrawalRequestReceipt) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn request_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredWithdrawalRequest>> {
    request_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn request_by_id_on(
    conn: &Connection,
    withdrawal_id: &str,
) -> Result<Option<StoredWithdrawalRequest>> {
    request_query(conn, "WHERE withdrawal_id=?1", params![withdrawal_id])
}

pub(super) fn list_requests_on(
    conn: &Connection,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<StoredWithdrawalRequest>> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE provider_id=?1 ORDER BY requested_at DESC, withdrawal_id DESC LIMIT ?2",
        SELECT_REQUEST
    ))?;
    let rows = stmt.query_map(params![provider_id, limit as i64], stored_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

const SELECT_REQUEST: &str =
    "SELECT withdrawal_id, provider_id, provider_policy_revision, provider_digest,
            provider_account_id, owner_user_id, amount_micros, destination_kind,
            destination_ref, request_posting_id, request_posting_digest,
            request_json, request_digest, receipt_json, event_digest,
            idempotency_scope, idempotency_key, requested_at
       FROM compute_settlement_withdrawal_requests";

fn request_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredWithdrawalRequest>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!("{SELECT_REQUEST} {where_clause}"),
        values,
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredWithdrawalRequest> {
    Ok(StoredWithdrawalRequest {
        withdrawal_id: row.get(0)?,
        provider_id: row.get(1)?,
        provider_policy_revision: row.get(2)?,
        provider_digest: row.get(3)?,
        provider_account_id: row.get(4)?,
        owner_user_id: row.get(5)?,
        amount_micros: row.get(6)?,
        destination_kind: row.get(7)?,
        destination_ref: row.get(8)?,
        request_posting_id: row.get(9)?,
        request_posting_digest: row.get(10)?,
        request_json: row.get(11)?,
        request_digest: row.get(12)?,
        receipt_json: row.get(13)?,
        event_digest: row.get(14)?,
        idempotency_scope: row.get(15)?,
        idempotency_key: row.get(16)?,
        requested_at: row.get(17)?,
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("{label}不能为空或包含首尾空白");
    }
    if value.chars().count() > max_len {
        bail!("{label}长度不能超过 {max_len}");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label}必须是 64 位十六进制摘要");
    }
    Ok(())
}
