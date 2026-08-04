use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::{
    billing_reservations::{
        settle_compute_billing_reservation_on, ComputeBillingSettlementOutcome,
    },
    new_id,
};

const PLATFORM_ACCOUNT_ID: &str = "platform:compute_market";

#[derive(Debug, Clone)]
pub(super) struct PostSettlementMoneyInput<'a> {
    pub settlement_receipt_id: &'a str,
    pub consumer_account_id: &'a str,
    pub provider_account_id: &'a str,
    pub budget_reservation_id: &'a str,
    pub budget_reserved_fen: i64,
    pub consumer_charge_fen: i64,
    pub consumer_charge_micros: i64,
    pub provider_payable_micros: i64,
    pub platform_margin_micros: i64,
    pub consumer_refund_micros: i64,
    pub settled_at: &'a str,
}

#[derive(Debug, Clone)]
pub(super) struct SettlementMoneyOutcome {
    pub billing: ComputeBillingSettlementOutcome,
    pub posting_id: String,
    pub posting_digest: String,
    pub provider_pending_balance_micros: i64,
    pub platform_pending_balance_micros: i64,
}

#[derive(Debug, Serialize)]
struct PostingDigest<'a> {
    schema: &'static str,
    posting_id: &'a str,
    settlement_receipt_id: &'a str,
    currency: &'static str,
    consumer_charge_micros: i64,
    consumer_refund_micros: i64,
    provider_pending_micros: i64,
    platform_pending_micros: i64,
    posted_at: &'a str,
}

pub(super) fn post_settlement_money_on(
    tx: &Transaction<'_>,
    input: PostSettlementMoneyInput<'_>,
) -> Result<SettlementMoneyOutcome> {
    if input.consumer_charge_micros
        != input
            .provider_payable_micros
            .checked_add(input.platform_margin_micros)
            .ok_or_else(|| anyhow!("结算贷方金额溢出"))?
    {
        bail!("消费者结算金额必须等于 Provider 应得与平台价差之和");
    }
    let billing = settle_compute_billing_reservation_on(
        tx,
        input.budget_reservation_id,
        input.consumer_account_id,
        input.budget_reserved_fen,
        input.consumer_charge_fen,
        input.settled_at,
    )?;
    let provider_pending = credit_pending_account_on(
        tx,
        "provider",
        input.provider_account_id,
        input.provider_payable_micros,
        input.settled_at,
    )?;
    let platform_pending = credit_pending_account_on(
        tx,
        "platform",
        PLATFORM_ACCOUNT_ID,
        input.platform_margin_micros,
        input.settled_at,
    )?;
    let posting_id = new_id("compute_settlement_posting");
    let posting_digest = settlement_posting_digest(
        &posting_id,
        input.settlement_receipt_id,
        input.consumer_charge_micros,
        input.consumer_refund_micros,
        input.provider_payable_micros,
        input.platform_margin_micros,
        input.settled_at,
    )?;
    tx.execute(
        "INSERT INTO compute_settlement_postings (
           posting_id, settlement_receipt_id, currency,
           consumer_charge_micros, consumer_refund_micros,
           provider_pending_micros, platform_pending_micros,
           posting_digest, posted_at
         ) VALUES (?1,?2,'CNY',?3,?4,?5,?6,?7,?8)",
        params![
            posting_id,
            input.settlement_receipt_id,
            input.consumer_charge_micros,
            input.consumer_refund_micros,
            input.provider_payable_micros,
            input.platform_margin_micros,
            posting_digest,
            input.settled_at,
        ],
    )?;
    let legs = [
        (
            1,
            "consumer_capture",
            input.consumer_account_id,
            "debit",
            input.consumer_charge_micros,
            "preauthorization",
            None,
        ),
        (
            2,
            "consumer_refund",
            input.consumer_account_id,
            "release",
            input.consumer_refund_micros,
            "preauthorization",
            None,
        ),
        (
            3,
            "provider_pending",
            input.provider_account_id,
            "credit",
            input.provider_payable_micros,
            "pending",
            Some(provider_pending),
        ),
        (
            4,
            "platform_pending",
            PLATFORM_ACCOUNT_ID,
            "credit",
            input.platform_margin_micros,
            "pending",
            Some(platform_pending),
        ),
    ];
    for (line_no, kind, account_id, direction, amount, state, balance_after) in legs {
        tx.execute(
            "INSERT INTO compute_settlement_ledger_legs (
               posting_id, line_no, leg_kind, account_id, currency,
               direction, amount_micros, balance_state, balance_after_micros
             ) VALUES (?1,?2,?3,?4,'CNY',?5,?6,?7,?8)",
            params![
                posting_id,
                line_no,
                kind,
                account_id,
                direction,
                amount,
                state,
                balance_after
            ],
        )?;
    }
    Ok(SettlementMoneyOutcome {
        billing,
        posting_id,
        posting_digest,
        provider_pending_balance_micros: provider_pending,
        platform_pending_balance_micros: platform_pending,
    })
}

pub(super) fn settlement_posting_digest(
    posting_id: &str,
    settlement_receipt_id: &str,
    consumer_charge_micros: i64,
    consumer_refund_micros: i64,
    provider_pending_micros: i64,
    platform_pending_micros: i64,
    posted_at: &str,
) -> Result<String> {
    let payload = PostingDigest {
        schema: "compute_federation.settlement_posting.v1",
        posting_id,
        settlement_receipt_id,
        currency: "CNY",
        consumer_charge_micros,
        consumer_refund_micros,
        provider_pending_micros,
        platform_pending_micros,
        posted_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

fn credit_pending_account_on(
    tx: &Transaction<'_>,
    account_kind: &str,
    account_id: &str,
    amount_micros: i64,
    settled_at: &str,
) -> Result<i64> {
    if amount_micros < 0 || account_id.trim().is_empty() {
        bail!("待结算账户或金额无效");
    }
    let existing = tx
        .query_row(
            "SELECT pending_micros, revision
               FROM compute_settlement_account_balances
              WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'",
            params![account_kind, account_id.trim()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    match existing {
        Some((pending, revision)) => {
            let next = pending
                .checked_add(amount_micros)
                .context("待结算余额溢出")?;
            let changed = tx.execute(
                "UPDATE compute_settlement_account_balances
                    SET pending_micros=?4, revision=revision+1, updated_at=?5
                  WHERE account_kind=?1 AND account_id=?2 AND currency='CNY' AND revision=?3",
                params![account_kind, account_id.trim(), revision, next, settled_at],
            )?;
            if changed != 1 {
                bail!("待结算余额并发更新失败");
            }
            Ok(next)
        }
        None => {
            tx.execute(
                "INSERT INTO compute_settlement_account_balances (
                   account_kind, account_id, currency, pending_micros,
                   available_micros, disputed_micros, withdrawn_micros,
                   revision, updated_at
                 ) VALUES (?1,?2,'CNY',?3,0,0,0,1,?4)",
                params![account_kind, account_id.trim(), amount_micros, settled_at],
            )?;
            Ok(amount_micros)
        }
    }
}

pub(super) fn posting_row_on(
    conn: &Connection,
    posting_id: &str,
) -> Result<Option<(String, i64, i64, i64, i64, String, String)>> {
    conn.query_row(
        "SELECT settlement_receipt_id, consumer_charge_micros,
                consumer_refund_micros, provider_pending_micros,
                platform_pending_micros, posting_digest, posted_at
           FROM compute_settlement_postings WHERE posting_id=?1",
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
