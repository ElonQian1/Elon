use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use super::{
    super::{
        compute_attempt_settlement_challenge_resolutions::compute_settlement_challenge_resolution_on,
        compute_attempt_settlements::{
            calculation::MICROS_PER_CNY_FEN, compute_attempt_settlement_on,
        },
        new_id,
    },
    ComputeSettlementCorrectionReceipt, CorrectComputeAttemptSettlementRequest,
    COMPUTE_SETTLEMENT_CORRECTION_POLICY_ID, COMPUTE_SETTLEMENT_CORRECTION_POLICY_VERSION,
    COMPUTE_SETTLEMENT_CORRECTION_SCHEMA,
};

mod audit;
mod money;

#[derive(Debug, Clone)]
pub(super) struct StoredCorrection {
    pub correction_id: String,
    pub challenge_id: String,
    pub challenge_event_digest: String,
    pub resolution_id: String,
    pub resolution_event_digest: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub original_consumer_charge_micros: i64,
    pub corrected_consumer_charge_micros: i64,
    pub consumer_refund_micros: i64,
    pub original_provider_payable_micros: i64,
    pub corrected_provider_payable_micros: i64,
    pub provider_reversal_micros: i64,
    pub original_platform_margin_micros: i64,
    pub corrected_platform_margin_micros: i64,
    pub platform_reversal_micros: i64,
    pub statement: String,
    pub evidence_refs_json: String,
    pub evidence_refs_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub posting_id: String,
    pub posting_digest: String,
    pub request_json: String,
    pub request_digest: String,
    pub receipt_json: String,
    pub event_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub corrected_by_user_id: String,
    pub corrected_at: String,
}

impl StoredCorrection {
    pub(super) fn into_receipt(
        &self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeSettlementCorrectionReceipt> {
        audit::audited_correction_on(conn, self, replayed)
    }
}

pub(super) fn normalize_correction_request(
    input: &CorrectComputeAttemptSettlementRequest,
) -> Result<CorrectComputeAttemptSettlementRequest> {
    let mut normalized = input.clone();
    for (label, value) in [
        ("Attempt Lease ID", &mut normalized.lease_id),
        ("结算挑战 ID", &mut normalized.expected_challenge_id),
        ("挑战决议 ID", &mut normalized.expected_resolution_id),
        (
            "Settlement Receipt ID",
            &mut normalized.expected_settlement_receipt_id,
        ),
        ("幂等键", &mut normalized.idempotency_key),
        ("纠正操作用户 ID", &mut normalized.corrected_by_user_id),
    ] {
        *value = value.trim().to_string();
        validate_exact(label, value, 240)?;
    }
    for (label, value) in [
        (
            "结算挑战事件摘要",
            &mut normalized.expected_challenge_event_digest,
        ),
        (
            "挑战决议事件摘要",
            &mut normalized.expected_resolution_event_digest,
        ),
        (
            "Settlement 事件摘要",
            &mut normalized.expected_settlement_event_digest,
        ),
    ] {
        *value = value.trim().to_ascii_lowercase();
        validate_digest(label, value)?;
    }
    if normalized.corrected_consumer_charge_fen < 0
        || normalized.corrected_provider_payable_micros < 0
        || normalized.corrected_platform_margin_micros < 0
    {
        bail!("结算纠正后的金额不能为负数");
    }
    normalized.statement = normalized.statement.trim().to_string();
    let statement_len = normalized.statement.chars().count();
    if statement_len < 8
        || statement_len > 1000
        || normalized.statement.chars().any(char::is_control)
    {
        bail!("结算纠正说明长度必须为 8 至 1000 个字符且不能包含控制字符");
    }
    if normalized.evidence_refs.len() > 16 {
        bail!("结算纠正证据引用不能超过 16 条");
    }
    let mut evidence_refs = Vec::with_capacity(normalized.evidence_refs.len());
    for value in &normalized.evidence_refs {
        let value = value.trim();
        validate_exact("结算纠正证据引用", value, 512)?;
        evidence_refs.push(value.to_string());
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    normalized.evidence_refs = evidence_refs;
    Ok(normalized)
}

pub(super) fn correct_settlement_on(
    tx: &Transaction<'_>,
    request: &CorrectComputeAttemptSettlementRequest,
    request_digest: &str,
) -> Result<ComputeSettlementCorrectionReceipt> {
    let settlement = compute_attempt_settlement_on(tx, &request.lease_id)?;
    let resolution = compute_settlement_challenge_resolution_on(tx, &request.lease_id)?;
    if settlement.settlement.settlement_receipt_id != request.expected_settlement_receipt_id
        || settlement.event_digest != request.expected_settlement_event_digest
        || resolution.challenge_id != request.expected_challenge_id
        || resolution.challenge_event_digest != request.expected_challenge_event_digest
        || resolution.resolution_id != request.expected_resolution_id
        || resolution.event_digest != request.expected_resolution_event_digest
        || resolution.settlement_receipt_id != settlement.settlement.settlement_receipt_id
        || resolution.settlement_event_digest != settlement.event_digest
        || resolution.action != "accepted"
        || !resolution.correction_required
    {
        bail!("结算纠正引用的 accepted 挑战、决议或 Settlement Receipt 不匹配");
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
        bail!("已释放到 available 的 Settlement Receipt 不能走 pending 挑战纠正");
    }
    let corrected_consumer_micros = request
        .corrected_consumer_charge_fen
        .checked_mul(MICROS_PER_CNY_FEN)
        .context("纠正后的消费者金额换算溢出")?;
    let original_consumer_micros = settlement.settlement.amounts.consumer_charge_micros;
    let original_provider = settlement.settlement.amounts.provider_payable_micros;
    let original_platform = settlement.settlement.amounts.platform_margin_micros;
    if request.corrected_consumer_charge_fen >= settlement.consumer_charged_fen
        || request.corrected_provider_payable_micros > original_provider
        || request.corrected_platform_margin_micros > original_platform
        || corrected_consumer_micros
            != request
                .corrected_provider_payable_micros
                .checked_add(request.corrected_platform_margin_micros)
                .context("纠正后的贷方金额溢出")?
    {
        bail!("accepted 挑战只能形成守恒且向下的结算金额纠正");
    }
    let consumer_refund_fen = settlement
        .consumer_charged_fen
        .checked_sub(request.corrected_consumer_charge_fen)
        .context("消费者纠正退款金额下溢")?;
    let consumer_refund_micros = original_consumer_micros
        .checked_sub(corrected_consumer_micros)
        .context("消费者纠正退款微单位下溢")?;
    let provider_reversal = original_provider
        .checked_sub(request.corrected_provider_payable_micros)
        .context("Provider 纠正冲减金额下溢")?;
    let platform_reversal = original_platform
        .checked_sub(request.corrected_platform_margin_micros)
        .context("平台纠正冲减金额下溢")?;
    if consumer_refund_micros
        != provider_reversal
            .checked_add(platform_reversal)
            .context("纠正冲减金额溢出")?
    {
        bail!("消费者纠正退款必须等于 Provider 与平台冲减之和");
    }

    let correction_id = new_id("compute_settlement_correction");
    let corrected_at = Utc::now().to_rfc3339();
    let money = money::post_correction_money_on(
        tx,
        money::PostCorrectionMoneyInput {
            correction_id: &correction_id,
            settlement_receipt_id: &settlement.settlement.settlement_receipt_id,
            consumer_account_id: &settlement.settlement.consumer_account_id,
            provider_account_id: &settlement.settlement.provider_account_id,
            consumer_refund_fen,
            consumer_refund_micros,
            provider_reversal_micros: provider_reversal,
            platform_reversal_micros: platform_reversal,
            corrected_at: &corrected_at,
        },
    )?;
    let evidence_refs_digest = evidence_refs_digest(&request.evidence_refs)?;
    let mut receipt = ComputeSettlementCorrectionReceipt {
        schema: COMPUTE_SETTLEMENT_CORRECTION_SCHEMA.to_string(),
        correction_id,
        challenge_id: resolution.challenge_id,
        challenge_event_digest: resolution.challenge_event_digest,
        resolution_id: resolution.resolution_id,
        resolution_event_digest: resolution.event_digest,
        settlement_receipt_id: settlement.settlement.settlement_receipt_id,
        settlement_event_digest: settlement.event_digest,
        lease_id: request.lease_id.clone(),
        consumer_account_id: settlement.settlement.consumer_account_id,
        provider_account_id: settlement.settlement.provider_account_id,
        platform_account_id: money::PLATFORM_ACCOUNT_ID.to_string(),
        currency: "CNY".to_string(),
        original_consumer_charge_fen: settlement.consumer_charged_fen,
        original_consumer_charge_micros: original_consumer_micros,
        corrected_consumer_charge_fen: request.corrected_consumer_charge_fen,
        corrected_consumer_charge_micros: corrected_consumer_micros,
        consumer_refund_fen,
        consumer_refund_micros,
        original_provider_payable_micros: original_provider,
        corrected_provider_payable_micros: request.corrected_provider_payable_micros,
        provider_reversal_micros: provider_reversal,
        original_platform_margin_micros: original_platform,
        corrected_platform_margin_micros: request.corrected_platform_margin_micros,
        platform_reversal_micros: platform_reversal,
        consumer_balance_after_fen: money.consumer_balance_after_fen,
        provider_pending_balance_after_micros: money.provider.pending_after_micros,
        provider_account_revision_after: money.provider.revision_after,
        platform_pending_balance_after_micros: money.platform.pending_after_micros,
        platform_account_revision_after: money.platform.revision_after,
        statement: request.statement.clone(),
        evidence_refs: request.evidence_refs.clone(),
        evidence_refs_digest,
        policy_id: COMPUTE_SETTLEMENT_CORRECTION_POLICY_ID.to_string(),
        policy_version: COMPUTE_SETTLEMENT_CORRECTION_POLICY_VERSION,
        posting_id: money.posting_id,
        posting_digest: money.posting_digest,
        request_digest: request_digest.to_string(),
        event_digest: String::new(),
        corrected_by_user_id: request.corrected_by_user_id.clone(),
        corrected_at,
        balance_effect: "consumer_refunded_provider_and_platform_pending_reversed".to_string(),
        settlement_release_effect: "accepted_challenge_corrected_release_net_amounts".to_string(),
        replayed: false,
    };
    receipt.event_digest = correction_event_digest(&receipt)?;
    Ok(receipt)
}

pub(super) fn correction_request_digest(
    input: &CorrectComputeAttemptSettlementRequest,
) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(input)?)))
}

pub(super) fn evidence_refs_digest(values: &[String]) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(values)?)))
}

pub(super) fn correction_event_digest(
    receipt: &ComputeSettlementCorrectionReceipt,
) -> Result<String> {
    let mut canonical = receipt.clone();
    canonical.event_digest.clear();
    canonical.replayed = false;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

pub(super) fn persist_correction_on(
    conn: &Connection,
    request: &CorrectComputeAttemptSettlementRequest,
    receipt: &ComputeSettlementCorrectionReceipt,
    idempotency_scope: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_settlement_corrections (
           correction_id, challenge_id, challenge_event_digest,
           resolution_id, resolution_event_digest,
           settlement_receipt_id, settlement_event_digest, lease_id,
           consumer_account_id, provider_account_id,
           original_consumer_charge_micros, corrected_consumer_charge_micros,
           consumer_refund_micros, original_provider_payable_micros,
           corrected_provider_payable_micros, provider_reversal_micros,
           original_platform_margin_micros, corrected_platform_margin_micros,
           platform_reversal_micros, statement, evidence_refs_json,
           evidence_refs_digest, policy_id, policy_version, posting_id,
           posting_digest, request_json, request_digest, receipt_json,
           event_digest, idempotency_scope, idempotency_key,
           corrected_by_user_id, corrected_at, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,
                   ?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,
                   ?27,?28,?29,?30,?31,?32,?33,?34,?34)",
        params![
            receipt.correction_id,
            receipt.challenge_id,
            receipt.challenge_event_digest,
            receipt.resolution_id,
            receipt.resolution_event_digest,
            receipt.settlement_receipt_id,
            receipt.settlement_event_digest,
            receipt.lease_id,
            receipt.consumer_account_id,
            receipt.provider_account_id,
            receipt.original_consumer_charge_micros,
            receipt.corrected_consumer_charge_micros,
            receipt.consumer_refund_micros,
            receipt.original_provider_payable_micros,
            receipt.corrected_provider_payable_micros,
            receipt.provider_reversal_micros,
            receipt.original_platform_margin_micros,
            receipt.corrected_platform_margin_micros,
            receipt.platform_reversal_micros,
            receipt.statement,
            serde_json::to_string(&receipt.evidence_refs)?,
            receipt.evidence_refs_digest,
            receipt.policy_id,
            receipt.policy_version,
            receipt.posting_id,
            receipt.posting_digest,
            serde_json::to_string(request)?,
            receipt.request_digest,
            serde_json::to_string(receipt)?,
            receipt.event_digest,
            idempotency_scope,
            request.idempotency_key,
            receipt.corrected_by_user_id,
            receipt.corrected_at,
        ],
    )?;
    Ok(())
}

pub(super) fn correction_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredCorrection>> {
    correction_query(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn correction_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredCorrection>> {
    correction_query(conn, "WHERE lease_id=?1", params![lease_id])
}

pub(super) fn correction_by_resolution_on(
    conn: &Connection,
    resolution_id: &str,
) -> Result<Option<StoredCorrection>> {
    correction_query(conn, "WHERE resolution_id=?1", params![resolution_id])
}

pub(super) fn correction_by_id_on(
    conn: &Connection,
    correction_id: &str,
) -> Result<Option<StoredCorrection>> {
    correction_query(conn, "WHERE correction_id=?1", params![correction_id])
}

fn correction_query<P>(
    conn: &Connection,
    where_clause: &str,
    values: P,
) -> Result<Option<StoredCorrection>>
where
    P: rusqlite::Params,
{
    conn.query_row(
        &format!(
            "SELECT correction_id, challenge_id, challenge_event_digest,
                    resolution_id, resolution_event_digest,
                    settlement_receipt_id, settlement_event_digest, lease_id,
                    consumer_account_id, provider_account_id,
                    original_consumer_charge_micros,
                    corrected_consumer_charge_micros, consumer_refund_micros,
                    original_provider_payable_micros,
                    corrected_provider_payable_micros, provider_reversal_micros,
                    original_platform_margin_micros,
                    corrected_platform_margin_micros, platform_reversal_micros,
                    statement, evidence_refs_json, evidence_refs_digest,
                    policy_id, policy_version, posting_id, posting_digest,
                    request_json, request_digest, receipt_json, event_digest,
                    idempotency_scope, idempotency_key, corrected_by_user_id,
                    corrected_at
               FROM compute_settlement_corrections {where_clause}"
        ),
        values,
        stored_correction_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_correction_from_row(row: &Row<'_>) -> rusqlite::Result<StoredCorrection> {
    Ok(StoredCorrection {
        correction_id: row.get(0)?,
        challenge_id: row.get(1)?,
        challenge_event_digest: row.get(2)?,
        resolution_id: row.get(3)?,
        resolution_event_digest: row.get(4)?,
        settlement_receipt_id: row.get(5)?,
        settlement_event_digest: row.get(6)?,
        lease_id: row.get(7)?,
        consumer_account_id: row.get(8)?,
        provider_account_id: row.get(9)?,
        original_consumer_charge_micros: row.get(10)?,
        corrected_consumer_charge_micros: row.get(11)?,
        consumer_refund_micros: row.get(12)?,
        original_provider_payable_micros: row.get(13)?,
        corrected_provider_payable_micros: row.get(14)?,
        provider_reversal_micros: row.get(15)?,
        original_platform_margin_micros: row.get(16)?,
        corrected_platform_margin_micros: row.get(17)?,
        platform_reversal_micros: row.get(18)?,
        statement: row.get(19)?,
        evidence_refs_json: row.get(20)?,
        evidence_refs_digest: row.get(21)?,
        policy_id: row.get(22)?,
        policy_version: row.get(23)?,
        posting_id: row.get(24)?,
        posting_digest: row.get(25)?,
        request_json: row.get(26)?,
        request_digest: row.get(27)?,
        receipt_json: row.get(28)?,
        event_digest: row.get(29)?,
        idempotency_scope: row.get(30)?,
        idempotency_key: row.get(31)?,
        corrected_by_user_id: row.get(32)?,
        corrected_at: row.get(33)?,
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
