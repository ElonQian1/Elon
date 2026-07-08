// server/src/store/token_usage_stats.rs
//! get_usage_stats 及辅助函数，从 token_usage.rs 提取。

use anyhow::Result;
use chrono::Datelike;
use rusqlite::params;

use super::token_usage::{
    UsageBillingSourceRow, UsageDayRow, UsageFeatureRow, UsageModeRow, UsageQuota, UsageStats,
    UsageTotals, BILLING_SOURCE_CLIENT_REPORTED, BILLING_SOURCE_OWN_CODEX,
    BILLING_SOURCE_PLATFORM, BILLING_SOURCE_SHARED_CODEX, BILLING_SOURCE_USER_API_KEY,
};
use super::Store;

impl Store {
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
