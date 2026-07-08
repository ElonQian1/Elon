//! 预存计费系统 — 数据库层。
//!
//! 全部操作通过 Store（Mutex<Connection>）同步执行。
//! 金额单位统一用"分"（i64），避免浮点误差。

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, BillingPriceSnapshot, Store};

// ── 对外公开类型 ──────────────────────────────────────────────────────────────

/// 单次 LLM 调用的扣费明细。
#[derive(Debug, Clone, Serialize)]
pub struct BillingEvent {
    pub id: String,
    pub token_usage_event_id: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub cost_rmb_fen: i64,
    pub exchange_rate_x10000: i64,
    pub markup_x1000: i64,
    pub price_rule_id: Option<String>,
    pub price_rule_version: Option<i64>,
    pub price_rule_pattern: Option<String>,
    pub input_usd_per_m: Option<f64>,
    pub cached_usd_per_m: Option<f64>,
    pub output_usd_per_m: Option<f64>,
    pub price_source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminBillingEventRow {
    pub id: String,
    pub user_id: String,
    pub account: Option<String>,
    pub nickname: Option<String>,
    pub token_usage_event_id: Option<String>,
    pub feature: Option<String>,
    pub usage_mode: Option<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub cost_rmb_fen: i64,
    pub exchange_rate_x10000: i64,
    pub markup_x1000: i64,
    pub price_rule_id: Option<String>,
    pub price_rule_version: Option<i64>,
    pub price_rule_pattern: Option<String>,
    pub input_usd_per_m: Option<f64>,
    pub cached_usd_per_m: Option<f64>,
    pub output_usd_per_m: Option<f64>,
    pub price_source: String,
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
        let mut stmt =
            conn.prepare_cached("SELECT balance_fen FROM user_balance WHERE user_id = ?1")?;
        let mut rows = stmt.query(params![user_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// 原子扣费：
    /// 1. 按实际用量扣减余额（允许本次扣成负数，下一次调用前会被余额检查拦截）
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
        price_snapshot: BillingPriceSnapshot,
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
                cost_rmb_fen, exchange_rate_x10000, markup_x1000, created_at,
                price_rule_id, price_rule_version, price_rule_pattern,
                input_usd_per_m, cached_usd_per_m, output_usd_per_m, price_source)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)"#,
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
                price_snapshot.price_rule_id.as_deref(),
                price_snapshot.price_rule_version,
                price_snapshot.price_rule_pattern.as_deref(),
                price_snapshot.input_usd_per_m,
                price_snapshot.cached_usd_per_m,
                price_snapshot.output_usd_per_m,
                price_snapshot.price_source.as_str(),
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
            params![
                record_id,
                user_id,
                amount_fen,
                method,
                operator_id,
                note,
                ts
            ],
        )?;
        tx.commit()?;
        Ok(new_balance)
    }

    /// 幂等赠送余额：同一 user_id + method + operator_id 只赠送一次。
    pub fn billing_grant_once(
        &self,
        user_id: &str,
        amount_fen: i64,
        method: &str,
        operator_id: &str,
        note: Option<&str>,
    ) -> Result<Option<i64>> {
        if amount_fen <= 0 {
            return Ok(None);
        }

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let already_granted = tx
            .query_row(
                "SELECT 1
                 FROM recharge_records
                 WHERE user_id = ?1 AND method = ?2 AND operator_id = ?3
                 LIMIT 1",
                params![user_id, method, operator_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if already_granted {
            tx.commit()?;
            return Ok(None);
        }

        let ts = now();
        tx.execute(
            "INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at)
             VALUES (?1, 0, ?2)",
            params![user_id, ts],
        )?;
        tx.execute(
            "UPDATE user_balance
             SET balance_fen = balance_fen + ?1, updated_at = ?2
             WHERE user_id = ?3",
            params![amount_fen, ts, user_id],
        )?;
        let new_balance: i64 = tx.query_row(
            "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
            params![user_id],
            |r| r.get(0),
        )?;
        tx.execute(
            r#"INSERT INTO recharge_records
               (id, user_id, amount_fen, method, operator_id, note, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
            params![
                new_id("rch"),
                user_id,
                amount_fen,
                method,
                operator_id,
                note,
                ts
            ],
        )?;
        tx.commit()?;
        Ok(Some(new_balance))
    }

    /// 查询用户是否已经获得过指定方式的赠送/充值记录，amount_fen 返回该方式累计金额。
    pub fn billing_find_recharge_by_method(
        &self,
        user_id: &str,
        method: &str,
    ) -> Result<Option<RechargeRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            r#"SELECT
                    (SELECT rr.id FROM recharge_records rr
                      WHERE rr.user_id = recharge_records.user_id AND rr.method = recharge_records.method
                      ORDER BY rr.created_at ASC LIMIT 1) AS id,
                    user_id,
                    COALESCE(SUM(amount_fen), 0) AS amount_fen,
                    method,
                    (SELECT rr.operator_id FROM recharge_records rr
                      WHERE rr.user_id = recharge_records.user_id AND rr.method = recharge_records.method
                      ORDER BY rr.created_at ASC LIMIT 1) AS operator_id,
                    (SELECT rr.note FROM recharge_records rr
                      WHERE rr.user_id = recharge_records.user_id AND rr.method = recharge_records.method
                      ORDER BY rr.created_at ASC LIMIT 1) AS note,
                    MIN(created_at) AS created_at
               FROM recharge_records
               WHERE user_id = ?1 AND method = ?2
               GROUP BY user_id, method"#,
            params![user_id, method],
            |r| {
                Ok(RechargeRecord {
                    id: r.get(0)?,
                    user_id: r.get(1)?,
                    amount_fen: r.get(2)?,
                    method: r.get(3)?,
                    operator_id: r.get(4)?,
                    note: r.get(5)?,
                    created_at: r.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
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
            r#"SELECT id, token_usage_event_id, model, input_tokens, cached_input_tokens, output_tokens,
                      cost_rmb_fen, exchange_rate_x10000, markup_x1000,
                      price_rule_id, price_rule_version, price_rule_pattern,
                      input_usd_per_m, cached_usd_per_m, output_usd_per_m,
                      COALESCE(price_source, 'legacy'), created_at
               FROM billing_events
               WHERE user_id = ?1
               ORDER BY created_at DESC
               LIMIT ?2 OFFSET ?3"#,
        )?;
        let events = stmt
            .query_map(params![user_id, size, offset], |row| {
                Ok(BillingEvent {
                    id: row.get(0)?,
                    token_usage_event_id: row.get(1)?,
                    model: row.get(2)?,
                    input_tokens: row.get(3)?,
                    cached_input_tokens: row.get(4)?,
                    output_tokens: row.get(5)?,
                    cost_rmb_fen: row.get(6)?,
                    exchange_rate_x10000: row.get(7)?,
                    markup_x1000: row.get(8)?,
                    price_rule_id: row.get(9)?,
                    price_rule_version: row.get(10)?,
                    price_rule_pattern: row.get(11)?,
                    input_usd_per_m: row.get(12)?,
                    cached_usd_per_m: row.get(13)?,
                    output_usd_per_m: row.get(14)?,
                    price_source: row.get(15)?,
                    created_at: row.get(16)?,
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
        let mut stmt = conn.prepare_cached("SELECT value FROM billing_config WHERE key = ?1")?;
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

    pub fn admin_billing_events(
        &self,
        user_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AdminBillingEventRow>> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.clamp(1, 500);
        let base_select = r#"SELECT
                 b.id, b.user_id, COALESCE(u.phone, u.email, u.account), u.nickname,
                 b.token_usage_event_id, t.feature, t.usage_mode, b.model,
                 b.input_tokens, b.cached_input_tokens, b.output_tokens, b.cost_rmb_fen,
                 b.exchange_rate_x10000, b.markup_x1000,
                 b.price_rule_id, b.price_rule_version, b.price_rule_pattern,
                 b.input_usd_per_m, b.cached_usd_per_m, b.output_usd_per_m,
                 COALESCE(b.price_source, 'legacy'), b.created_at
               FROM billing_events b
               LEFT JOIN users u ON u.id = b.user_id
               LEFT JOIN token_usage_events t ON t.id = b.token_usage_event_id"#;
        let sql = if user_id.is_some() {
            format!("{base_select} WHERE b.user_id = ?1 ORDER BY b.created_at DESC LIMIT ?2")
        } else {
            format!("{base_select} ORDER BY b.created_at DESC LIMIT ?1")
        };
        let mut stmt = conn.prepare_cached(&sql)?;
        let read_row = |row: &rusqlite::Row<'_>| {
            Ok(AdminBillingEventRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                account: row.get(2)?,
                nickname: row.get(3)?,
                token_usage_event_id: row.get(4)?,
                feature: row.get(5)?,
                usage_mode: row.get(6)?,
                model: row.get(7)?,
                input_tokens: row.get(8)?,
                cached_input_tokens: row.get(9)?,
                output_tokens: row.get(10)?,
                cost_rmb_fen: row.get(11)?,
                exchange_rate_x10000: row.get(12)?,
                markup_x1000: row.get(13)?,
                price_rule_id: row.get(14)?,
                price_rule_version: row.get(15)?,
                price_rule_pattern: row.get(16)?,
                input_usd_per_m: row.get(17)?,
                cached_usd_per_m: row.get(18)?,
                output_usd_per_m: row.get(19)?,
                price_source: row.get(20)?,
                created_at: row.get(21)?,
            })
        };
        let rows = if let Some(user_id) = user_id {
            stmt.query_map(params![user_id, limit], read_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![limit], read_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
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
}
