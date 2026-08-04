use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::super::new_id;

pub(super) const PLATFORM_ACCOUNT_ID: &str = "platform:compute_market";

#[derive(Debug, Clone)]
pub(super) struct PostReleaseMoneyInput<'a> {
    pub release_id: &'a str,
    pub settlement_receipt_id: &'a str,
    pub provider_account_id: &'a str,
    pub provider_released_micros: i64,
    pub platform_released_micros: i64,
    pub released_at: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AccountReleaseOutcome {
    pub pending_after_micros: i64,
    pub available_after_micros: i64,
    pub revision_after: i64,
}

#[derive(Debug, Clone)]
pub(super) struct ReleaseMoneyOutcome {
    pub posting_id: String,
    pub posting_digest: String,
    pub provider: AccountReleaseOutcome,
    pub platform: AccountReleaseOutcome,
}

#[derive(Debug, Serialize)]
struct ReleasePostingDigest<'a> {
    schema: &'static str,
    posting_id: &'a str,
    release_id: &'a str,
    settlement_receipt_id: &'a str,
    currency: &'static str,
    provider_released_micros: i64,
    platform_released_micros: i64,
    provider_pending_after_micros: i64,
    provider_available_after_micros: i64,
    provider_revision_after: i64,
    platform_pending_after_micros: i64,
    platform_available_after_micros: i64,
    platform_revision_after: i64,
    posted_at: &'a str,
}

pub(super) fn post_release_money_on(
    tx: &Transaction<'_>,
    input: PostReleaseMoneyInput<'_>,
) -> Result<ReleaseMoneyOutcome> {
    if input.provider_released_micros < 0 || input.platform_released_micros < 0 {
        bail!("待结算释放金额不能为负数");
    }
    let provider = transfer_pending_to_available_on(
        tx,
        "provider",
        input.provider_account_id,
        input.provider_released_micros,
        input.released_at,
    )?;
    let platform = transfer_pending_to_available_on(
        tx,
        "platform",
        PLATFORM_ACCOUNT_ID,
        input.platform_released_micros,
        input.released_at,
    )?;
    let posting_id = new_id("compute_settlement_release_posting");
    let posting_digest = release_posting_digest(&posting_id, &input, &provider, &platform)?;
    tx.execute(
        "INSERT INTO compute_settlement_release_postings (
           posting_id, release_id, settlement_receipt_id, currency,
           provider_released_micros, platform_released_micros,
           posting_digest, posted_at
         ) VALUES (?1,?2,?3,'CNY',?4,?5,?6,?7)",
        params![
            posting_id,
            input.release_id,
            input.settlement_receipt_id,
            input.provider_released_micros,
            input.platform_released_micros,
            posting_digest,
            input.released_at,
        ],
    )?;
    let legs = [
        (
            1,
            "provider",
            "provider_pending_release",
            input.provider_account_id,
            "debit",
            input.provider_released_micros,
            "pending",
            provider.pending_after_micros,
            provider.revision_after,
        ),
        (
            2,
            "provider",
            "provider_available_credit",
            input.provider_account_id,
            "credit",
            input.provider_released_micros,
            "available",
            provider.available_after_micros,
            provider.revision_after,
        ),
        (
            3,
            "platform",
            "platform_pending_release",
            PLATFORM_ACCOUNT_ID,
            "debit",
            input.platform_released_micros,
            "pending",
            platform.pending_after_micros,
            platform.revision_after,
        ),
        (
            4,
            "platform",
            "platform_available_credit",
            PLATFORM_ACCOUNT_ID,
            "credit",
            input.platform_released_micros,
            "available",
            platform.available_after_micros,
            platform.revision_after,
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
            "INSERT INTO compute_settlement_release_ledger_legs (
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
    Ok(ReleaseMoneyOutcome {
        posting_id,
        posting_digest,
        provider,
        platform,
    })
}

fn transfer_pending_to_available_on(
    tx: &Transaction<'_>,
    account_kind: &str,
    account_id: &str,
    amount_micros: i64,
    released_at: &str,
) -> Result<AccountReleaseOutcome> {
    if amount_micros < 0 || account_id.trim().is_empty() {
        bail!("待结算释放账户或金额无效");
    }
    let (pending, available, revision) = tx
        .query_row(
            "SELECT pending_micros, available_micros, revision
               FROM compute_settlement_account_balances
              WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'",
            params![account_kind, account_id.trim()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("待结算释放账户不存在"))?;
    let pending_after = pending
        .checked_sub(amount_micros)
        .filter(|value| *value >= 0)
        .ok_or_else(|| anyhow!("待结算释放金额超过 pending 余额"))?;
    let available_after = available
        .checked_add(amount_micros)
        .context("available 余额溢出")?;
    let revision_after = revision.checked_add(1).context("账户修订号溢出")?;
    let changed = tx.execute(
        "UPDATE compute_settlement_account_balances
            SET pending_micros=?4, available_micros=?5,
                revision=?6, updated_at=?7
          WHERE account_kind=?1 AND account_id=?2 AND currency='CNY'
            AND revision=?3 AND pending_micros=?8 AND available_micros=?9",
        params![
            account_kind,
            account_id.trim(),
            revision,
            pending_after,
            available_after,
            revision_after,
            released_at,
            pending,
            available,
        ],
    )?;
    if changed != 1 {
        bail!("待结算释放账户并发更新失败");
    }
    Ok(AccountReleaseOutcome {
        pending_after_micros: pending_after,
        available_after_micros: available_after,
        revision_after,
    })
}

pub(super) fn release_posting_digest(
    posting_id: &str,
    input: &PostReleaseMoneyInput<'_>,
    provider: &AccountReleaseOutcome,
    platform: &AccountReleaseOutcome,
) -> Result<String> {
    let payload = ReleasePostingDigest {
        schema: "compute_federation.settlement_release_posting.v1",
        posting_id,
        release_id: input.release_id,
        settlement_receipt_id: input.settlement_receipt_id,
        currency: "CNY",
        provider_released_micros: input.provider_released_micros,
        platform_released_micros: input.platform_released_micros,
        provider_pending_after_micros: provider.pending_after_micros,
        provider_available_after_micros: provider.available_after_micros,
        provider_revision_after: provider.revision_after,
        platform_pending_after_micros: platform.pending_after_micros,
        platform_available_after_micros: platform.available_after_micros,
        platform_revision_after: platform.revision_after,
        posted_at: input.released_at,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

pub(super) fn release_posting_row_on(
    conn: &Connection,
    posting_id: &str,
) -> Result<Option<(String, String, i64, i64, String, String)>> {
    conn.query_row(
        "SELECT release_id, settlement_receipt_id,
                provider_released_micros, platform_released_micros,
                posting_digest, posted_at
           FROM compute_settlement_release_postings WHERE posting_id=?1",
        params![posting_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )
    .optional()
    .map_err(Into::into)
}
