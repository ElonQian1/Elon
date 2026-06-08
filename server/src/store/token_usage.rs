//! Token 用量事件的存储与统计。
//!
//! 两个核心方法：
//! - `record_token_usage`  写入一条用量事件（微秒级，不阻塞业务流程）
//! - `get_usage_stats`     按用户返回聚合统计（供 APK 展示用量概览）

use anyhow::Result;
use chrono::Datelike;
use rusqlite::params;
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

// ── 查询结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UsageTotals {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
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
                    COALESCE(SUM(total_tokens),0)
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
