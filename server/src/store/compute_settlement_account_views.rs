use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    compute_provider_registry::current_registered_provider_on,
    compute_settlement_withdrawal_requests::compute_settlement_withdrawal_request_on,
    compute_settlement_withdrawal_terminals::compute_settlement_withdrawal_terminal_on, Store,
};

pub(crate) const COMPUTE_SETTLEMENT_ACCOUNT_VIEW_SCHEMA: &str =
    "compute_federation.settlement_account_view.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementAccountView {
    pub schema: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_account_id: String,
    pub owner_user_id: String,
    pub currency: String,
    pub pending_micros: i64,
    pub available_micros: i64,
    pub disputed_micros: i64,
    pub withdrawn_micros: i64,
    pub account_revision: i64,
    pub updated_at: Option<String>,
    pub withdrawal_request_count: i64,
    pub withdrawal_requested_micros: i64,
    pub pending_terminal_count: i64,
    pub pending_terminal_micros: i64,
    pub cancelled_count: i64,
    pub rejected_count: i64,
    pub external_paid_attested_count: i64,
    pub external_paid_attested_micros: i64,
    pub returned_to_available_micros: i64,
    pub projection_digest: String,
    pub audit_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementWithdrawalQueueItem {
    pub status: String,
    pub request: super::ComputeSettlementWithdrawalRequestReceipt,
    pub terminal: Option<super::ComputeSettlementWithdrawalTerminalReceipt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputeSettlementWithdrawalQueuePage {
    pub schema: String,
    pub status_filter: String,
    pub limit: usize,
    pub items: Vec<ComputeSettlementWithdrawalQueueItem>,
    pub external_transfer_effect: String,
}

impl Store {
    pub(crate) fn compute_settlement_account_view(
        &self,
        provider_id: &str,
    ) -> Result<ComputeSettlementAccountView> {
        validate_exact("Provider ID", provider_id, 160)?;
        let conn = self.conn()?;
        account_view_on(&conn, provider_id)
    }

    pub(crate) fn list_compute_settlement_withdrawal_queue(
        &self,
        status: &str,
        limit: usize,
    ) -> Result<ComputeSettlementWithdrawalQueuePage> {
        let status = normalize_status(status)?;
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;
        let ids = withdrawal_ids_on(&conn, &status, limit)?;
        let mut items = Vec::with_capacity(ids.len());
        for (withdrawal_id, terminal_action) in ids {
            let request = compute_settlement_withdrawal_request_on(&conn, &withdrawal_id)?;
            let terminal = terminal_action
                .as_ref()
                .map(|_| compute_settlement_withdrawal_terminal_on(&conn, &withdrawal_id))
                .transpose()?;
            let derived_status = terminal
                .as_ref()
                .map(|receipt| receipt.action.clone())
                .unwrap_or_else(|| "pending".to_string());
            if terminal_action.as_deref().unwrap_or("pending") != derived_status {
                bail!("提现队列状态与终态回执不一致");
            }
            items.push(ComputeSettlementWithdrawalQueueItem {
                status: derived_status,
                request,
                terminal,
            });
        }
        Ok(ComputeSettlementWithdrawalQueuePage {
            schema: "compute_federation.settlement_withdrawal_queue.v1".to_string(),
            status_filter: status,
            limit,
            items,
            external_transfer_effect: "read_only_no_external_transfer".to_string(),
        })
    }
}

fn account_view_on(conn: &Connection, provider_id: &str) -> Result<ComputeSettlementAccountView> {
    let provider = current_registered_provider_on(conn, provider_id)?
        .ok_or_else(|| anyhow!("算力 Provider 不存在"))?;
    let account_id = provider
        .provider
        .settlement_account_id
        .as_deref()
        .unwrap_or(provider.provider.owner_account_id.as_str());
    let current = current_balance_on(conn, account_id)?;
    let derived = derived_balances_on(conn, account_id)?;
    let lifecycle = withdrawal_lifecycle_on(conn, account_id)?;
    let lifecycle_withdrawn = lifecycle
        .pending_terminal_micros
        .checked_add(lifecycle.external_paid_attested_micros)
        .context("提款生命周期 withdrawn 汇总溢出")?;
    let lifecycle_requested = lifecycle
        .returned_micros
        .checked_add(lifecycle.pending_terminal_micros)
        .and_then(|value| value.checked_add(lifecycle.external_paid_attested_micros))
        .context("提款生命周期申请额汇总溢出")?;
    if current.pending != derived.pending
        || current.available != derived.available
        || current.disputed != 0
        || current.withdrawn != derived.withdrawn
        || derived.withdrawn != lifecycle_withdrawn
        || lifecycle.requested_micros != lifecycle_requested
    {
        bail!("Provider 结算账户与不可变账本或提款生命周期投影不一致");
    }
    let mut view = ComputeSettlementAccountView {
        schema: COMPUTE_SETTLEMENT_ACCOUNT_VIEW_SCHEMA.to_string(),
        provider_id: provider.provider.provider_id,
        provider_policy_revision: provider.provider.policy_revision,
        provider_digest: provider.provider_digest,
        provider_account_id: account_id.to_string(),
        owner_user_id: provider.provider.owner_account_id,
        currency: "CNY".to_string(),
        pending_micros: current.pending,
        available_micros: current.available,
        disputed_micros: current.disputed,
        withdrawn_micros: current.withdrawn,
        account_revision: current.revision,
        updated_at: current.updated_at,
        withdrawal_request_count: lifecycle.request_count,
        withdrawal_requested_micros: lifecycle.requested_micros,
        pending_terminal_count: lifecycle.pending_terminal_count,
        pending_terminal_micros: lifecycle.pending_terminal_micros,
        cancelled_count: lifecycle.cancelled_count,
        rejected_count: lifecycle.rejected_count,
        external_paid_attested_count: lifecycle.external_paid_attested_count,
        external_paid_attested_micros: lifecycle.external_paid_attested_micros,
        returned_to_available_micros: lifecycle.returned_micros,
        projection_digest: String::new(),
        audit_status: "verified_from_append_only_ledgers".to_string(),
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

fn current_balance_on(conn: &Connection, account_id: &str) -> Result<CurrentBalance> {
    Ok(conn
        .query_row(
            "SELECT pending_micros, available_micros, disputed_micros,
                    withdrawn_micros, revision, updated_at
               FROM compute_settlement_account_balances
              WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'",
            params![account_id],
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

struct DerivedBalances {
    pending: i64,
    available: i64,
    withdrawn: i64,
}

fn derived_balances_on(conn: &Connection, account_id: &str) -> Result<DerivedBalances> {
    let pending_credits = sum_on(
        conn,
        "compute_settlement_ledger_legs",
        "balance_state='pending' AND direction='credit' AND leg_kind='provider_pending'",
        account_id,
    )?;
    let pending_corrections = sum_on(
        conn,
        "compute_settlement_correction_ledger_legs",
        "balance_state='pending' AND direction='debit'",
        account_id,
    )?;
    let pending_releases = sum_on(
        conn,
        "compute_settlement_release_ledger_legs",
        "balance_state='pending' AND direction='debit'",
        account_id,
    )?;
    let available_credits = sum_on(
        conn,
        "compute_settlement_release_ledger_legs",
        "balance_state='available' AND direction='credit'",
        account_id,
    )?;
    let withdrawal_reserves = sum_on(
        conn,
        "compute_settlement_withdrawal_request_ledger_legs",
        "balance_state='available' AND direction='debit'",
        account_id,
    )?;
    let withdrawal_returns = sum_on(
        conn,
        "compute_settlement_withdrawal_terminal_ledger_legs",
        "balance_state='available' AND direction='credit'",
        account_id,
    )?;
    let pending = pending_credits
        .checked_sub(pending_corrections)
        .and_then(|value| value.checked_sub(pending_releases))
        .context("pending 账本投影溢出")?;
    let available = available_credits
        .checked_sub(withdrawal_reserves)
        .and_then(|value| value.checked_add(withdrawal_returns))
        .context("available 账本投影溢出")?;
    let withdrawn = withdrawal_reserves
        .checked_sub(withdrawal_returns)
        .context("withdrawn 账本投影溢出")?;
    Ok(DerivedBalances {
        pending,
        available,
        withdrawn,
    })
}

fn sum_on(conn: &Connection, table: &str, predicate: &str, account_id: &str) -> Result<i64> {
    conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(amount_micros),0) FROM {table}
              WHERE account_kind='provider' AND account_id=?1 AND currency='CNY'
                AND {predicate}"
        ),
        params![account_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

#[derive(Default)]
struct WithdrawalLifecycle {
    request_count: i64,
    requested_micros: i64,
    pending_terminal_count: i64,
    pending_terminal_micros: i64,
    cancelled_count: i64,
    rejected_count: i64,
    external_paid_attested_count: i64,
    external_paid_attested_micros: i64,
    returned_micros: i64,
}

fn withdrawal_lifecycle_on(conn: &Connection, account_id: &str) -> Result<WithdrawalLifecycle> {
    conn.query_row(
        "SELECT COUNT(r.withdrawal_id), COALESCE(SUM(r.amount_micros),0),
                COALESCE(SUM(CASE WHEN t.withdrawal_id IS NULL THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.withdrawal_id IS NULL THEN r.amount_micros ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.action='cancelled' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.action='rejected' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.action='external_paid_attested' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN t.action='external_paid_attested' THEN r.amount_micros ELSE 0 END),0),
                COALESCE(SUM(t.balance_returned_micros),0)
           FROM compute_settlement_withdrawal_requests r
           LEFT JOIN compute_settlement_withdrawal_terminals t
             ON t.withdrawal_id=r.withdrawal_id
          WHERE r.provider_account_id=?1",
        params![account_id],
        |row| {
            Ok(WithdrawalLifecycle {
                request_count: row.get(0)?,
                requested_micros: row.get(1)?,
                pending_terminal_count: row.get(2)?,
                pending_terminal_micros: row.get(3)?,
                cancelled_count: row.get(4)?,
                rejected_count: row.get(5)?,
                external_paid_attested_count: row.get(6)?,
                external_paid_attested_micros: row.get(7)?,
                returned_micros: row.get(8)?,
            })
        },
    )
    .map_err(Into::into)
}

fn withdrawal_ids_on(
    conn: &Connection,
    status: &str,
    limit: usize,
) -> Result<Vec<(String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT r.withdrawal_id, t.action
           FROM compute_settlement_withdrawal_requests r
           LEFT JOIN compute_settlement_withdrawal_terminals t
             ON t.withdrawal_id=r.withdrawal_id
          WHERE ?1='all'
             OR (?1='pending' AND t.withdrawal_id IS NULL)
             OR t.action=?1
          ORDER BY r.requested_at DESC, r.withdrawal_id DESC
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![status, limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn projection_digest(view: &ComputeSettlementAccountView) -> Result<String> {
    let mut canonical = view.clone();
    canonical.projection_digest.clear();
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?)))
}

fn normalize_status(status: &str) -> Result<String> {
    let status = status.trim().to_ascii_lowercase();
    if !matches!(
        status.as_str(),
        "all" | "pending" | "cancelled" | "rejected" | "external_paid_attested"
    ) {
        bail!("提现队列状态筛选值不受支持");
    }
    Ok(status)
}

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("{label}不能为空或包含首尾空白");
    }
    if value.chars().count() > max_len {
        bail!("{label}长度不能超过 {max_len}");
    }
    Ok(())
}
