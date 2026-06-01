//! 管理员视角的 token 用量统计与配额管理（跨用户聚合查询）。

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{now, Store};

// ── 查询结果结构 ──────────────────────────────────────────────────────────────

/// 全平台统计摘要
#[derive(Debug, Serialize)]
pub struct AdminPlatformSummary {
    pub total_tokens_all_time: i64,
    pub total_tokens_today: i64,
    pub total_tokens_period: i64,
    pub period_days: i64,
    pub active_users_period: i64,
    pub call_count_period: i64,
    pub estimated_cost_cny: f64,
}

/// 单用户在指定周期内的统计行（用于排行榜）
#[derive(Debug, Serialize)]
pub struct AdminUserUsageRow {
    pub user_id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub call_count: i64,
    pub estimated_cost_cny: f64,
    pub last_call_at: Option<String>,
    /// 是否被封禁
    pub is_blocked: bool,
    /// 月度 token 限额（None 表示无限制）
    pub monthly_token_limit: Option<i64>,
    /// 当月已用 token
    pub current_month_tokens: i64,
}

/// 单用户详情：含模型分布、功能分布、每日趋势
#[derive(Debug, Serialize)]
pub struct AdminUserDetail {
    pub user_id: String,
    pub period_days: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub call_count: i64,
    pub estimated_cost_cny: f64,
    pub by_model: Vec<AdminModelRow>,
    pub by_feature: Vec<AdminFeatureRow>,
    pub by_day: Vec<AdminDayRow>,
}

#[derive(Debug, Serialize)]
pub struct AdminModelRow {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub call_count: i64,
    pub estimated_cost_cny: f64,
}

#[derive(Debug, Serialize)]
pub struct AdminFeatureRow {
    pub feature: String,
    pub total_tokens: i64,
    pub call_count: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminDayRow {
    pub date: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
}

/// 全平台每日趋势行
#[derive(Debug, Serialize)]
pub struct AdminTrendRow {
    pub date: String,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub call_count: i64,
    pub active_users: i64,
}

/// 用户配额记录
#[derive(Debug, Serialize)]
pub struct UserQuota {
    pub user_id: String,
    pub account: Option<String>,
    pub nickname: Option<String>,
    pub monthly_token_limit: Option<i64>,
    pub is_blocked: bool,
    pub block_reason: Option<String>,
    pub current_month_tokens: i64,
    pub created_at: String,
    pub updated_at: String,
}

// ── 模型定价表（USD / 1M tokens）────────────────────────────────────────────

struct ModelPrice {
    input_per_m: f64,
    output_per_m: f64,
}

const CNY_PER_USD: f64 = 7.25;

fn model_price(model: &str) -> ModelPrice {
    let m = model.to_lowercase();
    if m.contains("gpt-4o-mini") || m.contains("gpt4o-mini") {
        ModelPrice { input_per_m: 0.15, output_per_m: 0.60 }
    } else if m.contains("gpt-4o") || m.contains("gpt4o") {
        ModelPrice { input_per_m: 2.5, output_per_m: 10.0 }
    } else if m.contains("o3-mini") {
        ModelPrice { input_per_m: 1.1, output_per_m: 4.4 }
    } else if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") {
        ModelPrice { input_per_m: 0.25, output_per_m: 1.25 }
    } else if m.contains("claude-3-haiku") {
        ModelPrice { input_per_m: 0.25, output_per_m: 1.25 }
    } else if m.contains("claude-opus-4") || m.contains("claude-opus") {
        ModelPrice { input_per_m: 15.0, output_per_m: 75.0 }
    } else if m.contains("claude-sonnet-4") || m.contains("claude-3-7") || m.contains("claude-3.7") {
        ModelPrice { input_per_m: 3.0, output_per_m: 15.0 }
    } else if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") {
        ModelPrice { input_per_m: 3.0, output_per_m: 15.0 }
    } else if m.contains("claude") {
        // 其他 claude 模型保守估算
        ModelPrice { input_per_m: 3.0, output_per_m: 15.0 }
    } else if m.contains("deepseek") {
        ModelPrice { input_per_m: 0.14, output_per_m: 0.28 }
    } else {
        // 未知模型：保守估算
        ModelPrice { input_per_m: 3.0, output_per_m: 15.0 }
    }
}

pub fn estimate_cost_cny(model: Option<&str>, input_tokens: i64, output_tokens: i64) -> f64 {
    let p = model_price(model.unwrap_or(""));
    let usd = (input_tokens as f64 / 1_000_000.0) * p.input_per_m
        + (output_tokens as f64 / 1_000_000.0) * p.output_per_m;
    (usd * CNY_PER_USD * 10000.0).round() / 10000.0
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 全平台统计摘要（管理员用）
    pub fn admin_platform_summary(&self, days: i64) -> Result<AdminPlatformSummary> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let today_start = chrono::Utc::now().format("%Y-%m-%dT00:00:00Z").to_string();

        let (total_all_time, total_today, total_period, active_users, call_count): (i64,i64,i64,i64,i64) = conn.query_row(
            "SELECT
               (SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events WHERE usage_mode != 'client_reported'),
               (SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events WHERE usage_mode != 'client_reported' AND created_at >= ?1),
               (SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events WHERE usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)),
               (SELECT COUNT(DISTINCT user_id) FROM token_usage_events WHERE usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)),
               (SELECT COUNT(*) FROM token_usage_events WHERE usage_mode != 'client_reported' AND created_at >= datetime('now', ?2))",
            params![today_start, since],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;

        // 估算费用：按模型聚合，分别估算
        let cost = self.admin_estimate_period_cost_cny(days)?;

        Ok(AdminPlatformSummary {
            total_tokens_all_time: total_all_time,
            total_tokens_today: total_today,
            total_tokens_period: total_period,
            period_days: days,
            active_users_period: active_users,
            call_count_period: call_count,
            estimated_cost_cny: cost,
        })
    }

    /// 按模型聚合估算指定周期内的总费用（CNY）
    fn admin_estimate_period_cost_cny(&self, days: i64) -> Result<f64> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let mut stmt = conn.prepare(
            "SELECT model, COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0)
             FROM token_usage_events
             WHERE usage_mode != 'client_reported' AND created_at >= datetime('now', ?1)
             GROUP BY model",
        )?;
        let total: f64 = stmt
            .query_map(params![since], |row| {
                let model: Option<String> = row.get(0)?;
                let input: i64 = row.get(1)?;
                let output: i64 = row.get(2)?;
                Ok((model, input, output))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(model, input, output)| estimate_cost_cny(model.as_deref(), input, output))
            .sum();
        Ok((total * 10000.0).round() / 10000.0)
    }

    /// 用户排行榜（按指定周期内总 token 排序）
    pub fn admin_user_usage_list(&self, days: i64, limit: i64) -> Result<Vec<AdminUserUsageRow>> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let month_start = chrono::Utc::now().format("%Y-%m-01T00:00:00Z").to_string();

        // 先拿各用户在周期内的模型级别用量
        let mut stmt = conn.prepare(
            "SELECT t.user_id,
                    u.phone, u.email, u.nickname,
                    COALESCE(SUM(t.input_tokens),0),
                    COALESCE(SUM(t.output_tokens),0),
                    COALESCE(SUM(t.total_tokens),0),
                    COUNT(*),
                    MAX(t.created_at),
                    q.is_blocked,
                    q.monthly_token_limit
             FROM token_usage_events t
             LEFT JOIN users u ON u.id = t.user_id
             LEFT JOIN user_token_quota q ON q.user_id = t.user_id
             WHERE t.usage_mode != 'client_reported'
               AND t.created_at >= datetime('now', ?1)
             GROUP BY t.user_id
             ORDER BY SUM(t.total_tokens) DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since, limit], |row| {
            let uid: String = row.get(0)?;
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            let nickname: Option<String> = row.get(3)?;
            let input: i64 = row.get(4)?;
            let output: i64 = row.get(5)?;
            let total: i64 = row.get(6)?;
            let count: i64 = row.get(7)?;
            let last: Option<String> = row.get(8)?;
            let blocked: Option<i64> = row.get(9)?;
            let limit: Option<i64> = row.get(10)?;
            let account = phone.or(email).unwrap_or_else(|| uid.clone());
            Ok((uid, account, nickname, input, output, total, count, last, blocked, limit))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        // 对每个用户，按模型估算费用，并查当月用量
        let mut result = Vec::with_capacity(rows.len());
        for (uid, account, nickname, input, output, total, count, last, blocked, quota_limit) in rows {
            // 费用：需要按模型拆分，这里用聚合近似：用 token_usage_events 里的 model 字段
            let cost = self.admin_user_cost_in_period(&conn, &uid, &since)?;
            let month_tokens: i64 = conn.query_row(
                "SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events
                 WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= ?2",
                params![uid, month_start],
                |r| r.get(0),
            )?;
            result.push(AdminUserUsageRow {
                user_id: uid,
                account,
                nickname,
                input_tokens: input,
                output_tokens: output,
                total_tokens: total,
                call_count: count,
                estimated_cost_cny: cost,
                last_call_at: last,
                is_blocked: blocked.unwrap_or(0) != 0,
                monthly_token_limit: quota_limit,
                current_month_tokens: month_tokens,
            });
        }
        Ok(result)
    }

    fn admin_user_cost_in_period(&self, conn: &rusqlite::Connection, user_id: &str, since: &str) -> Result<f64> {
        let mut stmt = conn.prepare(
            "SELECT model, COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0)
             FROM token_usage_events
             WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)
             GROUP BY model",
        )?;
        let total: f64 = stmt
            .query_map(params![user_id, since], |row| {
                let model: Option<String> = row.get(0)?;
                let input: i64 = row.get(1)?;
                let output: i64 = row.get(2)?;
                Ok((model, input, output))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(model, input, output)| estimate_cost_cny(model.as_deref(), input, output))
            .sum();
        Ok((total * 10000.0).round() / 10000.0)
    }

    /// 单用户详情：按模型/功能/每日分布
    pub fn admin_user_detail(&self, user_id: &str, days: i64) -> Result<AdminUserDetail> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);

        let (input, output, total, count): (i64,i64,i64,i64) = conn.query_row(
            "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0),
                    COALESCE(SUM(total_tokens),0), COUNT(*)
             FROM token_usage_events
             WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)",
            params![user_id, since],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;

        let cost = self.admin_user_cost_in_period(&conn, user_id, &since)?;

        // 按模型
        let mut model_stmt = conn.prepare(
            "SELECT COALESCE(model,'unknown'),
                    COALESCE(SUM(input_tokens),0),
                    COALESCE(SUM(output_tokens),0),
                    COALESCE(SUM(total_tokens),0),
                    COUNT(*)
             FROM token_usage_events
             WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)
             GROUP BY model ORDER BY SUM(total_tokens) DESC",
        )?;
        let by_model = model_stmt.query_map(params![user_id, since], |row| {
            let model: String = row.get(0)?;
            let input: i64 = row.get(1)?;
            let output: i64 = row.get(2)?;
            let total: i64 = row.get(3)?;
            let count: i64 = row.get(4)?;
            Ok((model, input, output, total, count))
        })?.collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(|(model, input, output, total, count)| {
            let cost = estimate_cost_cny(Some(&model), input, output);
            AdminModelRow { model, input_tokens: input, output_tokens: output, total_tokens: total, call_count: count, estimated_cost_cny: cost }
        })
        .collect();

        // 按功能
        let mut feat_stmt = conn.prepare(
            "SELECT feature, COALESCE(SUM(total_tokens),0), COUNT(*)
             FROM token_usage_events
             WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)
             GROUP BY feature ORDER BY SUM(total_tokens) DESC",
        )?;
        let by_feature = feat_stmt.query_map(params![user_id, since], |row| {
            Ok(AdminFeatureRow {
                feature: row.get(0)?,
                total_tokens: row.get(1)?,
                call_count: row.get(2)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        // 按天（最近 days 天，最多 90 天）
        let limit = days.min(90);
        let mut day_stmt = conn.prepare(
            "SELECT date(created_at),
                    COALESCE(SUM(total_tokens),0),
                    COALESCE(SUM(input_tokens),0),
                    COALESCE(SUM(output_tokens),0),
                    COUNT(*)
             FROM token_usage_events
             WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= datetime('now', ?2)
             GROUP BY date(created_at) ORDER BY 1 DESC LIMIT ?3",
        )?;
        let by_day = day_stmt.query_map(params![user_id, since, limit], |row| {
            Ok(AdminDayRow {
                date: row.get(0)?,
                total_tokens: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                call_count: row.get(4)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(AdminUserDetail {
            user_id: user_id.to_string(),
            period_days: days,
            input_tokens: input,
            output_tokens: output,
            total_tokens: total,
            call_count: count,
            estimated_cost_cny: cost,
            by_model,
            by_feature,
            by_day,
        })
    }

    /// 全平台每日趋势
    pub fn admin_platform_trend(&self, days: i64) -> Result<Vec<AdminTrendRow>> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let limit = days.min(90);
        let mut stmt = conn.prepare(
            "SELECT date(created_at),
                    COALESCE(SUM(total_tokens),0),
                    COALESCE(SUM(input_tokens),0),
                    COALESCE(SUM(output_tokens),0),
                    COUNT(*),
                    COUNT(DISTINCT user_id)
             FROM token_usage_events
             WHERE usage_mode != 'client_reported' AND created_at >= datetime('now', ?1)
             GROUP BY date(created_at)
             ORDER BY 1 DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since, limit], |row| {
            Ok(AdminTrendRow {
                date: row.get(0)?,
                total_tokens: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                call_count: row.get(4)?,
                active_users: row.get(5)?,
            })
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ── 配额管理 ──────────────────────────────────────────────────────────────

    /// 列出所有有配额设置的用户
    pub fn admin_list_quotas(&self) -> Result<Vec<UserQuota>> {
        let conn = self.conn()?;
        let month_start = chrono::Utc::now().format("%Y-%m-01T00:00:00Z").to_string();
        let mut stmt = conn.prepare(
            "SELECT q.user_id, u.phone, u.email, u.nickname,
                    q.monthly_token_limit, q.is_blocked, q.block_reason,
                    q.created_at, q.updated_at
             FROM user_token_quota q
             LEFT JOIN users u ON u.id = q.user_id
             ORDER BY q.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let uid: String = row.get(0)?;
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            let nickname: Option<String> = row.get(3)?;
            let limit: Option<i64> = row.get(4)?;
            let blocked: i64 = row.get(5)?;
            let reason: Option<String> = row.get(6)?;
            let created: String = row.get(7)?;
            let updated: String = row.get(8)?;
            Ok((uid, phone, email, nickname, limit, blocked, reason, created, updated))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        let mut result = Vec::with_capacity(rows.len());
        for (uid, phone, email, nickname, limit, blocked, reason, created, updated) in rows {
            let account = phone.or(email);
            let month_tokens: i64 = conn.query_row(
                "SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events
                 WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= ?2",
                params![uid, month_start],
                |r| r.get(0),
            )?;
            result.push(UserQuota {
                user_id: uid,
                account,
                nickname,
                monthly_token_limit: limit,
                is_blocked: blocked != 0,
                block_reason: reason,
                current_month_tokens: month_tokens,
                created_at: created,
                updated_at: updated,
            });
        }
        Ok(result)
    }

    /// 设置或更新用户配额
    pub fn admin_upsert_quota(
        &self,
        user_id: &str,
        monthly_token_limit: Option<i64>,
        is_blocked: bool,
        block_reason: Option<&str>,
    ) -> Result<()> {
        let ts = now();
        self.conn()?.execute(
            "INSERT INTO user_token_quota (user_id, monthly_token_limit, is_blocked, block_reason, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(user_id) DO UPDATE SET
               monthly_token_limit = excluded.monthly_token_limit,
               is_blocked = excluded.is_blocked,
               block_reason = excluded.block_reason,
               updated_at = excluded.updated_at",
            params![user_id, monthly_token_limit, is_blocked as i64, block_reason, ts],
        )?;
        Ok(())
    }

    /// 删除用户配额（恢复无限制）
    pub fn admin_delete_quota(&self, user_id: &str) -> Result<()> {
        self.conn()?.execute(
            "DELETE FROM user_token_quota WHERE user_id = ?1",
            params![user_id],
        )?;
        Ok(())
    }

    /// 检查用户是否超出配额。返回 Ok(()) 表示允许；返回 Err 表示超限/被封。
    /// 供 token_usage_api 在记录前调用。
    pub fn check_user_quota(&self, user_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let row: Option<(i64, Option<i64>)> = conn.query_row(
            "SELECT is_blocked, monthly_token_limit FROM user_token_quota WHERE user_id = ?1",
            params![user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;

        if let Some((blocked, limit)) = row {
            if blocked != 0 {
                return Err(anyhow::anyhow!("用户已被封禁，无法使用 AI 功能"));
            }
            if let Some(max_tokens) = limit {
                let month_start = chrono::Utc::now().format("%Y-%m-01T00:00:00Z").to_string();
                let used: i64 = conn.query_row(
                    "SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events
                     WHERE user_id=?1 AND usage_mode != 'client_reported' AND created_at >= ?2",
                    params![user_id, month_start],
                    |r| r.get(0),
                )?;
                if used >= max_tokens {
                    return Err(anyhow::anyhow!(
                        "本月 token 用量已达上限（已用 {}，限额 {}）",
                        used, max_tokens
                    ));
                }
            }
        }
        Ok(())
    }
}
