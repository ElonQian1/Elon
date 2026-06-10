//! 节点收益提现申请：把可用节点余额冻结为待打款申请，并由后台处理。

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, Store};

const STATUS_PENDING: &str = "pending";
const STATUS_CANCELLED: &str = "cancelled";
const STATUS_REJECTED: &str = "rejected";
const STATUS_PAID: &str = "paid";
const BALANCE_EPSILON: f64 = 0.000_001;

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
        let conn = self.conn.lock().unwrap();
        let total: f64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_credits), 0.0)
             FROM node_payout_requests
             WHERE provider_user_id = ?1 AND status = 'pending'",
            params![provider_user_id],
            |row| row.get(0),
        )?;
        Ok(total)
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
        let balance: f64 = tx
            .query_row(
                "SELECT credits FROM node_balances WHERE user_id = ?1",
                params![input.provider_user_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0.0);
        if balance + BALANCE_EPSILON < amount_credits {
            bail!("节点余额不足，无法申请提现");
        }

        tx.execute(
            "UPDATE node_balances
             SET credits = credits - ?2, updated_at = ?3
             WHERE user_id = ?1",
            params![input.provider_user_id, amount_credits, ts],
        )?;
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
        refund_frozen_payout(&tx, provider_user_id, payout.amount_credits)?;
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
            refund_frozen_payout(&tx, &payout.provider_user_id, payout.amount_credits)?;
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
    amount_credits: f64,
) -> Result<()> {
    let ts = now();
    tx.execute(
        "INSERT INTO node_balances (user_id, credits, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id) DO UPDATE SET
           credits = credits + excluded.credits,
           updated_at = excluded.updated_at",
        params![provider_user_id, amount_credits, ts],
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-payout-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    fn add_balance(store: &Store, user_id: &str, credits: f64) {
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO node_balances (user_id, credits, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id) DO UPDATE SET credits = ?2, updated_at = ?3",
            params![user_id, credits, now()],
        )
        .unwrap();
    }

    #[test]
    fn create_payout_freezes_available_balance() {
        let (store, path) = temp_store();
        let user = store
            .create_user("node-payout-freeze@example.com", "secret1", None, None)
            .unwrap();
        add_balance(&store, &user.id, 12.50);

        let payout = store
            .create_node_payout_request(CreateNodePayout {
                provider_user_id: &user.id,
                amount_fen: 500,
                payout_method: "wechat",
                payout_account: "wx-001",
                contact: Some("owner"),
            })
            .unwrap();

        assert_eq!(payout.status, STATUS_PENDING);
        assert_eq!(payout.amount_fen, 500);
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 7.50);
        assert_eq!(store.get_pending_node_payout_total(&user.id).unwrap(), 5.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reject_refunds_once_and_paid_keeps_frozen_balance() {
        let (store, path) = temp_store();
        let user = store
            .create_user("node-payout-resolve@example.com", "secret1", None, None)
            .unwrap();
        add_balance(&store, &user.id, 10.0);

        let rejected = store
            .create_node_payout_request(CreateNodePayout {
                provider_user_id: &user.id,
                amount_fen: 300,
                payout_method: "bank",
                payout_account: "bank-card",
                contact: None,
            })
            .unwrap();
        store
            .admin_reject_node_payout(&rejected.id, "admin", Some("资料不完整"))
            .unwrap();
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 10.0);
        store
            .admin_reject_node_payout(&rejected.id, "admin", Some("资料不完整"))
            .unwrap();
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 10.0);

        let paid = store
            .create_node_payout_request(CreateNodePayout {
                provider_user_id: &user.id,
                amount_fen: 400,
                payout_method: "usdt",
                payout_account: "wallet",
                contact: None,
            })
            .unwrap();
        store
            .admin_mark_node_payout_paid(&paid.id, "admin", Some("txid:1"))
            .unwrap();
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 6.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn insufficient_balance_is_rejected() {
        let (store, path) = temp_store();
        let user = store
            .create_user("node-payout-low@example.com", "secret1", None, None)
            .unwrap();
        add_balance(&store, &user.id, 1.0);

        let err = store
            .create_node_payout_request(CreateNodePayout {
                provider_user_id: &user.id,
                amount_fen: 200,
                payout_method: "wechat",
                payout_account: "wx-001",
                contact: None,
            })
            .unwrap_err();
        assert!(err.to_string().contains("余额不足"));
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 1.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn provider_can_cancel_pending_payout() {
        let (store, path) = temp_store();
        let user = store
            .create_user("node-payout-cancel@example.com", "secret1", None, None)
            .unwrap();
        add_balance(&store, &user.id, 8.0);
        let payout = store
            .create_node_payout_request(CreateNodePayout {
                provider_user_id: &user.id,
                amount_fen: 250,
                payout_method: "alipay",
                payout_account: "ali-001",
                contact: None,
            })
            .unwrap();
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 5.5);

        let cancelled = store
            .cancel_node_payout_request(&user.id, &payout.id)
            .unwrap();
        assert_eq!(cancelled.status, STATUS_CANCELLED);
        assert_eq!(store.get_node_balance(&user.id).unwrap(), 8.0);
        let _ = std::fs::remove_file(path);
    }
}
