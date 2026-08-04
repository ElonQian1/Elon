use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::Store;

const PLATFORM_ACCOUNT_ID: &str = "platform:compute_market";
const PLATFORM_ACCOUNT_VIEW_SCHEMA: &str = "compute_federation.platform_settlement_account_view.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputePlatformSettlementAccountView {
    pub schema: String,
    pub account_kind: String,
    pub account_id: String,
    pub currency: String,
    pub pending_micros: i64,
    pub available_micros: i64,
    pub disputed_micros: i64,
    pub withdrawn_micros: i64,
    pub account_revision: i64,
    pub updated_at: Option<String>,
    pub settlement_posting_count: i64,
    pub gross_margin_credited_micros: i64,
    pub correction_posting_count: i64,
    pub corrected_margin_micros: i64,
    pub release_posting_count: i64,
    pub released_margin_micros: i64,
    pub projection_digest: String,
    pub audit_status: String,
    pub withdrawal_effect: String,
}

impl Store {
    pub(crate) fn compute_platform_settlement_account_view(
        &self,
    ) -> Result<ComputePlatformSettlementAccountView> {
        let conn = self.conn()?;
        platform_account_view_on(&conn)
    }
}

fn platform_account_view_on(conn: &Connection) -> Result<ComputePlatformSettlementAccountView> {
    let current = current_balance_on(conn)?;
    let settlement = settlement_margin_on(conn)?;
    let correction = corrected_margin_on(conn)?;
    let release_pending = released_pending_margin_on(conn)?;
    let release_available = released_available_margin_on(conn)?;
    if release_pending != release_available {
        bail!("平台结算释放的 pending 借记与 available 贷记不守恒");
    }
    let pending = settlement
        .amount_micros
        .checked_sub(correction.amount_micros)
        .and_then(|value| value.checked_sub(release_pending.amount_micros))
        .context("平台 pending 账本投影溢出")?;
    if pending < 0 {
        bail!("平台 pending 账本投影不能为负数");
    }
    if current.pending != pending
        || current.available != release_available.amount_micros
        || current.disputed != 0
        || current.withdrawn != 0
    {
        bail!("平台结算账户与不可变账本投影不一致");
    }
    let mut view = ComputePlatformSettlementAccountView {
        schema: PLATFORM_ACCOUNT_VIEW_SCHEMA.to_string(),
        account_kind: "platform".to_string(),
        account_id: PLATFORM_ACCOUNT_ID.to_string(),
        currency: "CNY".to_string(),
        pending_micros: current.pending,
        available_micros: current.available,
        disputed_micros: current.disputed,
        withdrawn_micros: current.withdrawn,
        account_revision: current.revision,
        updated_at: current.updated_at,
        settlement_posting_count: settlement.posting_count,
        gross_margin_credited_micros: settlement.amount_micros,
        correction_posting_count: correction.posting_count,
        corrected_margin_micros: correction.amount_micros,
        release_posting_count: release_pending.posting_count,
        released_margin_micros: release_pending.amount_micros,
        projection_digest: String::new(),
        audit_status: "verified_from_append_only_ledgers".to_string(),
        withdrawal_effect: "platform_withdrawal_not_implemented".to_string(),
    };
    view.projection_digest = projection_digest(&view)?;
    Ok(view)
}

#[derive(Default)]
struct CurrentBalance {
    pending: i64,
    available: i64,
    disputed: i64,
    withdrawn: i64,
    revision: i64,
    updated_at: Option<String>,
}

fn current_balance_on(conn: &Connection) -> Result<CurrentBalance> {
    Ok(conn
        .query_row(
            "SELECT pending_micros, available_micros, disputed_micros,
                    withdrawn_micros, revision, updated_at
               FROM compute_settlement_account_balances
              WHERE account_kind='platform' AND account_id=?1 AND currency='CNY'",
            params![PLATFORM_ACCOUNT_ID],
            |row| {
                Ok(CurrentBalance {
                    pending: row.get(0)?,
                    available: row.get(1)?,
                    disputed: row.get(2)?,
                    withdrawn: row.get(3)?,
                    revision: row.get(4)?,
                    updated_at: Some(row.get(5)?),
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LedgerAggregate {
    posting_count: i64,
    amount_micros: i64,
}

fn settlement_margin_on(conn: &Connection) -> Result<LedgerAggregate> {
    aggregate_on(
        conn,
        "SELECT COUNT(DISTINCT posting_id), COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_ledger_legs
          WHERE leg_kind='platform_pending' AND account_id=?1
            AND currency='CNY' AND direction='credit' AND balance_state='pending'",
    )
}

fn corrected_margin_on(conn: &Connection) -> Result<LedgerAggregate> {
    aggregate_on(
        conn,
        "SELECT COUNT(DISTINCT posting_id), COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_correction_ledger_legs
          WHERE account_kind='platform' AND leg_kind='platform_pending_reversal'
            AND account_id=?1 AND currency='CNY' AND direction='debit'
            AND balance_state='pending'",
    )
}

fn released_pending_margin_on(conn: &Connection) -> Result<LedgerAggregate> {
    aggregate_on(
        conn,
        "SELECT COUNT(DISTINCT posting_id), COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind='platform' AND leg_kind='platform_pending_release'
            AND account_id=?1 AND currency='CNY' AND direction='debit'
            AND balance_state='pending'",
    )
}

fn released_available_margin_on(conn: &Connection) -> Result<LedgerAggregate> {
    aggregate_on(
        conn,
        "SELECT COUNT(DISTINCT posting_id), COALESCE(SUM(amount_micros),0)
           FROM compute_settlement_release_ledger_legs
          WHERE account_kind='platform' AND leg_kind='platform_available_credit'
            AND account_id=?1 AND currency='CNY' AND direction='credit'
            AND balance_state='available'",
    )
}

fn aggregate_on(conn: &Connection, sql: &str) -> Result<LedgerAggregate> {
    conn.query_row(sql, params![PLATFORM_ACCOUNT_ID], |row| {
        Ok(LedgerAggregate {
            posting_count: row.get(0)?,
            amount_micros: row.get(1)?,
        })
    })
    .map_err(Into::into)
}

fn projection_digest(view: &ComputePlatformSettlementAccountView) -> Result<String> {
    let mut canonical = view.clone();
    canonical.projection_digest.clear();
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}
