//! 预存计费系统 — 数据库层。
//!
//! 全部操作通过 Store（Mutex<Connection>）同步执行。
//! 金额单位统一用"分"（i64），避免浮点误差。

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{new_id, now, Store};

// ── 对外公开类型 ──────────────────────────────────────────────────────────────

/// 单次 LLM 调用的扣费明细。
#[derive(Debug, Clone, Serialize)]
pub struct BillingEvent {
    pub id: String,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub cost_rmb_fen: i64,
    pub created_at: String,
}

/// 管理员视图：用户余额概览（含 account/nickname）。
#[derive(Debug, Clone, Serialize)]
pub struct AdminBalanceRow {
    pub user_id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub balance_fen: i64,
    pub this_month_spent_fen: i64,
    pub last_recharge_at: Option<String>,
}

/// 充值记录。
#[derive(Debug, Clone, Serialize)]
pub struct RechargeRecord {
    pub id: String,
    pub user_id: String,
    pub amount_fen: i64,
    pub method: String,
    pub operator_id: String,
    pub note: Option<String>,
    pub created_at: String,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 查询用户当前余额（分）。
    /// 返回 `None` 表示该用户尚未开通计费，应放行。
    pub fn billing_get_balance(&self, user_id: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare_cached("SELECT balance_fen FROM user_balance WHERE user_id = ?1")?;
        let mut rows = stmt.query(params![user_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// 原子扣费：
    /// 1. 检查余额 >= amount_fen（不足则返回 Err）
    /// 2. 更新 user_balance
    /// 3. 插入 billing_events 明细
    ///
    /// 返回扣费后的新余额。
    pub fn billing_deduct(
        &self,
        user_id: &str,
        amount_fen: i64,
        model: Option<&str>,
        input_tokens: i64,
        cached_input_tokens: i64,
        output_tokens: i64,
        exchange_rate_x10000: i64,
        markup_x1000: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let balance: Option<i64> = tx
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .ok();
        let balance = balance.unwrap_or(0);
        if balance < amount_fen {
            return Err(anyhow::anyhow!(
                "余额不足：当前 {} 分，需要 {} 分",
                balance,
                amount_fen
            ));
        }
        let new_balance = balance - amount_fen;
        let ts = now();
        tx.execute(
            "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
            params![new_balance, ts, user_id],
        )?;
        let event_id = new_id("bev");
        tx.execute(
            r#"INSERT INTO billing_events
               (id, user_id, model, input_tokens, cached_input_tokens, output_tokens,
                cost_rmb_fen, exchange_rate_x10000, markup_x1000, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
            params![
                event_id,
                user_id,
                model,
                input_tokens,
                cached_input_tokens,
                output_tokens,
                amount_fen,
                exchange_rate_x10000,
                markup_x1000,
                ts,
            ],
        )?;
        tx.commit()?;
        Ok(new_balance)
    }

    /// 管理员充值：原子增加余额并写充值记录。
    /// 返回充值后的新余额。
    pub fn billing_recharge(
        &self,
        user_id: &str,
        amount_fen: i64,
        method: &str,
        operator_id: &str,
        note: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let ts = now();
        // 确保 user_balance 行存在
        tx.execute(
            "INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, ?2)",
            params![user_id, ts],
        )?;
        tx.execute(
            "UPDATE user_balance SET balance_fen = balance_fen + ?1, updated_at = ?2 WHERE user_id = ?3",
            params![amount_fen, ts, user_id],
        )?;
        let new_balance: i64 = tx.query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
            params![user_id],
            |r| r.get(0),
        )?;
        let record_id = new_id("rch");
        tx.execute(
            r#"INSERT INTO recharge_records
               (id, user_id, amount_fen, method, operator_id, note, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
            params![record_id, user_id, amount_fen, method, operator_id, note, ts],
        )?;
        tx.commit()?;
        Ok(new_balance)
    }

    /// 分页查询用户自己的扣费明细。返回 (事件列表, 总条数)。
    pub fn billing_list_events(
        &self,
        user_id: &str,
        page: i64,
        size: i64,
    ) -> Result<(Vec<BillingEvent>, i64)> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM billing_events WHERE user_id = ?1",
            params![user_id],
            |r| r.get(0),
        )?;
        let offset = (page - 1).max(0) * size;
        let mut stmt = conn.prepare_cached(
            r#"SELECT id, model, input_tokens, cached_input_tokens, output_tokens,
                      cost_rmb_fen, created_at
               FROM billing_events
               WHERE user_id = ?1
               ORDER BY created_at DESC
               LIMIT ?2 OFFSET ?3"#,
        )?;
        let events = stmt
            .query_map(params![user_id, size, offset], |row| {
                Ok(BillingEvent {
                    id: row.get(0)?,
                    model: row.get(1)?,
                    input_tokens: row.get(2)?,
                    cached_input_tokens: row.get(3)?,
                    output_tokens: row.get(4)?,
                    cost_rmb_fen: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok((events, total))
    }

    /// 查询用户本月已消费（分）。
    pub fn billing_get_month_cost(&self, user_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let cost: i64 = conn
            .query_row(
                r#"SELECT COALESCE(SUM(cost_rmb_fen), 0)
                   FROM billing_events
                   WHERE user_id = ?1
                     AND created_at >= strftime('%Y-%m-01', 'now')"#,
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok(cost)
    }

    /// 读取计费配置项。
    pub fn billing_get_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare_cached("SELECT value FROM billing_config WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// 写入/更新计费配置项。
    pub fn billing_set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO billing_config (key, value, updated_at) VALUES (?1,?2,datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params![key, value],
        )?;
        Ok(())
    }

    /// 管理员：分页列出所有已开通计费的用户余额概览。
    pub fn billing_admin_list_balances(
        &self,
        page: i64,
        size: i64,
    ) -> Result<Vec<AdminBalanceRow>> {
        let conn = self.conn.lock().unwrap();
        let offset = (page - 1).max(0) * size;
        let mut stmt = conn.prepare_cached(
            r#"SELECT
                 ub.user_id,
                 COALESCE(u.account, ub.user_id) AS account,
                 u.nickname,
                 ub.balance_fen,
                 COALESCE((
                     SELECT SUM(cost_rmb_fen) FROM billing_events
                     WHERE user_id = ub.user_id
                       AND created_at >= strftime('%Y-%m-01', 'now')
                 ), 0) AS this_month_spent_fen,
                 (SELECT created_at FROM recharge_records
                  WHERE user_id = ub.user_id
                  ORDER BY created_at DESC LIMIT 1) AS last_recharge_at
               FROM user_balance ub
               LEFT JOIN users u ON u.id = ub.user_id
               ORDER BY ub.updated_at DESC
               LIMIT ?1 OFFSET ?2"#,
        )?;
        let rows = stmt
            .query_map(params![size, offset], |row| {
                Ok(AdminBalanceRow {
                    user_id: row.get(0)?,
                    account: row.get(1)?,
                    nickname: row.get(2)?,
                    balance_fen: row.get(3)?,
                    this_month_spent_fen: row.get(4)?,
                    last_recharge_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 管理员：查询单个用户的计费详情（余额 + 最近充值记录）。
    /// 返回 None 表示该用户没有余额记录。
    pub fn billing_admin_get_user(
        &self,
        user_id: &str,
    ) -> Result<Option<(AdminBalanceRow, Vec<RechargeRecord>)>> {
        let conn = self.conn.lock().unwrap();
        let row: Option<AdminBalanceRow> = conn
            .query_row(
                r#"SELECT
                     ub.user_id,
                     COALESCE(u.account, ub.user_id),
                     u.nickname,
                     ub.balance_fen,
                     COALESCE((
                         SELECT SUM(cost_rmb_fen) FROM billing_events
                         WHERE user_id = ub.user_id
                           AND created_at >= strftime('%Y-%m-01', 'now')
                     ), 0),
                     (SELECT created_at FROM recharge_records
                      WHERE user_id = ub.user_id
                      ORDER BY created_at DESC LIMIT 1)
                   FROM user_balance ub
                   LEFT JOIN users u ON u.id = ub.user_id
                   WHERE ub.user_id = ?1"#,
                params![user_id],
                |row| {
                    Ok(AdminBalanceRow {
                        user_id: row.get(0)?,
                        account: row.get(1)?,
                        nickname: row.get(2)?,
                        balance_fen: row.get(3)?,
                        this_month_spent_fen: row.get(4)?,
                        last_recharge_at: row.get(5)?,
                    })
                },
            )
            .ok();
        if row.is_none() {
            return Ok(None);
        }
        let mut stmt = conn.prepare_cached(
            r#"SELECT id, user_id, amount_fen, method, operator_id, note, created_at
               FROM recharge_records
               WHERE user_id = ?1
               ORDER BY created_at DESC
               LIMIT 20"#,
        )?;
        let records = stmt
            .query_map(params![user_id], |r| {
                Ok(RechargeRecord {
                    id: r.get(0)?,
                    user_id: r.get(1)?,
                    amount_fen: r.get(2)?,
                    method: r.get(3)?,
                    operator_id: r.get(4)?,
                    note: r.get(5)?,
                    created_at: r.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(Some((row.unwrap(), records)))
    }

    /// 确保 user_balance 行存在（Admin 开通计费时调用）。
    pub fn billing_ensure_balance_row(&self, user_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, datetime('now'))",
            params![user_id],
        )?;
        Ok(())
    }

    /// 读取当前计费参数（汇率、加价率）。
    /// 返回 (exchange_rate_x10000, markup_x1000)。
    pub fn billing_get_rate_and_markup(&self) -> (i64, i64) {
        let rate = self
            .billing_get_config("usd_to_rmb_rate_x10000")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(73000);
        let markup = self
            .billing_get_config("markup_x1000")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1200);
        (rate, markup)
    }

    // ── 微信支付订单 ──────────────────────────────────────────────────────────

    /// 创建待支付订单记录。
    pub fn pay_order_create(&self, out_trade_no: &str, user_id: &str, amount_fen: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO wechat_pay_orders
             (out_trade_no, user_id, amount_fen, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'pending', datetime('now'), datetime('now'))",
            params![out_trade_no, user_id, amount_fen],
        )?;
        Ok(())
    }

    /// 查询订单（out_trade_no → user_id + amount_fen + status）。
    pub fn pay_order_find(&self, out_trade_no: &str) -> Result<Option<(String, i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let row = conn.query_row(
            "SELECT user_id, amount_fen, status FROM wechat_pay_orders WHERE out_trade_no = ?1",
            params![out_trade_no],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)),
        ).ok();
        Ok(row)
    }

    /// 将订单标记为已支付，同时为用户充值。
    /// 幂等：status 已是 'paid' 时直接返回 Ok，不重复充值。
    pub fn pay_order_complete(
        &self,
        out_trade_no: &str,
        wechat_tx_id: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        // 查订单
        let row: Option<(String, i64, String)> = tx.query_row(
            "SELECT user_id, amount_fen, status FROM wechat_pay_orders WHERE out_trade_no = ?1",
            params![out_trade_no],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).ok();

        let (user_id, amount_fen, status) = match row {
            Some(r) => r,
            None => return Err(anyhow::anyhow!("找不到订单 {out_trade_no}")),
        };

        if status == "paid" {
            // 幂等：已处理过
            return Ok(());
        }

        let ts = now();

        // 1. 更新订单状态
        tx.execute(
            "UPDATE wechat_pay_orders SET status='paid', wechat_tx_id=?1, updated_at=?2 WHERE out_trade_no=?3",
            params![wechat_tx_id, ts, out_trade_no],
        )?;

        // 2. 初始化余额行（如不存在）
        tx.execute(
            "INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, ?2)",
            params![user_id, ts],
        )?;

        // 3. 充值
        tx.execute(
            "UPDATE user_balance SET balance_fen = balance_fen + ?1, updated_at = ?2 WHERE user_id = ?3",
            params![amount_fen, ts, user_id],
        )?;

        // 4. 插入充值记录（method=wechat_pay）
        let rid = new_id();
        tx.execute(
            "INSERT INTO recharge_records (id, user_id, amount_fen, method, operator_id, note, created_at)
             VALUES (?1, ?2, ?3, 'wechat_pay', 'system', ?4, ?5)",
            params![rid, user_id, amount_fen, wechat_tx_id, ts],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 分页查询用户的支付订单（最新在前）。
    pub fn pay_orders_list_user(&self, user_id: &str, size: i64, offset: i64)
        -> Result<Vec<serde_json::Value>>
    {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT out_trade_no, amount_fen, status, wechat_tx_id, created_at, updated_at
             FROM wechat_pay_orders
             WHERE user_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(params![user_id, size, offset], |r| {
                Ok(serde_json::json!({
                    "out_trade_no": r.get::<_,String>(0)?,
                    "amount_fen":   r.get::<_,i64>(1)?,
                    "status":       r.get::<_,String>(2)?,
                    "wechat_tx_id": r.get::<_,Option<String>>(3)?,
                    "created_at":   r.get::<_,String>(4)?,
                    "updated_at":   r.get::<_,String>(5)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}


