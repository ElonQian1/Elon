//! Token 用量事件的存储与统计。
//!
//! 两个核心方法：
//! - `record_token_usage`  写入一条用量事件（微秒级，不阻塞业务流程）
//! - `get_usage_stats`     按用户返回聚合统计（供 APK 展示用量概览）

use anyhow::Result;
use chrono::Datelike;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, Store};

// ── 写入结构 ──────────────────────────────────────────────────────────────────

/// 单次 LLM 调用的 token 用量，用于写入数据库。
pub struct TokenUsageRecord<'a> {
    pub user_id: &'a str,
    /// 功能标识，例如 "chat" | "project_chat" | "codex_cli" | "agent_tool"
    pub feature: &'a str,
    /// 来源模式：
    /// - `server_api_key`   服务器 API Key（强可信）
    /// - `server_codex_cli` 服务器 Codex CLI（强可信）
    /// - `pc_agent_cli`     PC 节点 CLI 回传（强可信）
    /// - `server_node_llm`  分布式节点 LLM 结算（强可信）
    /// - `client_reported`  APK 直连上报（仅供参考，不扣余额/额度）
    pub usage_mode: &'a str,
    pub model: Option<&'a str>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
}

/// 可信用量对应的扣费参数。
pub struct TokenUsageBillingCharge<'a> {
    pub model: Option<&'a str>,
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub cost_rmb_fen: i64,
    pub exchange_rate_x10000: i64,
    pub markup_x1000: i64,
    pub bill_missing_balance: bool,
}

#[derive(Debug, Clone)]
pub struct TokenUsageAccountingResult {
    pub token_usage_event_id: String,
    pub billing_event_id: Option<String>,
    pub cost_rmb_fen: i64,
    pub balance_after_fen: Option<i64>,
    pub accounting_status: String,
}

// ── 查询结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UsageTotals {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
    pub billable_tokens: i64,
    pub billed_cost_rmb_fen: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageModeRow {
    pub usage_mode: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageFeatureRow {
    pub feature: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageDayRow {
    pub date: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageQuota {
    pub limit_tokens: Option<i64>,
    pub used_tokens: i64,
    pub remaining_tokens: Option<i64>,
    pub is_blocked: bool,
    pub block_reason: Option<String>,
    pub reset_at: String,
}

/// 汇总统计，直接序列化返回给 APK。
#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub user_id: String,
    pub period_days: i64,
    pub total: UsageTotals,
    /// 按来源模式分组（服务器 key / Codex CLI / 客户端上报）
    pub by_mode: Vec<UsageModeRow>,
    /// 按功能分组（chat / project_chat / codex_cli …）
    pub by_feature: Vec<UsageFeatureRow>,
    /// 按自然日分组，最近 30 天
    pub by_day: Vec<UsageDayRow>,
    /// 当前自然月配额与剩余额度。未配置上限时 `limit_tokens` / `remaining_tokens` 为 null。
    pub quota: UsageQuota,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 写入一条 token 用量记录。调用方无需处理返回错误，失败只记日志。
    pub fn record_token_usage(&self, r: &TokenUsageRecord<'_>) -> Result<()> {
        let id = new_id("tok");
        let created = now();
        self.conn()?.execute(
            "INSERT INTO token_usage_events (
               id, user_id, feature, usage_mode, model,
               input_tokens, cached_input_tokens, output_tokens,
               reasoning_tokens, total_tokens, created_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                id,
                r.user_id,
                r.feature,
                r.usage_mode,
                r.model,
                r.input_tokens,
                r.cached_input_tokens,
                r.output_tokens,
                r.reasoning_tokens,
                r.total_tokens,
                created,
            ],
        )?;
        Ok(())
    }

    /// 原子写入可信 token 用量，并在用户已开通预存计费时同步扣费。
    ///
    /// 事务内完成：
    /// - 插入 `token_usage_events`
    /// - 更新 `user_balance`
    /// - 插入 `billing_events`
    /// - 将 token 事件回填 `billing_event_id` / `cost_rmb_fen` / `balance_after_fen`
    ///
    /// 没有 `user_balance` 行表示未开通预存计费：仍记录可信用量，但不扣 RMB。
    pub fn record_token_usage_with_billing(
        &self,
        r: &TokenUsageRecord<'_>,
        charge: &TokenUsageBillingCharge<'_>,
    ) -> Result<TokenUsageAccountingResult> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let token_event_id = new_id("tok");
        let created = now();
        let mut billing_event_id = None;
        let mut balance_after = None;
        let mut billed_cost = 0;
        let mut accounting_status = "unbilled_no_balance".to_string();

        let mut balance = tx
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                params![r.user_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if balance.is_none() && charge.bill_missing_balance {
            tx.execute(
                "INSERT INTO user_balance (user_id, balance_fen, updated_at) VALUES (?1, 0, ?2)",
                params![r.user_id, created],
            )?;
            balance = Some(0);
        }

        if let Some(balance) = balance {
            if charge.cost_rmb_fen > 0 && (charge.input_tokens > 0 || charge.output_tokens > 0) {
                let new_balance = balance - charge.cost_rmb_fen;
                tx.execute(
                    "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
                    params![new_balance, created, r.user_id],
                )?;
                let event_id = new_id("bev");
                tx.execute(
                    r#"INSERT INTO billing_events
                       (id, user_id, model, input_tokens, cached_input_tokens, output_tokens,
                        cost_rmb_fen, exchange_rate_x10000, markup_x1000, created_at,
                        token_usage_event_id)
                       VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"#,
                    params![
                        event_id,
                        r.user_id,
                        charge.model,
                        charge.input_tokens.max(0),
                        charge.cached_input_tokens.max(0),
                        charge.output_tokens.max(0),
                        charge.cost_rmb_fen,
                        charge.exchange_rate_x10000,
                        charge.markup_x1000,
                        created,
                        token_event_id,
                    ],
                )?;
                billing_event_id = Some(event_id);
                balance_after = Some(new_balance);
                billed_cost = charge.cost_rmb_fen;
                accounting_status = "billed".to_string();
            } else {
                balance_after = Some(balance);
                accounting_status = "zero_cost".to_string();
            }
        }

        tx.execute(
            "INSERT INTO token_usage_events (
               id, user_id, feature, usage_mode, model,
               input_tokens, cached_input_tokens, output_tokens,
               reasoning_tokens, total_tokens, created_at,
               accounting_status, billing_event_id, cost_rmb_fen, balance_after_fen
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                token_event_id,
                r.user_id,
                r.feature,
                r.usage_mode,
                r.model,
                r.input_tokens,
                r.cached_input_tokens,
                r.output_tokens,
                r.reasoning_tokens,
                r.total_tokens,
                created,
                accounting_status,
                billing_event_id,
                billed_cost,
                balance_after,
            ],
        )?;
        tx.commit()?;

        Ok(TokenUsageAccountingResult {
            token_usage_event_id: token_event_id,
            billing_event_id,
            cost_rmb_fen: billed_cost,
            balance_after_fen: balance_after,
            accounting_status,
        })
    }

    /// 返回用户在最近 `days` 天内的 token 用量聚合统计。
    pub fn get_usage_stats(&self, user_id: &str, days: i64) -> Result<UsageStats> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let month_start = chrono::Utc::now().format("%Y-%m-01T00:00:00Z").to_string();

        // ── 总量 ──────────────────────────────────────────────────────────
        let total: UsageTotals = conn.query_row(
            "SELECT COALESCE(SUM(input_tokens),0),
                    COALESCE(SUM(cached_input_tokens),0),
                    COALESCE(SUM(output_tokens),0),
                    COALESCE(SUM(reasoning_tokens),0),
                    COALESCE(SUM(total_tokens),0),
                    COALESCE(SUM(CASE WHEN usage_mode != 'client_reported' THEN total_tokens ELSE 0 END),0),
                    COALESCE(SUM(cost_rmb_fen),0)
             FROM token_usage_events
             WHERE user_id=?1 AND created_at >= datetime('now', ?2)",
            params![user_id, &since],
            |row| {
                Ok(UsageTotals {
                    input_tokens: row.get(0)?,
                    cached_input_tokens: row.get(1)?,
                    output_tokens: row.get(2)?,
                    reasoning_tokens: row.get(3)?,
                    total_tokens: row.get(4)?,
                    billable_tokens: row.get(5)?,
                    billed_cost_rmb_fen: row.get(6)?,
                })
            },
        )?;

        // ── 按模式 ────────────────────────────────────────────────────────
        let by_mode = {
            let mut stmt = conn.prepare(
                "SELECT usage_mode,
                        COALESCE(SUM(total_tokens),0),
                        COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0),
                        COUNT(*)
                 FROM token_usage_events
                 WHERE user_id=?1 AND created_at >= datetime('now', ?2)
                 GROUP BY usage_mode ORDER BY 2 DESC",
            )?;
            let rows = stmt
                .query_map(params![user_id, &since], |row| {
                    Ok(UsageModeRow {
                        usage_mode: row.get(0)?,
                        total_tokens: row.get(1)?,
                        input_tokens: row.get(2)?,
                        output_tokens: row.get(3)?,
                        call_count: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };

        // ── 按功能 ────────────────────────────────────────────────────────
        let by_feature = {
            let mut stmt = conn.prepare(
                "SELECT feature,
                        COALESCE(SUM(total_tokens),0),
                        COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0),
                        COUNT(*)
                 FROM token_usage_events
                 WHERE user_id=?1 AND created_at >= datetime('now', ?2)
                 GROUP BY feature ORDER BY 2 DESC",
            )?;
            let rows = stmt
                .query_map(params![user_id, &since], |row| {
                    Ok(UsageFeatureRow {
                        feature: row.get(0)?,
                        total_tokens: row.get(1)?,
                        input_tokens: row.get(2)?,
                        output_tokens: row.get(3)?,
                        call_count: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };

        // ── 按天 ──────────────────────────────────────────────────────────
        let by_day = {
            let mut stmt = conn.prepare(
                "SELECT date(created_at),
                        COALESCE(SUM(total_tokens),0),
                        COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0),
                        COUNT(*)
                 FROM token_usage_events
                 WHERE user_id=?1 AND created_at >= datetime('now', ?2)
                 GROUP BY date(created_at)
                 ORDER BY 1 DESC
                 LIMIT 30",
            )?;
            let rows = stmt
                .query_map(params![user_id, &since], |row| {
                    Ok(UsageDayRow {
                        date: row.get(0)?,
                        total_tokens: row.get(1)?,
                        input_tokens: row.get(2)?,
                        output_tokens: row.get(3)?,
                        call_count: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };

        let quota = {
            let row = conn
                .query_row(
                    "SELECT monthly_token_limit, is_blocked, block_reason
                     FROM user_token_quota WHERE user_id = ?1",
                    params![user_id],
                    |row| {
                        Ok((
                            row.get::<_, Option<i64>>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .ok();
            let month_used: i64 = conn.query_row(
                "SELECT COALESCE(SUM(total_tokens),0)
                 FROM token_usage_events
                 WHERE user_id=?1
                   AND usage_mode != 'client_reported'
                   AND created_at >= ?2",
                params![user_id, month_start],
                |row| row.get(0),
            )?;
            let (limit_tokens, blocked, block_reason) = row.unwrap_or((None, 0, None));
            let remaining_tokens = limit_tokens.map(|limit| (limit - month_used).max(0));
            let now = chrono::Utc::now();
            let first_next_month = if now.month() == 12 {
                chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
            }
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|dt| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                    .to_rfc3339()
            })
            .unwrap_or_else(|| now.to_rfc3339());
            UsageQuota {
                limit_tokens,
                used_tokens: month_used,
                remaining_tokens,
                is_blocked: blocked != 0,
                block_reason,
                reset_at: first_next_month,
            }
        };

        Ok(UsageStats {
            user_id: user_id.to_string(),
            period_days: days,
            total,
            by_mode,
            by_feature,
            by_day,
            quota,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-token-usage-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn trusted_usage_records_token_event_and_billing_event_atomically() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("billing-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 1_000, "test", "test", None)
            .unwrap();

        let record = TokenUsageRecord {
            user_id: &user.id,
            feature: "test_feature",
            usage_mode: "server_codex_cli",
            model: Some("gpt-4o-mini"),
            input_tokens: 1_000,
            cached_input_tokens: 100,
            output_tokens: 1_000,
            reasoning_tokens: 0,
            total_tokens: 2_000,
        };
        let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

        assert_eq!(result.accounting_status, "billed");
        assert!(result.cost_rmb_fen > 0);
        assert!(result.billing_event_id.is_some());
        assert_eq!(
            store.billing_get_balance(&user.id).unwrap(),
            Some(1_000 - result.cost_rmb_fen)
        );

        let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(
            events[0].token_usage_event_id.as_deref(),
            Some(result.token_usage_event_id.as_str())
        );

        let stats = store.get_usage_stats(&user.id, 30).unwrap();
        assert_eq!(stats.total.total_tokens, 2_000);
        assert_eq!(stats.total.billable_tokens, 2_000);
        assert_eq!(stats.total.billed_cost_rmb_fen, result.cost_rmb_fen);
        let audit = store.admin_accounting_audit(30, 10).unwrap();
        assert!(audit.iter().any(|row| {
            row.user_id == user.id
                && row.accounting_status == "billed"
                && row.billed_cost_rmb_fen == result.cost_rmb_fen
        }));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn trusted_usage_without_balance_row_is_recorded_but_not_billed() {
        let (store, path) = temp_store();
        store
            .billing_set_config("billing_required_for_all_users", "false")
            .unwrap();
        let user = store
            .create_user(
                &format!("unbilled-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        let record = TokenUsageRecord {
            user_id: &user.id,
            feature: "test_feature",
            usage_mode: "server_codex_cli",
            model: Some("gpt-4o-mini"),
            input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 100,
            reasoning_tokens: 0,
            total_tokens: 200,
        };
        let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

        assert_eq!(result.accounting_status, "unbilled_no_balance");
        assert_eq!(result.cost_rmb_fen, 0);
        assert!(result.billing_event_id.is_none());
        let (_events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
        assert_eq!(total, 0);

        let stats = store.get_usage_stats(&user.id, 30).unwrap();
        assert_eq!(stats.total.total_tokens, 200);
        assert_eq!(stats.total.billable_tokens, 200);
        assert_eq!(stats.total.billed_cost_rmb_fen, 0);
        let audit = store.admin_accounting_audit(30, 10).unwrap();
        assert!(audit.iter().any(|row| {
            row.user_id == user.id
                && row.accounting_status == "unbilled_no_balance"
                && row.total_tokens == 200
        }));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn strict_billing_auto_opens_missing_balance_row_and_bills_negative() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("strict-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        let record = TokenUsageRecord {
            user_id: &user.id,
            feature: "test_feature",
            usage_mode: "server_codex_cli",
            model: Some("gpt-4o-mini"),
            input_tokens: 100,
            cached_input_tokens: 0,
            output_tokens: 100,
            reasoning_tokens: 0,
            total_tokens: 200,
        };
        let result = crate::billing::account_trusted_usage(&store, &record).unwrap();

        assert_eq!(result.accounting_status, "billed");
        assert!(result.cost_rmb_fen > 0);
        assert_eq!(
            store.billing_get_balance(&user.id).unwrap(),
            Some(-result.cost_rmb_fen)
        );
        let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(
            events[0].token_usage_event_id.as_deref(),
            Some(result.token_usage_event_id.as_str())
        );

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
