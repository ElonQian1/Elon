use anyhow::Result;
use rusqlite::params;

use crate::open_commerce_app_activity_health_model::{
    OpenCommerceAppActivityHealth, APP_ACTIVITY_STATUS_ATTENTION, APP_ACTIVITY_STATUS_NORMAL,
};

use super::Store;

impl Store {
    pub(crate) fn open_commerce_app_activity_health(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceAppActivityHealth>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT merchant_id, requester_app_id,
                    COUNT(*) AS total_invocations,
                    SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END) AS succeeded_invocations,
                    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed_invocations,
                    SUM(CASE WHEN error_code = 'rate_limited' THEN 1 ELSE 0 END) AS rate_limited_invocations,
                    SUM(CASE WHEN error_code IN ('grant_budget_exceeded', 'grant_budget_rejected') THEN 1 ELSE 0 END) AS grant_budget_rejections,
                    SUM(CASE WHEN error_code IN ('server_restart_interrupted', 'invocation_lease_expired') THEN 1 ELSE 0 END) AS recovered_invocations,
                    MAX(COALESCE(completed_at, created_at)) AS last_invoked_at
               FROM open_commerce_invocations
              WHERE project_id = ?1
                AND requester_app_id NOT IN ('pc-web', 'mcp-client')
                AND julianday(created_at) >= julianday('now', '-24 hours')
              GROUP BY merchant_id, requester_app_id
              ORDER BY last_invoked_at DESC
              LIMIT 500",
        )?;
        let rows = stmt.query_map(params![project_id.trim()], |row| {
            let failed_invocations = row.get::<_, i64>(4)?;
            let rate_limited_invocations = row.get::<_, i64>(5)?;
            let grant_budget_rejections = row.get::<_, i64>(6)?;
            let recovered_invocations = row.get::<_, i64>(7)?;
            let attention_codes = attention_codes(
                failed_invocations,
                rate_limited_invocations,
                grant_budget_rejections,
                recovered_invocations,
            );
            Ok(OpenCommerceAppActivityHealth {
                merchant_id: row.get(0)?,
                requester_app_id: row.get(1)?,
                status: if attention_codes.is_empty() {
                    APP_ACTIVITY_STATUS_NORMAL
                } else {
                    APP_ACTIVITY_STATUS_ATTENTION
                }
                .to_string(),
                total_invocations_24h: row.get(2)?,
                succeeded_invocations_24h: row.get(3)?,
                failed_invocations_24h: failed_invocations,
                rate_limited_invocations_24h: rate_limited_invocations,
                grant_budget_rejections_24h: grant_budget_rejections,
                recovered_invocations_24h: recovered_invocations,
                last_invoked_at: row.get(8)?,
                attention_codes,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn attention_codes(
    failed_invocations: i64,
    rate_limited_invocations: i64,
    grant_budget_rejections: i64,
    recovered_invocations: i64,
) -> Vec<String> {
    let mut codes = Vec::new();
    if recovered_invocations > 0 {
        codes.push("recovered_invocation".to_string());
    }
    if failed_invocations >= 3 {
        codes.push("repeated_failures".to_string());
    }
    if rate_limited_invocations >= 3 {
        codes.push("rate_limit_pressure".to_string());
    }
    if grant_budget_rejections > 0 {
        codes.push("grant_budget_pressure".to_string());
    }
    codes
}
