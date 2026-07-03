//! Token 用量事件的存储与统计。
//!
//! 两个核心方法：
//! - `record_token_usage`  写入一条用量事件（微秒级，不阻塞业务流程）
//! - `get_usage_stats`     按用户返回聚合统计（供 APK 展示用量概览）

use anyhow::Result;
use chrono::Datelike;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::billing_reservations::{load_reservation_for_settlement, mark_reservation_settled};
use super::{new_id, now, Store};

pub const BILLING_SOURCE_PLATFORM: &str = "platform";
pub const BILLING_SOURCE_OWN_CODEX: &str = "own_codex";
pub const BILLING_SOURCE_SHARED_CODEX: &str = "shared_codex";
pub const BILLING_SOURCE_USER_API_KEY: &str = "user_api_key";
pub const BILLING_SOURCE_CLIENT_REPORTED: &str = "client_reported";

// ── 写入结构 ──────────────────────────────────────────────────────────────────

/// 单次 LLM 调用的 token 用量，用于写入数据库。
pub struct TokenUsageRecord<'a> {
    pub user_id: &'a str,
    /// 功能标识，例如 "chat" | "project_chat" | "codex_cli" | "agent_tool"
    pub feature: &'a str,
    /// 来源模式：
    /// - `server_api_key`   服务器 API Key（强可信）
    /// - `user_api_key_proxy` 用户托管 API Key 由服务器代理调用（审计，不扣平台余额/额度）
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
    /// 资源来源：
    /// - `platform` 平台余额承载
    /// - `own_codex` 用户自己的 Codex CLI 账号，记录 token 但不扣平台余额
    /// - `shared_codex` 使用其他用户分享的 Codex/PC 节点，按平台策略结算
    /// - `user_api_key` 用户自带 API key，不扣平台余额
    /// - `client_reported` 客户端参考上报，不扣平台余额
    pub billing_source: Option<&'a str>,
    /// 承载本次调用的资源属主。自用时通常等于 `user_id`；使用别人节点时是节点 owner。
    pub resource_owner_user_id: Option<&'a str>,
    /// Stable request/trace key for idempotent trusted accounting.
    ///
    /// When present, the same `(user_id, idempotency_key)` is billed at most once.
    pub idempotency_key: Option<&'a str>,
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
    pub price_snapshot: super::BillingPriceSnapshot,
    pub bill_missing_balance: bool,
    pub charge_platform_balance: bool,
}

#[derive(Debug, Clone)]
pub struct TokenUsageAccountingResult {
    pub token_usage_event_id: String,
    pub billing_event_id: Option<String>,
    pub cost_rmb_fen: i64,
    pub balance_after_fen: Option<i64>,
    pub accounting_status: String,
    pub idempotency_key: Option<String>,
    pub deduplicated: bool,
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
pub struct UsageBillingSourceRow {
    pub billing_source: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
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
    /// 按资源来源分组（平台 / 自己 Codex / 别人 Codex / 自带 key / 客户端上报）
    pub by_billing_source: Vec<UsageBillingSourceRow>,
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
        let idempotency_key = normalized_idempotency_key(r.idempotency_key);
        let billing_source = normalized_billing_source(r.billing_source, r.usage_mode);
        let resource_owner_user_id = normalized_optional_user_id(r.resource_owner_user_id);
        self.conn()?.execute(
            "INSERT INTO token_usage_events (
               id, user_id, feature, usage_mode, model,
               input_tokens, cached_input_tokens, output_tokens,
               reasoning_tokens, total_tokens, created_at, idempotency_key,
               billing_source, resource_owner_user_id
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
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
                idempotency_key,
                billing_source,
                resource_owner_user_id,
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
        let idempotency_key = normalized_idempotency_key(r.idempotency_key);
        let billing_source = normalized_billing_source(r.billing_source, r.usage_mode);
        let resource_owner_user_id = normalized_optional_user_id(r.resource_owner_user_id);
        let charge_platform_balance = charge.charge_platform_balance
            && billing_source_charges_platform_balance(&billing_source);
        let mut billing_event_id = None;
        let mut balance_after = None;
        let mut billed_cost = 0;
        let mut accounting_status = "unbilled_no_balance".to_string();

        if let Some(key) = idempotency_key.as_deref() {
            let existing = tx
                .query_row(
                    "SELECT id, billing_event_id, cost_rmb_fen, balance_after_fen, accounting_status
                     FROM token_usage_events
                     WHERE user_id = ?1 AND idempotency_key = ?2",
                    params![r.user_id, key],
                    |row| {
                        Ok(TokenUsageAccountingResult {
                            token_usage_event_id: row.get(0)?,
                            billing_event_id: row.get(1)?,
                            cost_rmb_fen: row.get(2)?,
                            balance_after_fen: row.get(3)?,
                            accounting_status: row.get(4)?,
                            idempotency_key: Some(key.to_string()),
                            deduplicated: true,
                        })
                    },
                )
                .optional()?;
            if let Some(existing) = existing {
                tx.commit()?;
                return Ok(existing);
            }
        }

        if !charge_platform_balance {
            accounting_status = unbilled_accounting_status_for_source(&billing_source).to_string();
            if let Some(key) = idempotency_key.as_deref() {
                if let Some(reservation) = load_reservation_for_settlement(&tx, r.user_id, key)? {
                    let balance_after_reserve = tx
                        .query_row(
                            "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                            params![r.user_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .optional()?
                        .unwrap_or(0);
                    let new_balance = balance_after_reserve + reservation.reserved_fen;
                    tx.execute(
                        "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
                        params![new_balance, created, r.user_id],
                    )?;
                    mark_reservation_settled(
                        &tx,
                        &reservation.id,
                        &token_event_id,
                        None,
                        0,
                        reservation.reserved_fen,
                        &created,
                    )?;
                    balance_after = Some(new_balance);
                }
            }
            tx.execute(
                "INSERT INTO token_usage_events (
                   id, user_id, feature, usage_mode, model,
                   input_tokens, cached_input_tokens, output_tokens,
                   reasoning_tokens, total_tokens, created_at,
                   accounting_status, billing_event_id, cost_rmb_fen, balance_after_fen,
                   idempotency_key, billing_source, resource_owner_user_id
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
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
                    idempotency_key,
                    billing_source,
                    resource_owner_user_id,
                ],
            )?;
            tx.commit()?;
            return Ok(TokenUsageAccountingResult {
                token_usage_event_id: token_event_id,
                billing_event_id,
                cost_rmb_fen: billed_cost,
                balance_after_fen: balance_after,
                accounting_status,
                idempotency_key,
                deduplicated: false,
            });
        }

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

        let reservation = if let Some(key) = idempotency_key.as_deref() {
            load_reservation_for_settlement(&tx, r.user_id, key)?
        } else {
            None
        };

        if let Some(reservation) = reservation {
            let balance_after_reserve = tx
                .query_row(
                    "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                    params![r.user_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .unwrap_or(0);
            if charge.cost_rmb_fen > 0 && (charge.input_tokens > 0 || charge.output_tokens > 0) {
                let delta = charge.cost_rmb_fen - reservation.reserved_fen;
                let new_balance = balance_after_reserve - delta;
                tx.execute(
                    "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
                    params![new_balance, created, r.user_id],
                )?;
                let event_id = new_id("bev");
                tx.execute(
                    r#"INSERT INTO billing_events
                       (id, user_id, model, input_tokens, cached_input_tokens, output_tokens,
                        cost_rmb_fen, exchange_rate_x10000, markup_x1000, created_at,
                        token_usage_event_id, price_rule_id, price_rule_version,
                        price_rule_pattern, input_usd_per_m, cached_usd_per_m,
                        output_usd_per_m, price_source)
                       VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
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
                        charge.price_snapshot.price_rule_id.as_deref(),
                        charge.price_snapshot.price_rule_version,
                        charge.price_snapshot.price_rule_pattern.as_deref(),
                        charge.price_snapshot.input_usd_per_m,
                        charge.price_snapshot.cached_usd_per_m,
                        charge.price_snapshot.output_usd_per_m,
                        charge.price_snapshot.price_source.as_str(),
                    ],
                )?;
                mark_reservation_settled(
                    &tx,
                    &reservation.id,
                    &token_event_id,
                    Some(&event_id),
                    charge.cost_rmb_fen,
                    (reservation.reserved_fen - charge.cost_rmb_fen).max(0),
                    &created,
                )?;
                billing_event_id = Some(event_id);
                balance_after = Some(new_balance);
                billed_cost = charge.cost_rmb_fen;
                accounting_status = "billed".to_string();
            } else {
                let new_balance = balance_after_reserve + reservation.reserved_fen;
                tx.execute(
                    "UPDATE user_balance SET balance_fen = ?1, updated_at = ?2 WHERE user_id = ?3",
                    params![new_balance, created, r.user_id],
                )?;
                mark_reservation_settled(
                    &tx,
                    &reservation.id,
                    &token_event_id,
                    None,
                    0,
                    reservation.reserved_fen,
                    &created,
                )?;
                balance_after = Some(new_balance);
                accounting_status = "zero_cost".to_string();
            }
        } else if let Some(balance) = balance {
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
                        token_usage_event_id, price_rule_id, price_rule_version,
                        price_rule_pattern, input_usd_per_m, cached_usd_per_m,
                        output_usd_per_m, price_source)
                       VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
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
                        charge.price_snapshot.price_rule_id.as_deref(),
                        charge.price_snapshot.price_rule_version,
                        charge.price_snapshot.price_rule_pattern.as_deref(),
                        charge.price_snapshot.input_usd_per_m,
                        charge.price_snapshot.cached_usd_per_m,
                        charge.price_snapshot.output_usd_per_m,
                        charge.price_snapshot.price_source.as_str(),
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
               accounting_status, billing_event_id, cost_rmb_fen, balance_after_fen,
               idempotency_key, billing_source, resource_owner_user_id
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
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
                idempotency_key,
                billing_source,
                resource_owner_user_id,
            ],
        )?;
        tx.commit()?;

        Ok(TokenUsageAccountingResult {
            token_usage_event_id: token_event_id,
            billing_event_id,
            cost_rmb_fen: billed_cost,
            balance_after_fen: balance_after,
            accounting_status,
            idempotency_key,
            deduplicated: false,
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
                    COALESCE(SUM(CASE
                      WHEN usage_mode NOT IN ('client_reported','user_api_key_proxy')
                       AND COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') != 'own_codex'
                      THEN total_tokens ELSE 0 END),0),
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

        // ── 按资源来源 ────────────────────────────────────────────────────
        let by_billing_source = {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(NULLIF(TRIM(billing_source), ''), 'platform'),
                        COALESCE(SUM(total_tokens),0),
                        COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0),
                        COUNT(*)
                 FROM token_usage_events
                 WHERE user_id=?1 AND created_at >= datetime('now', ?2)
                 GROUP BY 1 ORDER BY 2 DESC",
            )?;
            let rows = stmt
                .query_map(params![user_id, &since], |row| {
                    Ok(UsageBillingSourceRow {
                        billing_source: row.get(0)?,
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
                   AND usage_mode NOT IN ('client_reported','user_api_key_proxy')
                   AND COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') != 'own_codex'
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
            by_billing_source,
            by_feature,
            by_day,
            quota,
        })
    }
}

fn normalized_idempotency_key(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(160).collect())
}

fn normalized_optional_user_id(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(80).collect())
}

fn normalized_billing_source(value: Option<&str>, usage_mode: &str) -> String {
    let clean = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    match clean.as_deref() {
        Some(BILLING_SOURCE_OWN_CODEX) => BILLING_SOURCE_OWN_CODEX.to_string(),
        Some(BILLING_SOURCE_SHARED_CODEX) => BILLING_SOURCE_SHARED_CODEX.to_string(),
        Some(BILLING_SOURCE_USER_API_KEY) => BILLING_SOURCE_USER_API_KEY.to_string(),
        Some(BILLING_SOURCE_CLIENT_REPORTED) => BILLING_SOURCE_CLIENT_REPORTED.to_string(),
        Some(BILLING_SOURCE_PLATFORM) => BILLING_SOURCE_PLATFORM.to_string(),
        _ => match usage_mode {
            "client_reported" => BILLING_SOURCE_CLIENT_REPORTED.to_string(),
            "user_api_key_proxy" => BILLING_SOURCE_USER_API_KEY.to_string(),
            _ => BILLING_SOURCE_PLATFORM.to_string(),
        },
    }
}

fn billing_source_charges_platform_balance(source: &str) -> bool {
    matches!(
        source,
        BILLING_SOURCE_PLATFORM | BILLING_SOURCE_SHARED_CODEX
    )
}

fn unbilled_accounting_status_for_source(source: &str) -> &'static str {
    match source {
        BILLING_SOURCE_OWN_CODEX => "unbilled_own_codex",
        BILLING_SOURCE_USER_API_KEY => "unbilled_user_api_key",
        BILLING_SOURCE_CLIENT_REPORTED => "not_billable",
        _ => "unbilled_no_platform_charge",
    }
}
