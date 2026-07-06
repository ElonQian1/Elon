//! User progression ledger aggregation.
//!
//! The level system is intentionally read-model only: it derives experience from
//! the existing trusted token ledger and node settlement ledger, so every agent
//! that already records usage through the shared accounting path is included.

use anyhow::Result;
use chrono::{Datelike, Duration, Utc};
use rusqlite::params;

use super::Store;

#[derive(Debug, Clone, Default)]
pub struct UserProgressionLedger {
    pub consumed_tokens: i64,
    pub consumed_call_count: i64,
    pub own_codex_tokens: i64,
    pub own_codex_call_count: i64,
    pub shared_codex_tokens: i64,
    pub shared_codex_call_count: i64,
    pub platform_tokens: i64,
    pub platform_call_count: i64,
    pub provided_tokens: i64,
    pub provided_run_count: i64,
    pub provider_earned_fen: i64,
    pub provider_week_start_at: String,
    pub provider_week_end_at: String,
    pub provider_week_tokens: i64,
    pub provider_week_run_count: i64,
    pub provider_week_billed_fen: i64,
    pub provider_week_earned_fen: i64,
}

impl Store {
    pub fn user_progression_ledger(&self, user_id: &str) -> Result<UserProgressionLedger> {
        let conn = self.conn.lock().unwrap();
        let (
            consumed_tokens,
            consumed_call_count,
            own_codex_tokens,
            own_codex_call_count,
            shared_codex_tokens,
            shared_codex_call_count,
            platform_tokens,
            platform_call_count,
        ): (i64, i64, i64, i64, i64, i64, i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(total_tokens), 0),
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'own_codex' THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'own_codex' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'shared_codex' THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'shared_codex' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') NOT IN ('own_codex', 'shared_codex') THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') NOT IN ('own_codex', 'shared_codex') THEN 1 ELSE 0 END), 0)
               FROM token_usage_events
              WHERE user_id = ?1
                AND usage_mode NOT IN ('client_reported', 'user_api_key_proxy')
                AND total_tokens > 0",
            params![user_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )?;

        let (provided_tokens, provided_run_count, provider_earned_fen): (i64, i64, i64) = conn
            .query_row(
                "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0),
                        COUNT(*),
                        COALESCE(SUM(provider_earned_fen), 0)
                   FROM node_transactions
                  WHERE provider_user_id = ?1
                    AND consumer_user_id != provider_user_id
                    AND (prompt_tokens + completion_tokens) > 0",
                params![user_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

        let (week_start, week_end) = current_utc_week_window();
        let (
            provider_week_tokens,
            provider_week_run_count,
            provider_week_billed_fen,
            provider_week_earned_fen,
        ): (i64, i64, i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0),
                    COUNT(*),
                    COALESCE(SUM(billed_cost_rmb_fen), 0),
                    COALESCE(SUM(provider_earned_fen), 0)
               FROM node_transactions
              WHERE provider_user_id = ?1
                AND consumer_user_id != provider_user_id
                AND (prompt_tokens + completion_tokens) > 0
                AND created_at >= ?2
                AND created_at < ?3",
            params![user_id, week_start, week_end],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        Ok(UserProgressionLedger {
            consumed_tokens: consumed_tokens.max(0),
            consumed_call_count: consumed_call_count.max(0),
            own_codex_tokens: own_codex_tokens.max(0),
            own_codex_call_count: own_codex_call_count.max(0),
            shared_codex_tokens: shared_codex_tokens.max(0),
            shared_codex_call_count: shared_codex_call_count.max(0),
            platform_tokens: platform_tokens.max(0),
            platform_call_count: platform_call_count.max(0),
            provided_tokens: provided_tokens.max(0),
            provided_run_count: provided_run_count.max(0),
            provider_earned_fen: provider_earned_fen.max(0),
            provider_week_start_at: week_start,
            provider_week_end_at: week_end,
            provider_week_tokens: provider_week_tokens.max(0),
            provider_week_run_count: provider_week_run_count.max(0),
            provider_week_billed_fen: provider_week_billed_fen.max(0),
            provider_week_earned_fen: provider_week_earned_fen.max(0),
        })
    }
}

fn current_utc_week_window() -> (String, String) {
    let now = Utc::now();
    let start_date = now.date_naive() - Duration::days(now.weekday().num_days_from_monday() as i64);
    let start_naive = start_date
        .and_hms_opt(0, 0, 0)
        .expect("midnight should always be valid");
    let start = start_naive.and_utc();
    let end = start + Duration::days(7);
    (start.to_rfc3339(), end.to_rfc3339())
}


#[cfg(test)]
#[path = "user_progression_tests.rs"]
mod tests;
