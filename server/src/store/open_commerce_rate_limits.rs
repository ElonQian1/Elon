use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::open_commerce_rate_limit_model::{
    OpenCommerceRateLimitDecision, OpenCommerceRateLimitPolicy, OpenCommerceRateLimitUsage,
    RATE_LIMIT_STATUS_ACTIVE, RATE_LIMIT_STATUS_DISABLED, RATE_LIMIT_WILDCARD_APP,
};

use super::{new_id, now, Store};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn upsert_open_commerce_rate_limit(
        &self,
        project_id: &str,
        merchant_id: &str,
        capability_id: &str,
        capability_key: &str,
        requester_app_id: Option<&str>,
        window_seconds: i64,
        max_requests: i64,
        enabled: bool,
        actor_user_id: &str,
    ) -> Result<OpenCommerceRateLimitPolicy> {
        let selector = requester_app_id.unwrap_or(RATE_LIMIT_WILDCARD_APP);
        let status = if enabled {
            RATE_LIMIT_STATUS_ACTIVE
        } else {
            RATE_LIMIT_STATUS_DISABLED
        };
        let timestamp = now();
        let id = new_id("ratelimit");
        self.conn()?.execute(
            "INSERT INTO open_commerce_rate_limit_policies (
                id, project_id, merchant_id, capability_id, capability_key,
                requester_app_id, window_seconds, max_requests, status,
                created_by_user_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(capability_id, requester_app_id) DO UPDATE SET
                capability_key = excluded.capability_key,
                window_seconds = excluded.window_seconds,
                max_requests = excluded.max_requests,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                id,
                project_id.trim(),
                merchant_id.trim(),
                capability_id.trim(),
                capability_key.trim(),
                selector,
                window_seconds,
                max_requests,
                status,
                actor_user_id.trim(),
                timestamp
            ],
        )?;
        self.find_open_commerce_rate_limit_by_selector(capability_id, selector)?
            .context("限流策略保存后不存在")
    }

    pub(crate) fn set_open_commerce_rate_limit_enabled(
        &self,
        project_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<OpenCommerceRateLimitPolicy> {
        let status = if enabled {
            RATE_LIMIT_STATUS_ACTIVE
        } else {
            RATE_LIMIT_STATUS_DISABLED
        };
        let updated = self.conn()?.execute(
            "UPDATE open_commerce_rate_limit_policies
                SET status = ?1, updated_at = ?2
              WHERE id = ?3 AND project_id = ?4",
            params![status, now(), policy_id.trim(), project_id.trim()],
        )?;
        if updated == 0 {
            bail!("限流策略不存在");
        }
        self.open_commerce_rate_limit_policy(project_id, policy_id)
    }

    pub(crate) fn list_project_open_commerce_rate_limits(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceRateLimitPolicy>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{POLICY_SELECT} WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let policies = stmt
            .query_map(params![project_id.trim()], policy_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(policies)
    }

    pub(crate) fn list_project_open_commerce_rate_limit_usage(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceRateLimitUsage>> {
        let policies = self.list_project_open_commerce_rate_limits(project_id)?;
        let current_epoch = chrono::Utc::now().timestamp();
        let conn = self.conn()?;
        policies
            .into_iter()
            .map(|policy| {
                let window_started_at = aligned_window(current_epoch, policy.window_seconds);
                let (accepted_requests, active_subjects) = conn.query_row(
                    "SELECT COALESCE(SUM(request_count), 0), COUNT(*)
                       FROM open_commerce_rate_limit_counters
                      WHERE policy_id = ?1 AND window_started_at = ?2",
                    params![policy.id, window_started_at],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                Ok(OpenCommerceRateLimitUsage {
                    policy_id: policy.id,
                    window_started_at_unix: window_started_at,
                    accepted_requests,
                    active_subjects,
                })
            })
            .collect()
    }

    pub(crate) fn claim_open_commerce_rate_limit(
        &self,
        project_id: &str,
        merchant_id: &str,
        capability_id: &str,
        requester_app_id: &str,
        subject_key: &str,
    ) -> Result<Option<OpenCommerceRateLimitDecision>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let policy = tx
            .query_row(
                &format!(
                    "{POLICY_SELECT}
                     WHERE project_id = ?1 AND merchant_id = ?2 AND capability_id = ?3
                       AND status = 'active' AND requester_app_id IN (?4, '*')
                     ORDER BY CASE WHEN requester_app_id = ?4 THEN 0 ELSE 1 END
                     LIMIT 1"
                ),
                params![
                    project_id.trim(),
                    merchant_id.trim(),
                    capability_id.trim(),
                    requester_app_id.trim()
                ],
                policy_from_row,
            )
            .optional()?;
        let Some(policy) = policy else {
            tx.commit()?;
            return Ok(None);
        };
        let current_epoch = chrono::Utc::now().timestamp();
        let window_started_at = aligned_window(current_epoch, policy.window_seconds);
        let updated = tx.execute(
            "INSERT INTO open_commerce_rate_limit_counters (
                policy_id, subject_key, window_started_at, request_count, updated_at
             ) VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(policy_id, subject_key) DO UPDATE SET
                window_started_at = CASE
                    WHEN open_commerce_rate_limit_counters.window_started_at < excluded.window_started_at
                    THEN excluded.window_started_at
                    ELSE open_commerce_rate_limit_counters.window_started_at END,
                request_count = CASE
                    WHEN open_commerce_rate_limit_counters.window_started_at < excluded.window_started_at
                    THEN 1 ELSE open_commerce_rate_limit_counters.request_count + 1 END,
                updated_at = excluded.updated_at
             WHERE open_commerce_rate_limit_counters.window_started_at < excluded.window_started_at
                OR open_commerce_rate_limit_counters.request_count < ?5",
            params![
                policy.id,
                subject_key.trim(),
                window_started_at,
                now(),
                policy.max_requests
            ],
        )?;
        let used_requests = tx.query_row(
            "SELECT request_count FROM open_commerce_rate_limit_counters
              WHERE policy_id = ?1 AND subject_key = ?2",
            params![policy.id, subject_key.trim()],
            |row| row.get::<_, i64>(0),
        )?;
        tx.commit()?;
        let allowed = updated > 0;
        Ok(Some(OpenCommerceRateLimitDecision {
            policy_id: policy.id,
            window_seconds: policy.window_seconds,
            max_requests: policy.max_requests,
            used_requests,
            remaining_requests: (policy.max_requests - used_requests).max(0),
            reset_at_unix: window_started_at + policy.window_seconds,
            allowed,
        }))
    }

    fn open_commerce_rate_limit_policy(
        &self,
        project_id: &str,
        policy_id: &str,
    ) -> Result<OpenCommerceRateLimitPolicy> {
        self.conn()?
            .query_row(
                &format!("{POLICY_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), policy_id.trim()],
                policy_from_row,
            )
            .map_err(|error| anyhow!(error).context("限流策略不存在"))
    }

    fn find_open_commerce_rate_limit_by_selector(
        &self,
        capability_id: &str,
        requester_app_id: &str,
    ) -> Result<Option<OpenCommerceRateLimitPolicy>> {
        self.conn()?
            .query_row(
                &format!("{POLICY_SELECT} WHERE capability_id = ?1 AND requester_app_id = ?2"),
                params![capability_id.trim(), requester_app_id.trim()],
                policy_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn aligned_window(current_epoch: i64, window_seconds: i64) -> i64 {
    current_epoch - current_epoch.rem_euclid(window_seconds)
}

fn policy_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceRateLimitPolicy> {
    let requester_app_id = row.get::<_, String>(5)?;
    Ok(OpenCommerceRateLimitPolicy {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        capability_id: row.get(3)?,
        capability_key: row.get(4)?,
        requester_app_id: (requester_app_id != RATE_LIMIT_WILDCARD_APP).then_some(requester_app_id),
        window_seconds: row.get(6)?,
        max_requests: row.get(7)?,
        status: row.get(8)?,
        created_by_user_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

const POLICY_SELECT: &str = "SELECT id, project_id, merchant_id, capability_id,
            capability_key, requester_app_id, window_seconds, max_requests,
            status, created_by_user_id, created_at, updated_at
       FROM open_commerce_rate_limit_policies";
