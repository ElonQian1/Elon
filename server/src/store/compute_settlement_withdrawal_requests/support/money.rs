use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::super::common::new_id;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WithdrawalReserveOutcome {
    pub posting_id: String,
    pub posting_digest: String,
    pub available_after_micros: i64,
    pub withdrawn_after_micros: i64,
    pub revision_after: i64,
}

#[derive(Debug, Serialize)]
struct PostingDigest<'a> {
    schema: &'static str,
    posting_id: &'a str,
    withdrawal_id: &'a str,
    provider_account_id: &'a str,
    currency: &'static str,
    amount_micros: i64,
    available_after_micros: i64,
    withdrawn_after_micros: i64,
    revision_after: i64,
    posted_at: &'a str,
}

pub(super) fn reserve_withdrawal_on(
    tx: &Transaction<'_>,
    withdrawal_id: &str,
    provider_account_id: &str,
    amount_micros: i64,
    requested_at: &str,
) -> Result<WithdrawalReserveOutcome> {
    if amount_micros <= 0 || provider_account_id.trim().is_empty() {
        bail!("提现申请账户或金额无效");
    }
    let (available, withdrawn, revision) = tx
        .query_row(
            "SELECT available_micros, withdrawn_micros, revision
               FROM compute_settlement_account_balances
              WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'",
            params![provider_account_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Provider 结算账户不存在"))?;
    let available_after = available
        .checked_sub(amount_micros)
        .filter(|value| *value >= 0)
        .ok_or_else(|| anyhow!("提现申请金额超过 available 余额"))?;
    let withdrawn_after = withdrawn
        .checked_add(amount_micros)
        .context("withdrawn 冻结余额溢出")?;
    let revision_after = revision.checked_add(1).context("账户修订号溢出")?;
    let changed = tx.execute(
        "UPDATE compute_settlement_account_balances
            SET available_micros=?4, withdrawn_micros=?5,
                revision=?6, updated_at=?7
          WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
            AND revision=?2 AND available_micros=?3 AND withdrawn_micros=?8",
        params![
            provider_account_id,
            revision,
            available,
            available_after,
            withdrawn_after,
            revision_after,
            requested_at,
            withdrawn,
        ],
    )?;
    if changed != 1 {
        bail!("提现申请账户并发更新失败");
    }

    let posting_id = new_id("compute_settlement_withdrawal_request_posting");
    let posting_digest = posting_digest(
        &posting_id,
        withdrawal_id,
        provider_account_id,
        amount_micros,
        available_after,
        withdrawn_after,
        revision_after,
        requested_at,
    )?;
    tx.execute(
        "INSERT INTO compute_settlement_withdrawal_request_postings (
           posting_id, withdrawal_id, provider_account_id, currency,
           amount_micros, posting_digest, posted_at
         ) VALUES (?1,?2,?3,'CNY',?4,?5,?6)",
        params![
            posting_id,
            withdrawal_id,
            provider_account_id,
            amount_micros,
            posting_digest,
            requested_at,
        ],
    )?;
    let legs = [
        (
            1,
            "provider_available_withdrawal_reserve",
            "debit",
            "available",
            available_after,
        ),
        (
            2,
            "provider_withdrawn_reserve_credit",
            "credit",
            "withdrawn",
            withdrawn_after,
        ),
    ];
    for (line_no, leg_kind, direction, balance_state, balance_after) in legs {
        tx.execute(
            "INSERT INTO compute_settlement_withdrawal_request_ledger_legs (
               posting_id, line_no, account_kind, leg_kind, account_id,
               currency, direction, amount_micros, balance_state,
               balance_after_micros, account_revision_after
             ) VALUES (?1,?2,'provider',?3,?4,'CNY',?5,?6,?7,?8,?9)",
            params![
                posting_id,
                line_no,
                leg_kind,
                provider_account_id,
                direction,
                amount_micros,
                balance_state,
                balance_after,
                revision_after,
            ],
        )?;
    }
    Ok(WithdrawalReserveOutcome {
        posting_id,
        posting_digest,
        available_after_micros: available_after,
        withdrawn_after_micros: withdrawn_after,
        revision_after,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn posting_digest(
    posting_id: &str,
    withdrawal_id: &str,
    provider_account_id: &str,
    amount_micros: i64,
    available_after_micros: i64,
    withdrawn_after_micros: i64,
    revision_after: i64,
    posted_at: &str,
) -> Result<String> {
    let payload = PostingDigest {
        schema: "compute_federation.settlement_withdrawal_request_posting.v1",
        posting_id,
        withdrawal_id,
        provider_account_id,
        currency: "CNY",
        amount_micros,
        available_after_micros,
        withdrawn_after_micros,
        revision_after,
        posted_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

pub(super) fn posting_row_on(
    conn: &Connection,
    posting_id: &str,
) -> Result<Option<(String, String, i64, String, String)>> {
    conn.query_row(
        "SELECT withdrawal_id, provider_account_id, amount_micros,
                posting_digest, posted_at
           FROM compute_settlement_withdrawal_request_postings
          WHERE posting_id=?1",
        params![posting_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )
    .optional()
    .map_err(Into::into)
}
