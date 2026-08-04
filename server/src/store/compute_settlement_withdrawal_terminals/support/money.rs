use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::super::common::new_id;

#[derive(Debug, Clone)]
pub(super) struct PostTerminalInput<'a> {
    pub terminal_id: &'a str,
    pub withdrawal_id: &'a str,
    pub provider_account_id: &'a str,
    pub action: &'a str,
    pub amount_micros: i64,
    pub balance_returned_micros: i64,
    pub terminal_at: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TerminalMoneyOutcome {
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
    terminal_id: &'a str,
    withdrawal_id: &'a str,
    provider_account_id: &'a str,
    currency: &'static str,
    action: &'a str,
    amount_micros: i64,
    balance_returned_micros: i64,
    available_after_micros: i64,
    withdrawn_after_micros: i64,
    revision_after: i64,
    posted_at: &'a str,
}

pub(super) fn post_terminal_on(
    tx: &Transaction<'_>,
    input: PostTerminalInput<'_>,
) -> Result<TerminalMoneyOutcome> {
    if input.amount_micros <= 0
        || input.balance_returned_micros < 0
        || input.balance_returned_micros > input.amount_micros
        || input.provider_account_id.trim().is_empty()
    {
        bail!("提现终态账户或金额无效");
    }
    let (available, withdrawn, revision) = current_balance_on(tx, input.provider_account_id)?;
    let (available_after, withdrawn_after, revision_after) = if input.balance_returned_micros > 0 {
        let available_after = available
            .checked_add(input.balance_returned_micros)
            .context("available 返还余额溢出")?;
        let withdrawn_after = withdrawn
            .checked_sub(input.balance_returned_micros)
            .filter(|value| *value >= 0)
            .ok_or_else(|| anyhow!("提现终态返还额超过 withdrawn 保留余额"))?;
        let revision_after = revision.checked_add(1).context("账户修订号溢出")?;
        let changed = tx.execute(
            "UPDATE compute_settlement_account_balances
                    SET available_micros=?4, withdrawn_micros=?5,
                        revision=?6, updated_at=?7
                  WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
                    AND revision=?2 AND available_micros=?3 AND withdrawn_micros=?8",
            params![
                input.provider_account_id,
                revision,
                available,
                available_after,
                withdrawn_after,
                revision_after,
                input.terminal_at,
                withdrawn,
            ],
        )?;
        if changed != 1 {
            bail!("提现终态账户并发更新失败");
        }
        (available_after, withdrawn_after, revision_after)
    } else {
        (available, withdrawn, revision)
    };

    let posting_id = new_id("compute_settlement_withdrawal_terminal_posting");
    let outcome = TerminalMoneyOutcome {
        posting_digest: posting_digest(
            &posting_id,
            &input,
            available_after,
            withdrawn_after,
            revision_after,
        )?,
        posting_id,
        available_after_micros: available_after,
        withdrawn_after_micros: withdrawn_after,
        revision_after,
    };
    tx.execute(
        "INSERT INTO compute_settlement_withdrawal_terminal_postings (
           posting_id, terminal_id, withdrawal_id, provider_account_id,
           currency, action, amount_micros, balance_returned_micros,
           posting_digest, posted_at
         ) VALUES (?1,?2,?3,?4,'CNY',?5,?6,?7,?8,?9)",
        params![
            outcome.posting_id,
            input.terminal_id,
            input.withdrawal_id,
            input.provider_account_id,
            input.action,
            input.amount_micros,
            input.balance_returned_micros,
            outcome.posting_digest,
            input.terminal_at,
        ],
    )?;
    if input.balance_returned_micros > 0 {
        insert_refund_legs(tx, &input, &outcome)?;
    }
    Ok(outcome)
}

fn current_balance_on(conn: &Connection, provider_account_id: &str) -> Result<(i64, i64, i64)> {
    conn.query_row(
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
    .ok_or_else(|| anyhow!("Provider 结算账户不存在"))
}

fn insert_refund_legs(
    tx: &Transaction<'_>,
    input: &PostTerminalInput<'_>,
    outcome: &TerminalMoneyOutcome,
) -> Result<()> {
    let legs = [
        (
            1,
            "provider_withdrawn_terminal_release",
            "debit",
            "withdrawn",
            outcome.withdrawn_after_micros,
        ),
        (
            2,
            "provider_available_terminal_return",
            "credit",
            "available",
            outcome.available_after_micros,
        ),
    ];
    for (line_no, leg_kind, direction, balance_state, balance_after) in legs {
        tx.execute(
            "INSERT INTO compute_settlement_withdrawal_terminal_ledger_legs (
               posting_id, line_no, account_kind, leg_kind, account_id,
               currency, direction, amount_micros, balance_state,
               balance_after_micros, account_revision_after
             ) VALUES (?1,?2,'provider',?3,?4,'CNY',?5,?6,?7,?8,?9)",
            params![
                outcome.posting_id,
                line_no,
                leg_kind,
                input.provider_account_id,
                direction,
                input.balance_returned_micros,
                balance_state,
                balance_after,
                outcome.revision_after,
            ],
        )?;
    }
    Ok(())
}

pub(super) fn posting_digest(
    posting_id: &str,
    input: &PostTerminalInput<'_>,
    available_after_micros: i64,
    withdrawn_after_micros: i64,
    revision_after: i64,
) -> Result<String> {
    let payload = PostingDigest {
        schema: "compute_federation.settlement_withdrawal_terminal_posting.v1",
        posting_id,
        terminal_id: input.terminal_id,
        withdrawal_id: input.withdrawal_id,
        provider_account_id: input.provider_account_id,
        currency: "CNY",
        action: input.action,
        amount_micros: input.amount_micros,
        balance_returned_micros: input.balance_returned_micros,
        available_after_micros,
        withdrawn_after_micros,
        revision_after,
        posted_at: input.terminal_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

pub(super) fn posting_row_on(
    conn: &Connection,
    posting_id: &str,
) -> Result<Option<(String, String, String, i64, i64, String, String)>> {
    conn.query_row(
        "SELECT terminal_id, withdrawal_id, action, amount_micros,
                balance_returned_micros, posting_digest, posted_at
           FROM compute_settlement_withdrawal_terminal_postings WHERE posting_id=?1",
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
