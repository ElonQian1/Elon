use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::super::{compute_attempt_settlements::calculation::MICROS_PER_CNY_FEN, new_id};

pub(super) const PLATFORM_ACCOUNT_ID: &str = "platform:compute_market";

#[derive(Debug, Clone)]
pub(super) struct PostCorrectionMoneyInput<'a> {
    pub correction_id: &'a str,
    pub settlement_receipt_id: &'a str,
    pub consumer_account_id: &'a str,
    pub provider_account_id: &'a str,
    pub consumer_refund_fen: i64,
    pub consumer_refund_micros: i64,
    pub provider_reversal_micros: i64,
    pub platform_reversal_micros: i64,
    pub corrected_at: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingReversalOutcome {
    pub pending_after_micros: i64,
    pub revision_after: i64,
}

#[derive(Debug, Clone)]
pub(super) struct CorrectionMoneyOutcome {
    pub posting_id: String,
    pub posting_digest: String,
    pub consumer_balance_after_fen: i64,
    pub provider: PendingReversalOutcome,
    pub platform: PendingReversalOutcome,
}

#[derive(Debug, Serialize)]
struct CorrectionPostingDigest<'a> {
    schema: &'static str,
    posting_id: &'a str,
    correction_id: &'a str,
    settlement_receipt_id: &'a str,
    currency: &'static str,
    consumer_refund_micros: i64,
    provider_pending_reversal_micros: i64,
    platform_pending_reversal_micros: i64,
    consumer_balance_after_fen: i64,
    provider_pending_after_micros: i64,
    provider_revision_after: i64,
    platform_pending_after_micros: i64,
    platform_revision_after: i64,
    posted_at: &'a str,
}

pub(super) fn post_correction_money_on(
    tx: &Transaction<'_>,
    input: PostCorrectionMoneyInput<'_>,
) -> Result<CorrectionMoneyOutcome> {
    if input.consumer_refund_fen <= 0
        || input.consumer_refund_micros <= 0
        || input.provider_reversal_micros < 0
        || input.platform_reversal_micros < 0
        || input.consumer_refund_micros
            != input
                .consumer_refund_fen
                .checked_mul(MICROS_PER_CNY_FEN)
                .context("消费者纠正退款换算溢出")?
        || input.consumer_refund_micros
            != input
                .provider_reversal_micros
                .checked_add(input.platform_reversal_micros)
                .context("结算纠正冲减金额溢出")?
    {
        bail!("结算纠正资金腿不守恒");
    }
    let consumer_balance_after = refund_consumer_on(
        tx,
        input.consumer_account_id,
        input.consumer_refund_fen,
        input.corrected_at,
    )?;
    let provider = reverse_pending_on(
        tx,
        "provider",
        input.provider_account_id,
        input.provider_reversal_micros,
        input.corrected_at,
    )?;
    let platform = reverse_pending_on(
        tx,
        "platform",
        PLATFORM_ACCOUNT_ID,
        input.platform_reversal_micros,
        input.corrected_at,
    )?;
    let posting_id = new_id("compute_settlement_correction_posting");
    let posting_digest = correction_posting_digest(
        &posting_id,
        &input,
        consumer_balance_after,
        &provider,
        &platform,
    )?;
    tx.execute(
        "INSERT INTO compute_settlement_correction_postings (
           posting_id, correction_id, settlement_receipt_id, currency,
           consumer_refund_micros, provider_pending_reversal_micros,
           platform_pending_reversal_micros, posting_digest, posted_at
         ) VALUES (?1,?2,?3,'CNY',?4,?5,?6,?7,?8)",
        params![
            posting_id,
            input.correction_id,
            input.settlement_receipt_id,
            input.consumer_refund_micros,
            input.provider_reversal_micros,
            input.platform_reversal_micros,
            posting_digest,
            input.corrected_at,
        ],
    )?;
    let consumer_balance_after_micros = consumer_balance_after
        .checked_mul(MICROS_PER_CNY_FEN)
        .context("消费者纠正后余额换算溢出")?;
    let legs = [
        (
            1,
            "consumer",
            "consumer_correction_refund",
            input.consumer_account_id,
            "credit",
            input.consumer_refund_micros,
            "consumer_balance",
            consumer_balance_after_micros,
            None,
        ),
        (
            2,
            "provider",
            "provider_pending_reversal",
            input.provider_account_id,
            "debit",
            input.provider_reversal_micros,
            "pending",
            provider.pending_after_micros,
            Some(provider.revision_after),
        ),
        (
            3,
            "platform",
            "platform_pending_reversal",
            PLATFORM_ACCOUNT_ID,
            "debit",
            input.platform_reversal_micros,
            "pending",
            platform.pending_after_micros,
            Some(platform.revision_after),
        ),
    ];
    for (
        line_no,
        account_kind,
        leg_kind,
        account_id,
        direction,
        amount,
        state,
        balance_after,
        revision_after,
    ) in legs
    {
        tx.execute(
            "INSERT INTO compute_settlement_correction_ledger_legs (
               posting_id, line_no, account_kind, leg_kind, account_id,
               currency, direction, amount_micros, balance_state,
               balance_after_micros, account_revision_after
             ) VALUES (?1,?2,?3,?4,?5,'CNY',?6,?7,?8,?9,?10)",
            params![
                posting_id,
                line_no,
                account_kind,
                leg_kind,
                account_id,
                direction,
                amount,
                state,
                balance_after,
                revision_after,
            ],
        )?;
    }
    Ok(CorrectionMoneyOutcome {
        posting_id,
        posting_digest,
        consumer_balance_after_fen: consumer_balance_after,
        provider,
        platform,
    })
}

fn refund_consumer_on(
    tx: &Transaction<'_>,
    consumer_account_id: &str,
    refund_fen: i64,
    corrected_at: &str,
) -> Result<i64> {
    let current = tx
        .query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id=?1",
            params![consumer_account_id.trim()],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("消费者余额账户不存在"))?;
    let next = current
        .checked_add(refund_fen)
        .context("消费者纠正退款后余额溢出")?;
    let changed = tx.execute(
        "UPDATE user_balance SET balance_fen=?1, updated_at=?2
          WHERE user_id=?3 AND balance_fen=?4",
        params![next, corrected_at, consumer_account_id.trim(), current],
    )?;
    if changed != 1 {
        bail!("消费者余额并发纠正失败");
    }
    Ok(next)
}

fn reverse_pending_on(
    tx: &Transaction<'_>,
    account_kind: &str,
    account_id: &str,
    amount_micros: i64,
    corrected_at: &str,
) -> Result<PendingReversalOutcome> {
    let (pending, revision) = tx
        .query_row(
            "SELECT pending_micros, revision
               FROM compute_settlement_account_balances
              WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'",
            params![account_kind, account_id.trim()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("结算纠正 pending 账户不存在"))?;
    let pending_after = pending
        .checked_sub(amount_micros)
        .filter(|value| *value >= 0)
        .ok_or_else(|| anyhow!("结算纠正冲减金额超过 pending 余额"))?;
    let revision_after = revision.checked_add(1).context("账户修订号溢出")?;
    let changed = tx.execute(
        "UPDATE compute_settlement_account_balances
            SET pending_micros=?4, revision=?5, updated_at=?6
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND revision=?3 AND pending_micros=?7",
        params![
            account_kind,
            account_id.trim(),
            revision,
            pending_after,
            revision_after,
            corrected_at,
            pending,
        ],
    )?;
    if changed != 1 {
        bail!("结算纠正 pending 账户并发更新失败");
    }
    Ok(PendingReversalOutcome {
        pending_after_micros: pending_after,
        revision_after,
    })
}

pub(super) fn correction_posting_digest(
    posting_id: &str,
    input: &PostCorrectionMoneyInput<'_>,
    consumer_balance_after_fen: i64,
    provider: &PendingReversalOutcome,
    platform: &PendingReversalOutcome,
) -> Result<String> {
    let payload = CorrectionPostingDigest {
        schema: "compute_federation.settlement_correction_posting.v1",
        posting_id,
        correction_id: input.correction_id,
        settlement_receipt_id: input.settlement_receipt_id,
        currency: "CNY",
        consumer_refund_micros: input.consumer_refund_micros,
        provider_pending_reversal_micros: input.provider_reversal_micros,
        platform_pending_reversal_micros: input.platform_reversal_micros,
        consumer_balance_after_fen,
        provider_pending_after_micros: provider.pending_after_micros,
        provider_revision_after: provider.revision_after,
        platform_pending_after_micros: platform.pending_after_micros,
        platform_revision_after: platform.revision_after,
        posted_at: input.corrected_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

pub(super) fn correction_posting_row_on(
    conn: &Connection,
    posting_id: &str,
) -> Result<Option<(String, String, i64, i64, i64, String, String)>> {
    conn.query_row(
        "SELECT correction_id, settlement_receipt_id, consumer_refund_micros,
                provider_pending_reversal_micros,
                platform_pending_reversal_micros, posting_digest, posted_at
           FROM compute_settlement_correction_postings WHERE posting_id=?1",
        params![posting_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )
    .optional()
    .map_err(Into::into)
}
