// server/src/store/admin_stats_quotas.rs
//! admin 趋势、审计和配额管理

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::admin_stats::{AdminAccountingAuditRow, AdminTrendRow, UserQuota};
use super::{now, Store};

impl Store {
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
        let rows = stmt
            .query_map(params![since, limit], |row| {
                Ok(AdminTrendRow {
                    date: row.get(0)?,
                    total_tokens: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    call_count: row.get(4)?,
                    active_users: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 管理员对账审计：按记账状态、用户、功能和来源汇总可信用量。
    pub fn admin_accounting_audit(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<AdminAccountingAuditRow>> {
        let conn = self.conn()?;
        let since = format!("-{} days", days);
        let mut stmt = conn.prepare(
            "SELECT COALESCE(t.accounting_status, 'legacy') AS accounting_status,
                    t.user_id,
                    u.phone, u.email, u.nickname,
                    t.feature,
                    t.usage_mode,
                    COALESCE(SUM(t.total_tokens),0) AS total_tokens,
                    COUNT(*) AS call_count,
                    COALESCE(SUM(t.cost_rmb_fen),0) AS billed_cost_rmb_fen,
                    MAX(t.created_at) AS last_call_at
             FROM token_usage_events t
             LEFT JOIN users u ON u.id = t.user_id
             WHERE t.usage_mode != 'client_reported'
               AND t.created_at >= datetime('now', ?1)
             GROUP BY accounting_status, t.user_id, t.feature, t.usage_mode
             ORDER BY
               CASE accounting_status
                 WHEN 'billed' THEN 3
                 WHEN 'zero_cost' THEN 4
                 ELSE 1
               END ASC,
               total_tokens DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![since, limit], |row| {
                let accounting_status: String = row.get(0)?;
                let user_id: String = row.get(1)?;
                let phone: Option<String> = row.get(2)?;
                let email: Option<String> = row.get(3)?;
                let nickname: Option<String> = row.get(4)?;
                Ok(AdminAccountingAuditRow {
                    accounting_status,
                    account: phone.or(email).unwrap_or_else(|| user_id.clone()),
                    user_id,
                    nickname,
                    feature: row.get(5)?,
                    usage_mode: row.get(6)?,
                    total_tokens: row.get(7)?,
                    call_count: row.get(8)?,
                    billed_cost_rmb_fen: row.get(9)?,
                    last_call_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
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
        let rows = stmt
            .query_map([], |row| {
                let uid: String = row.get(0)?;
                let phone: Option<String> = row.get(1)?;
                let email: Option<String> = row.get(2)?;
                let nickname: Option<String> = row.get(3)?;
                let limit: Option<i64> = row.get(4)?;
                let blocked: i64 = row.get(5)?;
                let reason: Option<String> = row.get(6)?;
                let created: String = row.get(7)?;
                let updated: String = row.get(8)?;
                Ok((
                    uid, phone, email, nickname, limit, blocked, reason, created, updated,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut result = Vec::with_capacity(rows.len());
        for (uid, phone, email, nickname, limit, blocked, reason, created, updated) in rows {
            let account = phone.or(email);
            let month_tokens: i64 = conn.query_row(
                "SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events
                 WHERE user_id=?1
                   AND usage_mode NOT IN ('client_reported','user_api_key_proxy')
                   AND created_at >= ?2",
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
        let row: Option<(i64, Option<i64>)> = conn
            .query_row(
                "SELECT is_blocked, monthly_token_limit FROM user_token_quota WHERE user_id = ?1",
                params![user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        if let Some((blocked, limit)) = row {
            if blocked != 0 {
                return Err(anyhow::anyhow!("用户已被封禁，无法使用 AI 功能"));
            }
            if let Some(max_tokens) = limit {
                let month_start = chrono::Utc::now().format("%Y-%m-01T00:00:00Z").to_string();
                let used: i64 = conn.query_row(
                    "SELECT COALESCE(SUM(total_tokens),0) FROM token_usage_events
                     WHERE user_id=?1
                       AND usage_mode NOT IN ('client_reported','user_api_key_proxy')
                       AND COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') != 'own_codex'
                       AND created_at >= ?2",
                    params![user_id, month_start],
                    |r| r.get(0),
                )?;
                if used >= max_tokens {
                    return Err(anyhow::anyhow!(
                        "本月 token 用量已达上限（已用 {}，限额 {}）",
                        used,
                        max_tokens
                    ));
                }
            }
        }
        Ok(())
    }
}
