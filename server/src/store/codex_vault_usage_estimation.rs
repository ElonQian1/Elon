use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{common::new_id, now, Store};

#[derive(Debug, Clone, Deserialize)]
pub struct CodexVaultUsageSnapshotWrite {
    pub provider_user_id: String,
    pub observed_by_user_id: String,
    pub lease_id: Option<String>,
    pub account_hint_hash: Option<String>,
    pub source: Option<String>,
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<String>,
    pub rate_limit_reached_type: Option<String>,
    pub credits_balance: Option<String>,
    pub lifetime_tokens: Option<i64>,
    pub daily_bucket_date: Option<String>,
    pub daily_tokens: Option<i64>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultUsageSnapshotRecord {
    pub id: String,
    pub provider_user_id: String,
    pub observed_by_user_id: String,
    pub lease_id: Option<String>,
    pub account_hint_hash: Option<String>,
    pub source: String,
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<String>,
    pub rate_limit_reached_type: Option<String>,
    pub credits_balance: Option<String>,
    pub lifetime_tokens: Option<i64>,
    pub daily_bucket_date: Option<String>,
    pub daily_tokens: Option<i64>,
    pub observed_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultUsageEstimateReport {
    pub provider_user_id: String,
    pub limit_id: String,
    pub days: i64,
    pub monthly_usd_cents: i64,
    pub windows: Vec<CodexVaultUsageEstimateWindow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultUsageEstimateWindow {
    pub limit_id: String,
    pub resets_at: Option<String>,
    pub window_duration_mins: Option<i64>,
    pub first_snapshot_at: String,
    pub last_snapshot_at: String,
    pub first_used_percent: Option<f64>,
    pub last_used_percent: Option<f64>,
    pub first_remaining_percent: Option<f64>,
    pub last_remaining_percent: Option<f64>,
    pub consumed_percent: f64,
    pub official_token_delta: Option<i64>,
    pub shared_token_total: i64,
    pub denominator_tokens: Option<i64>,
    pub confidence: String,
    pub estimated_window_cost_usd_cents: i64,
    pub allocations: Vec<CodexVaultUsageAllocation>,
    pub unattributed_percent: f64,
    pub unattributed_cost_usd_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultUsageAllocation {
    pub consumer_user_id: String,
    pub consumer_account: String,
    pub consumer_nickname: Option<String>,
    pub consumer_node_id: String,
    pub lease_id: String,
    pub tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub token_share: f64,
    pub estimated_percent: f64,
    pub estimated_cost_usd_cents: i64,
    pub billed_cost_rmb_fen: i64,
    pub provider_earned_fen: i64,
    pub event_count: i64,
}

impl Store {
    pub fn record_codex_vault_usage_snapshot(
        &self,
        write: &CodexVaultUsageSnapshotWrite,
    ) -> Result<CodexVaultUsageSnapshotRecord> {
        let id = new_id("cvus");
        let created = now();
        let observed_at = write.observed_at.clone().unwrap_or_else(|| created.clone());
        let source = clean_label(write.source.as_deref(), "codex_app_server");
        let limit_id = clean_label(Some(&write.limit_id), "codex");
        self.conn()?.execute(
            "INSERT INTO codex_vault_usage_snapshots (
               id, provider_user_id, observed_by_user_id, lease_id, account_hint_hash, source,
               limit_id, limit_name, plan_type, used_percent, remaining_percent,
               window_duration_mins, resets_at, rate_limit_reached_type, credits_balance,
               lifetime_tokens, daily_bucket_date, daily_tokens, observed_at, created_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                id,
                write.provider_user_id.trim(),
                write.observed_by_user_id.trim(),
                clean_optional(write.lease_id.as_deref()),
                clean_optional(write.account_hint_hash.as_deref()),
                source,
                limit_id,
                clean_optional(write.limit_name.as_deref()),
                clean_optional(write.plan_type.as_deref()),
                write.used_percent.map(clamp_percent),
                write.remaining_percent.map(clamp_percent),
                write.window_duration_mins.map(|v| v.max(0)),
                clean_optional(write.resets_at.as_deref()),
                clean_optional(write.rate_limit_reached_type.as_deref()),
                clean_optional(write.credits_balance.as_deref()),
                write.lifetime_tokens.map(|v| v.max(0)),
                clean_optional(write.daily_bucket_date.as_deref()),
                write.daily_tokens.map(|v| v.max(0)),
                observed_at,
                created,
            ],
        )?;
        self.get_codex_vault_usage_snapshot(&id)?
            .ok_or_else(|| anyhow::anyhow!("用量快照保存后无法读取"))
    }

    pub fn get_codex_vault_usage_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<CodexVaultUsageSnapshotRecord>> {
        self.conn()?
            .query_row(
                snapshot_select_sql("WHERE id = ?1").as_str(),
                params![id],
                read_snapshot,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn codex_vault_usage_estimate_report(
        &self,
        provider_user_id: &str,
        days: i64,
        limit_id: &str,
        monthly_usd_cents: i64,
    ) -> Result<CodexVaultUsageEstimateReport> {
        let days = days.clamp(1, 365);
        let since = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let clean_limit_id = clean_label(Some(limit_id), "codex");
        let snapshots =
            self.codex_vault_usage_snapshots_since(provider_user_id, &clean_limit_id, &since)?;
        let mut windows = Vec::new();
        let mut start = 0;
        while start < snapshots.len() {
            let key = snapshots[start].resets_at.clone();
            let mut end = start + 1;
            while end < snapshots.len() && snapshots[end].resets_at == key {
                end += 1;
            }
            if end - start >= 2 {
                windows.push(self.estimate_window(
                    &snapshots[start],
                    &snapshots[end - 1],
                    monthly_usd_cents.max(0),
                )?);
            }
            start = end;
        }
        Ok(CodexVaultUsageEstimateReport {
            provider_user_id: provider_user_id.to_string(),
            limit_id: clean_limit_id,
            days,
            monthly_usd_cents: monthly_usd_cents.max(0),
            windows,
        })
    }

    fn codex_vault_usage_snapshots_since(
        &self,
        provider_user_id: &str,
        limit_id: &str,
        since: &str,
    ) -> Result<Vec<CodexVaultUsageSnapshotRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            snapshot_select_sql(
                "WHERE provider_user_id = ?1
                   AND limit_id = ?2
                   AND observed_at >= ?3
                 ORDER BY COALESCE(resets_at, ''), observed_at, id",
            )
            .as_str(),
        )?;
        let rows = stmt.query_map(params![provider_user_id, limit_id, since], read_snapshot)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn estimate_window(
        &self,
        first: &CodexVaultUsageSnapshotRecord,
        last: &CodexVaultUsageSnapshotRecord,
        monthly_usd_cents: i64,
    ) -> Result<CodexVaultUsageEstimateWindow> {
        let consumed_percent = consumed_percent(first, last);
        let official_delta = match (first.lifetime_tokens, last.lifetime_tokens) {
            (Some(a), Some(b)) if b > a => Some(b - a),
            _ => None,
        };
        let allocations = self.shared_codex_allocations_between(
            &first.provider_user_id,
            &first.observed_at,
            &last.observed_at,
        )?;
        let shared_total = allocations.iter().map(|a| a.tokens).sum::<i64>().max(0);
        let denominator = if let Some(delta) = official_delta {
            Some(delta.max(shared_total).max(1))
        } else if shared_total > 0 {
            Some(shared_total)
        } else {
            None
        };
        let confidence = match (official_delta, shared_total) {
            (Some(delta), total) if total > delta => "official_delta_below_shared_tokens",
            (Some(_), _) => "official_lifetime_calibrated",
            (None, total) if total > 0 => "shared_token_proportional",
            _ => "insufficient_token_data",
        }
        .to_string();
        let window_cost = amortized_window_cost_usd_cents(
            monthly_usd_cents,
            first.window_duration_mins.or(last.window_duration_mins),
        );
        let mut estimated_allocations = Vec::new();
        let mut allocated_percent = 0.0_f64;
        for mut allocation in allocations {
            let share = denominator
                .map(|d| allocation.tokens.max(0) as f64 / d as f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            allocation.token_share = share;
            allocation.estimated_percent = consumed_percent * share;
            allocation.estimated_cost_usd_cents =
                (window_cost as f64 * allocation.estimated_percent / 100.0).round() as i64;
            allocated_percent += allocation.estimated_percent;
            estimated_allocations.push(allocation);
        }
        let unattributed_percent = (consumed_percent - allocated_percent).max(0.0);
        Ok(CodexVaultUsageEstimateWindow {
            limit_id: first.limit_id.clone(),
            resets_at: first.resets_at.clone(),
            window_duration_mins: first.window_duration_mins.or(last.window_duration_mins),
            first_snapshot_at: first.observed_at.clone(),
            last_snapshot_at: last.observed_at.clone(),
            first_used_percent: first.used_percent,
            last_used_percent: last.used_percent,
            first_remaining_percent: first.remaining_percent,
            last_remaining_percent: last.remaining_percent,
            consumed_percent,
            official_token_delta: official_delta,
            shared_token_total: shared_total,
            denominator_tokens: denominator,
            confidence,
            estimated_window_cost_usd_cents: window_cost,
            allocations: estimated_allocations,
            unattributed_percent,
            unattributed_cost_usd_cents: (window_cost as f64 * unattributed_percent / 100.0).round()
                as i64,
        })
    }

    fn shared_codex_allocations_between(
        &self,
        provider_user_id: &str,
        start_at: &str,
        end_at: &str,
    ) -> Result<Vec<CodexVaultUsageAllocation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               l.consumer_user_id,
               COALESCE(c.email, c.phone, c.id),
               c.nickname,
               l.consumer_node_id,
               l.id,
               COALESCE(SUM(u.total_tokens), 0),
               COALESCE(SUM(u.input_tokens), 0),
               COALESCE(SUM(u.output_tokens), 0),
               COALESCE(SUM(u.billed_cost_rmb_fen), 0),
               COALESCE(SUM(u.provider_earned_fen), 0),
               COUNT(*)
             FROM codex_vault_emergency_lease_usage_events u
             JOIN codex_vault_emergency_leases l ON l.id = u.lease_id
             JOIN users c ON c.id = l.consumer_user_id
             WHERE l.provider_user_id = ?1
               AND u.created_at >= ?2
               AND u.created_at <= ?3
             GROUP BY l.consumer_user_id, l.consumer_node_id, l.id
             ORDER BY 6 DESC, 1, 4",
        )?;
        let rows = stmt.query_map(params![provider_user_id, start_at, end_at], |row| {
            Ok(CodexVaultUsageAllocation {
                consumer_user_id: row.get(0)?,
                consumer_account: row.get(1)?,
                consumer_nickname: row.get(2)?,
                consumer_node_id: row.get(3)?,
                lease_id: row.get(4)?,
                tokens: row.get(5)?,
                input_tokens: row.get(6)?,
                output_tokens: row.get(7)?,
                token_share: 0.0,
                estimated_percent: 0.0,
                estimated_cost_usd_cents: 0,
                billed_cost_rmb_fen: row.get(8)?,
                provider_earned_fen: row.get(9)?,
                event_count: row.get(10)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn snapshot_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT id, provider_user_id, observed_by_user_id, lease_id, account_hint_hash,
                source, limit_id, limit_name, plan_type, used_percent, remaining_percent,
                window_duration_mins, resets_at, rate_limit_reached_type, credits_balance,
                lifetime_tokens, daily_bucket_date, daily_tokens, observed_at, created_at
           FROM codex_vault_usage_snapshots
          {where_clause}"
    )
}

fn read_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexVaultUsageSnapshotRecord> {
    Ok(CodexVaultUsageSnapshotRecord {
        id: row.get(0)?,
        provider_user_id: row.get(1)?,
        observed_by_user_id: row.get(2)?,
        lease_id: row.get(3)?,
        account_hint_hash: row.get(4)?,
        source: row.get(5)?,
        limit_id: row.get(6)?,
        limit_name: row.get(7)?,
        plan_type: row.get(8)?,
        used_percent: row.get(9)?,
        remaining_percent: row.get(10)?,
        window_duration_mins: row.get(11)?,
        resets_at: row.get(12)?,
        rate_limit_reached_type: row.get(13)?,
        credits_balance: row.get(14)?,
        lifetime_tokens: row.get(15)?,
        daily_bucket_date: row.get(16)?,
        daily_tokens: row.get(17)?,
        observed_at: row.get(18)?,
        created_at: row.get(19)?,
    })
}

fn consumed_percent(
    first: &CodexVaultUsageSnapshotRecord,
    last: &CodexVaultUsageSnapshotRecord,
) -> f64 {
    if let (Some(a), Some(b)) = (first.used_percent, last.used_percent) {
        return (b - a).max(0.0);
    }
    if let (Some(a), Some(b)) = (first.remaining_percent, last.remaining_percent) {
        return (a - b).max(0.0);
    }
    0.0
}

fn amortized_window_cost_usd_cents(
    monthly_usd_cents: i64,
    window_duration_mins: Option<i64>,
) -> i64 {
    let minutes = window_duration_mins.unwrap_or(300).clamp(1, 31 * 24 * 60);
    (monthly_usd_cents.max(0) as f64 * minutes as f64 / (30.0 * 24.0 * 60.0)).round() as i64
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(200).collect())
}

fn clean_label(value: Option<&str>, fallback: &str) -> String {
    value
        .and_then(|value| clean_optional(Some(value)))
        .unwrap_or_else(|| fallback.to_string())
        .chars()
        .take(80)
        .collect()
}

fn clamp_percent(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{codex_vault_emergency::CodexVaultEmergencyLeaseCreate, Store};

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-codex-usage-estimation-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn estimate_allocates_observed_percent_by_shared_tokens() {
        let (store, path) = temp_store();
        let provider = store
            .create_user(
                "usage-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        let a = store
            .create_user("usage-a@example.com", "secret1", Some("A"), None)
            .unwrap();
        let b = store
            .create_user("usage-b@example.com", "secret1", Some("B"), None)
            .unwrap();
        let grant_a = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &a.id,
                Some("provider to A"),
                Some("test"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let grant_b = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &b.id,
                Some("provider to B"),
                Some("test"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let lease_a = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant_a.id,
                provider_user_id: &provider.id,
                consumer_user_id: &a.id,
                consumer_node_id: "node-a",
                provider_slot_id: "slot",
                account_hint_hash: Some("hint"),
                purpose: Some("patient"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        let lease_b = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant_b.id,
                provider_user_id: &provider.id,
                consumer_user_id: &b.id,
                consumer_node_id: "node-b",
                provider_slot_id: "slot",
                account_hint_hash: Some("hint"),
                purpose: Some("patient"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        store
            .record_codex_vault_usage_snapshot(&CodexVaultUsageSnapshotWrite {
                provider_user_id: provider.id.clone(),
                observed_by_user_id: provider.id.clone(),
                lease_id: None,
                account_hint_hash: Some("hint".to_string()),
                source: None,
                limit_id: "codex".to_string(),
                limit_name: None,
                plan_type: Some("pro".to_string()),
                used_percent: Some(10.0),
                remaining_percent: Some(90.0),
                window_duration_mins: Some(300),
                resets_at: Some("2026-07-06T10:00:00Z".to_string()),
                rate_limit_reached_type: None,
                credits_balance: None,
                lifetime_tokens: Some(1_000_000),
                daily_bucket_date: None,
                daily_tokens: None,
                observed_at: Some("2026-07-06T05:00:00Z".to_string()),
            })
            .unwrap();
        store
            .attach_codex_vault_emergency_usage(
                &lease_a.id,
                Some("tok-a"),
                None,
                None,
                600_000,
                0,
                0,
                0,
                Some("billed"),
            )
            .unwrap();
        store
            .attach_codex_vault_emergency_usage(
                &lease_b.id,
                Some("tok-b"),
                None,
                None,
                800_000,
                0,
                0,
                0,
                Some("billed"),
            )
            .unwrap();
        store
            .conn()
            .unwrap()
            .execute(
                "UPDATE codex_vault_emergency_lease_usage_events
                    SET created_at = CASE lease_id
                        WHEN ?1 THEN '2026-07-06T05:10:00Z'
                        WHEN ?2 THEN '2026-07-06T05:20:00Z'
                        ELSE created_at
                    END
                  WHERE lease_id IN (?1, ?2)",
                rusqlite::params![lease_a.id, lease_b.id],
            )
            .unwrap();
        store
            .record_codex_vault_usage_snapshot(&CodexVaultUsageSnapshotWrite {
                provider_user_id: provider.id.clone(),
                observed_by_user_id: provider.id.clone(),
                lease_id: None,
                account_hint_hash: Some("hint".to_string()),
                source: None,
                limit_id: "codex".to_string(),
                limit_name: None,
                plan_type: Some("pro".to_string()),
                used_percent: Some(30.0),
                remaining_percent: Some(70.0),
                window_duration_mins: Some(300),
                resets_at: Some("2026-07-06T10:00:00Z".to_string()),
                rate_limit_reached_type: None,
                credits_balance: None,
                lifetime_tokens: Some(2_400_000),
                daily_bucket_date: None,
                daily_tokens: None,
                observed_at: Some("2026-07-06T05:30:00Z".to_string()),
            })
            .unwrap();

        let report = store
            .codex_vault_usage_estimate_report(&provider.id, 30, "codex", 20_000)
            .unwrap();
        assert_eq!(report.windows.len(), 1);
        let window = &report.windows[0];
        assert_eq!(window.consumed_percent, 20.0);
        assert_eq!(window.official_token_delta, Some(1_400_000));
        assert_eq!(window.shared_token_total, 1_400_000);
        assert_eq!(window.allocations.len(), 2);
        let total_estimated: f64 = window.allocations.iter().map(|a| a.estimated_percent).sum();
        assert!((total_estimated - 20.0).abs() < 0.001);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
