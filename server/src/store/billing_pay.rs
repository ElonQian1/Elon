// server/src/store/billing_pay.rs
//! 微信支付订单相关操作，从 billing.rs 提取。

use anyhow::Result;
use rusqlite::params;

use super::{new_id, now, Store};

impl Store {
    // ── 微信支付订单 ──────────────────────────────────────────────────────────

    /// 创建待支付订单记录。
    pub fn pay_order_create(
        &self,
        out_trade_no: &str,
        user_id: &str,
        amount_fen: i64,
    ) -> Result<()> {
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
        let row = conn
            .query_row(
                "SELECT user_id, amount_fen, status FROM wechat_pay_orders WHERE out_trade_no = ?1",
                params![out_trade_no],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .ok();
        Ok(row)
    }

    /// 将订单标记为已支付，同时为用户充值。
    /// 幂等：status 已是 'paid' 时直接返回 Ok，不重复充值。
    pub fn pay_order_complete(&self, out_trade_no: &str, wechat_tx_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        // 查订单
        let row: Option<(String, i64, String)> = tx
            .query_row(
                "SELECT user_id, amount_fen, status FROM wechat_pay_orders WHERE out_trade_no = ?1",
                params![out_trade_no],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();

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
        let rid = new_id("rchrg");
        tx.execute(
            "INSERT INTO recharge_records (id, user_id, amount_fen, method, operator_id, note, created_at)
             VALUES (?1, ?2, ?3, 'wechat_pay', 'system', ?4, ?5)",
            params![rid, user_id, amount_fen, wechat_tx_id, ts],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 分页查询用户的支付订单（最新在前）。
    pub fn pay_orders_list_user(
        &self,
        user_id: &str,
        size: i64,
        offset: i64,
    ) -> Result<Vec<serde_json::Value>> {
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
