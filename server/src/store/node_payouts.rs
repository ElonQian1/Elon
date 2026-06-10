//! 节点收益提现申请：把可用节点余额冻结为待打款申请，并由后台处理。

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, Store};

const STATUS_PENDING: &str = "pending";
const STATUS_CANCELLED: &str = "cancelled";
const STATUS_REJECTED: &str = "rejected";
const STATUS_PAID: &str = "paid";

#[derive(Debug, Clone, Serialize)]
pub struct NodePayoutRequest {
    pub id: String,
    pub provider_user_id: String,
    pub amount_fen: i64,
    pub amount_credits: f64,
    pub payout_method: String,
    pub payout_account: String,
    pub contact: Option<String>,
    pub status: String,
    pub admin_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
}

pub struct CreateNodePayout<'a> {
    pub provider_user_id: &'a str,
    pub amount_fen: i64,
    pub payout_method: &'a str,
    pub payout_account: &'a str,
    pub contact: Option<&'a str>,
}

impl Store {
    pub fn get_pending_node_payout_total(&self, provider_user_id: &str) -> Result<f64> {
        Ok(fen_to_credits(
            self.get_pending_node_payout_total_fen(provider_user_id)?,
        ))
    }

    pub fn get_pending_node_payout_total_fen(&self, provider_user_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let total_fen: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_fen), 0)
             FROM node_payout_requests
             WHERE provider_user_id = ?1 AND status = 'pending'",
            params![provider_user_id],
            |row| row.get(0),
        )?;
        Ok(total_fen.max(0))
    }

    pub fn create_node_payout_request(
        &self,
        input: CreateNodePayout<'_>,
    ) -> Result<NodePayoutRequest> {
        if input.amount_fen <= 0 {
            bail!("提现金额必须大于 0");
        }
        let payout_method = required_text(input.payout_method, "提现方式", 32)?;
        let payout_account = required_text(input.payout_account, "收款账号", 256)?;
        let contact = optional_text(input.contact, 128);
        let amount_credits = fen_to_credits(input.amount_fen);
        let payout_id = new_id("npayout");
        let ts = now();

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE node_balances
             SET available_fen = available_fen - ?2,
                 frozen_fen = frozen_fen + ?2,
                 credits = (available_fen - ?2) / 100.0,
                 updated_at = ?3
             WHERE user_id = ?1 AND available_fen >= ?2",
            params![input.provider_user_id, input.amount_fen, ts],
        )?;
        if changed == 0 {
            bail!("节点余额不足，无法申请提现");
        }
        tx.execute(
            "INSERT INTO node_payout_requests (
               id, provider_user_id, amount_fen, amount_credits,
               payout_method, payout_account, contact, status,
               admin_note, created_at, updated_at, resolved_at, resolved_by
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', NULL, ?8, ?8, NULL, NULL)",
            params![
                payout_id,
                input.provider_user_id,
                input.amount_fen,
                amount_credits,
                payout_method,
                payout_account,
                contact,
                ts
            ],
        )?;
        let payout = select_payout_by_id(&tx, &payout_id)?;
        tx.commit()?;
        Ok(payout)
    }

    pub fn list_node_payout_requests(
        &self,
        provider_user_id: &str,
        limit: i64,
    ) -> Result<Vec<NodePayoutRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider_user_id, amount_fen, amount_credits,
                    payout_method, payout_account, contact, status,
                    admin_note, created_at, updated_at, resolved_at, resolved_by
             FROM node_payout_requests
             WHERE provider_user_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![provider_user_id, limit.clamp(1, 200)], read_payout)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn cancel_node_payout_request(
        &self,
        provider_user_id: &str,
        payout_id: &str,
    ) -> Result<NodePayoutRequest> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let payout = select_payout_for_provider(&tx, provider_user_id, payout_id)?;
        if payout.status == STATUS_CANCELLED {
            tx.commit()?;
            return Ok(payout);
        }
        if payout.status != STATUS_PENDING {
            bail!("只有待处理的提现申请可以取消");
        }
        refund_frozen_payout(&tx, provider_user_id, payout.amount_fen)?;
        let ts = now();
        tx.execute(
            "UPDATE node_payout_requests
             SET status = 'cancelled', updated_at = ?3, resolved_at = ?3, resolved_by = ?1
             WHERE id = ?2",
            params![provider_user_id, payout_id, ts],
        )?;
        let updated = select_payout_by_id(&tx, payout_id)?;
        tx.commit()?;
        Ok(updated)
    }

    pub fn admin_list_node_payout_requests(
        &self,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<NodePayoutRequest>> {
        let status = status.and_then(normalize_status_filter);
        let conn = self.conn.lock().unwrap();
        if let Some(status) = status {
            let mut stmt = conn.prepare(
                "SELECT id, provider_user_id, amount_fen, amount_credits,
                        payout_method, payout_account, contact, status,
                        admin_note, created_at, updated_at, resolved_at, resolved_by
                 FROM node_payout_requests
                 WHERE status = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![status, limit.clamp(1, 500)], read_payout)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, provider_user_id, amount_fen, amount_credits,
                        payout_method, payout_account, contact, status,
                        admin_note, created_at, updated_at, resolved_at, resolved_by
                 FROM node_payout_requests
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit.clamp(1, 500)], read_payout)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        }
    }

    pub fn admin_mark_node_payout_paid(
        &self,
        payout_id: &str,
        operator_id: &str,
        admin_note: Option<&str>,
    ) -> Result<NodePayoutRequest> {
        self.admin_resolve_node_payout(payout_id, STATUS_PAID, operator_id, admin_note)
    }

    pub fn admin_reject_node_payout(
        &self,
        payout_id: &str,
        operator_id: &str,
        admin_note: Option<&str>,
    ) -> Result<NodePayoutRequest> {
        self.admin_resolve_node_payout(payout_id, STATUS_REJECTED, operator_id, admin_note)
    }

    fn admin_resolve_node_payout(
        &self,
        payout_id: &str,
        target_status: &str,
        operator_id: &str,
        admin_note: Option<&str>,
    ) -> Result<NodePayoutRequest> {
        let operator_id = required_text(operator_id, "操作员", 64)?;
        let admin_note = optional_text(admin_note, 512);
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let payout = select_payout_by_id(&tx, payout_id)?;
        if payout.status == target_status {
            tx.commit()?;
            return Ok(payout);
        }
        if payout.status != STATUS_PENDING {
            bail!("只有待处理的提现申请可以处理");
        }
        if target_status == STATUS_REJECTED {
            refund_frozen_payout(&tx, &payout.provider_user_id, payout.amount_fen)?;
        } else if target_status == STATUS_PAID {
            mark_frozen_payout_paid(&tx, &payout.provider_user_id, payout.amount_fen)?;
        }
        let ts = now();
        tx.execute(
            "UPDATE node_payout_requests
             SET status = ?2,
                 admin_note = ?3,
                 updated_at = ?4,
                 resolved_at = ?4,
                 resolved_by = ?5
             WHERE id = ?1",
            params![payout_id, target_status, admin_note, ts, operator_id],
        )?;
        let updated = select_payout_by_id(&tx, payout_id)?;
        tx.commit()?;
        Ok(updated)
    }
}

fn refund_frozen_payout(
    tx: &rusqlite::Transaction<'_>,
    provider_user_id: &str,
    amount_fen: i64,
) -> Result<()> {
    if amount_fen <= 0 {
        return Ok(());
    }
    let ts = now();
    tx.execute(
        "INSERT INTO node_balances (user_id, credits, available_fen, frozen_fen, updated_at)
         VALUES (?1, ?2, ?3, 0, ?4)
         ON CONFLICT(user_id) DO UPDATE SET
           available_fen = node_balances.available_fen + excluded.available_fen,
           frozen_fen = CASE
             WHEN node_balances.frozen_fen >= excluded.available_fen
             THEN node_balances.frozen_fen - excluded.available_fen
             ELSE 0
           END,
           credits = (node_balances.available_fen + excluded.available_fen) / 100.0,
           updated_at = excluded.updated_at",
        params![provider_user_id, fen_to_credits(amount_fen), amount_fen, ts],
    )?;
    Ok(())
}

fn mark_frozen_payout_paid(
    tx: &rusqlite::Transaction<'_>,
    provider_user_id: &str,
    amount_fen: i64,
) -> Result<()> {
    if amount_fen <= 0 {
        return Ok(());
    }
    tx.execute(
        "UPDATE node_balances
         SET frozen_fen = CASE
               WHEN frozen_fen >= ?2 THEN frozen_fen - ?2
               ELSE 0
             END,
             paid_fen = paid_fen + ?2,
             updated_at = ?3
         WHERE user_id = ?1",
        params![provider_user_id, amount_fen, now()],
    )?;
    Ok(())
}

fn select_payout_for_provider(
    tx: &rusqlite::Transaction<'_>,
    provider_user_id: &str,
    payout_id: &str,
) -> Result<NodePayoutRequest> {
    tx.query_row(
        "SELECT id, provider_user_id, amount_fen, amount_credits,
                payout_method, payout_account, contact, status,
                admin_note, created_at, updated_at, resolved_at, resolved_by
         FROM node_payout_requests
         WHERE id = ?1 AND provider_user_id = ?2",
        params![payout_id, provider_user_id],
        read_payout,
    )
    .optional()?
    .ok_or_else(|| anyhow!("提现申请不存在"))
}

fn select_payout_by_id(
    tx: &rusqlite::Transaction<'_>,
    payout_id: &str,
) -> Result<NodePayoutRequest> {
    tx.query_row(
        "SELECT id, provider_user_id, amount_fen, amount_credits,
                payout_method, payout_account, contact, status,
                admin_note, created_at, updated_at, resolved_at, resolved_by
         FROM node_payout_requests
         WHERE id = ?1",
        params![payout_id],
        read_payout,
    )
    .optional()?
    .ok_or_else(|| anyhow!("提现申请不存在"))
}

fn read_payout(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodePayoutRequest> {
    Ok(NodePayoutRequest {
        id: row.get(0)?,
        provider_user_id: row.get(1)?,
        amount_fen: row.get(2)?,
        amount_credits: row.get(3)?,
        payout_method: row.get(4)?,
        payout_account: row.get(5)?,
        contact: row.get(6)?,
        status: row.get(7)?,
        admin_note: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        resolved_at: row.get(11)?,
        resolved_by: row.get(12)?,
    })
}

fn required_text(value: &str, field: &str, max_len: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field}不能为空");
    }
    Ok(value.chars().take(max_len).collect())
}

fn optional_text(value: Option<&str>, max_len: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_len).collect())
}

fn normalize_status_filter(value: &str) -> Option<&str> {
    match value.trim() {
        "" | "all" => None,
        STATUS_PENDING => Some(STATUS_PENDING),
        STATUS_CANCELLED => Some(STATUS_CANCELLED),
        STATUS_REJECTED => Some(STATUS_REJECTED),
        STATUS_PAID => Some(STATUS_PAID),
        _ => None,
    }
}

fn fen_to_credits(fen: i64) -> f64 {
    fen.max(0) as f64 / 100.0
}
