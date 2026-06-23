//! Persistent Route C runtime call budget ledger.
//!
//! This records Route C admission attempts before the provider call starts, so
//! the platform-level daily budget survives server restarts.

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use crate::agent_runtime_error_summary::operational_error_summary;

use super::{new_id, now, Store};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCBudgetDaySummary {
    pub route_day: String,
    pub total_calls: i64,
    pub completed_calls: i64,
    pub success_calls: i64,
    pub failed_calls: i64,
    pub unique_users: i64,
    pub total_tokens: Option<i64>,
    pub first_created_at: Option<String>,
    pub last_created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCBudgetOutcomeSummary {
    pub outcome: String,
    pub total_calls: i64,
    pub completed_calls: i64,
    pub unique_users: i64,
    pub total_tokens: Option<i64>,
    pub first_created_at: Option<String>,
    pub last_created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCBudgetEventRow {
    pub id: String,
    pub user_id: String,
    pub request_fingerprint: String,
    pub route_day: String,
    pub created_at: String,
    pub outcome: String,
    pub completed_at: Option<String>,
    pub model: Option<String>,
    pub total_tokens: Option<i64>,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteCBudgetRecordResult {
    Recorded {
        event_id: String,
        total_used: usize,
        user_used: usize,
    },
    PlatformLimitReached {
        total_used: usize,
        user_used: usize,
    },
    UserLimitReached {
        total_used: usize,
        user_used: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteCBudgetCompletion {
    pub outcome: String,
    pub model: Option<String>,
    pub total_tokens: Option<i64>,
    pub error_summary: Option<String>,
}

impl Store {
    pub(crate) fn route_c_budget_count_for_day(&self, route_day: &str) -> Result<usize> {
        let count: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events WHERE route_day = ?1",
            params![route_day],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as usize)
    }

    pub(crate) fn route_c_budget_count_for_day_and_user(
        &self,
        route_day: &str,
        user_id: &str,
    ) -> Result<usize> {
        let count: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events
              WHERE route_day = ?1 AND user_id = ?2",
            params![route_day, user_id],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as usize)
    }

    pub(crate) fn route_c_budget_try_record_call(
        &self,
        user_id: &str,
        request_fingerprint: &str,
        route_day: &str,
        daily_call_limit: Option<usize>,
        per_user_daily_call_limit: Option<usize>,
    ) -> Result<RouteCBudgetRecordResult> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let current: i64 = tx.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events WHERE route_day = ?1",
            params![route_day],
            |row| row.get(0),
        )?;
        let total_used = current.max(0) as usize;
        let user_current: i64 = tx.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events
              WHERE route_day = ?1 AND user_id = ?2",
            params![route_day, user_id],
            |row| row.get(0),
        )?;
        let user_used = user_current.max(0) as usize;

        if daily_call_limit.is_some_and(|limit| total_used >= limit) {
            tx.commit()?;
            return Ok(RouteCBudgetRecordResult::PlatformLimitReached {
                total_used,
                user_used,
            });
        }
        if per_user_daily_call_limit.is_some_and(|limit| user_used >= limit) {
            tx.commit()?;
            return Ok(RouteCBudgetRecordResult::UserLimitReached {
                total_used,
                user_used,
            });
        }

        let event_id = new_id("rcb");
        tx.execute(
            "INSERT INTO route_c_runtime_budget_events (
               id, user_id, request_fingerprint, route_day, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, user_id, request_fingerprint, route_day, now(),],
        )?;
        tx.commit()?;
        Ok(RouteCBudgetRecordResult::Recorded {
            event_id,
            total_used: total_used + 1,
            user_used: user_used + 1,
        })
    }

    pub(crate) fn route_c_budget_mark_completed(
        &self,
        event_id: &str,
        completion: RouteCBudgetCompletion,
    ) -> Result<()> {
        let event_id = event_id.trim();
        if event_id.is_empty() {
            return Ok(());
        }
        self.conn()?.execute(
            "UPDATE route_c_runtime_budget_events
                SET outcome = ?2,
                    completed_at = ?3,
                    model = ?4,
                    total_tokens = ?5,
                    error_summary = ?6
              WHERE id = ?1",
            params![
                event_id,
                clean_outcome(&completion.outcome),
                now(),
                clean_optional(completion.model, 120),
                completion.total_tokens.filter(|value| *value >= 0),
                clean_error_summary(completion.error_summary),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn route_c_budget_day_summaries(
        &self,
        days: i64,
    ) -> Result<Vec<RouteCBudgetDaySummary>> {
        let limit = days.clamp(1, 90);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT route_day,
                    COUNT(*) AS total_calls,
                    SUM(CASE WHEN completed_at IS NOT NULL THEN 1 ELSE 0 END) AS completed_calls,
                    SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) AS success_calls,
                    SUM(CASE WHEN outcome != 'success' AND completed_at IS NOT NULL THEN 1 ELSE 0 END) AS failed_calls,
                    COUNT(DISTINCT user_id) AS unique_users,
                    SUM(total_tokens) AS total_tokens,
                    MIN(created_at) AS first_created_at,
                    MAX(created_at) AS last_created_at
               FROM route_c_runtime_budget_events
              GROUP BY route_day
              ORDER BY route_day DESC
              LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(RouteCBudgetDaySummary {
                    route_day: row.get(0)?,
                    total_calls: row.get(1)?,
                    completed_calls: row.get(2)?,
                    success_calls: row.get(3)?,
                    failed_calls: row.get(4)?,
                    unique_users: row.get(5)?,
                    total_tokens: row.get(6)?,
                    first_created_at: row.get(7)?,
                    last_created_at: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub(crate) fn route_c_budget_outcome_summaries(
        &self,
        route_day: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<Vec<RouteCBudgetOutcomeSummary>> {
        let route_day = route_day.map(str::trim).filter(|value| !value.is_empty());
        let user_id = user_id.map(str::trim).filter(|value| !value.is_empty());
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT outcome,
                    COUNT(*) AS total_calls,
                    SUM(CASE WHEN completed_at IS NOT NULL THEN 1 ELSE 0 END) AS completed_calls,
                    COUNT(DISTINCT user_id) AS unique_users,
                    SUM(total_tokens) AS total_tokens,
                    MIN(created_at) AS first_created_at,
                    MAX(created_at) AS last_created_at
               FROM route_c_runtime_budget_events
              WHERE (?1 IS NULL OR route_day = ?1)
                AND (?2 IS NULL OR user_id = ?2)
              GROUP BY outcome
              ORDER BY total_calls DESC, outcome ASC",
        )?;
        let rows = stmt
            .query_map(params![route_day, user_id], |row| {
                Ok(RouteCBudgetOutcomeSummary {
                    outcome: row.get(0)?,
                    total_calls: row.get(1)?,
                    completed_calls: row.get(2)?,
                    unique_users: row.get(3)?,
                    total_tokens: row.get(4)?,
                    first_created_at: row.get(5)?,
                    last_created_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub(crate) fn route_c_budget_recent_events(
        &self,
        route_day: Option<&str>,
        user_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RouteCBudgetEventRow>> {
        let limit = limit.clamp(1, 500);
        let route_day = route_day.map(str::trim).filter(|value| !value.is_empty());
        let user_id = user_id.map(str::trim).filter(|value| !value.is_empty());
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, request_fingerprint, route_day, created_at,
                    outcome, completed_at, model, total_tokens, error_summary
               FROM route_c_runtime_budget_events
              WHERE (?1 IS NULL OR route_day = ?1)
                AND (?2 IS NULL OR user_id = ?2)
              ORDER BY created_at DESC, id DESC
              LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(params![route_day, user_id, limit], |row| {
                Ok(RouteCBudgetEventRow {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    request_fingerprint: row.get(2)?,
                    route_day: row.get(3)?,
                    created_at: row.get(4)?,
                    outcome: row.get(5)?,
                    completed_at: row.get(6)?,
                    model: row.get(7)?,
                    total_tokens: row.get(8)?,
                    error_summary: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn clean_outcome(outcome: &str) -> String {
    match outcome.trim() {
        "success" => "success".to_string(),
        "provider_error" => "provider_error".to_string(),
        "output_rejected" => "output_rejected".to_string(),
        "server_error" => "server_error".to_string(),
        "canceled" => "canceled".to_string(),
        "admitted" => "admitted".to_string(),
        _ => "server_error".to_string(),
    }
}

fn clean_optional(value: Option<String>, max_chars: usize) -> Option<String> {
    value
        .map(|value| value.trim().chars().take(max_chars).collect::<String>())
        .filter(|value| !value.is_empty())
}

fn clean_error_summary(value: Option<String>) -> Option<String> {
    let value = value?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    if is_safe_error_code(&value) || is_operational_error_summary(&value) {
        return Some(value.chars().take(500).collect());
    }
    Some(operational_error_summary(&value))
}

fn is_safe_error_code(value: &str) -> bool {
    value.len() <= 120
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
}

fn is_operational_error_summary(value: &str) -> bool {
    value.len() <= 220
        && value.starts_with("category=")
        && value.contains("fingerprint=")
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '=' | ',' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_route_c_budget_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    fn recorded_event_id(
        result: RouteCBudgetRecordResult,
        total_used: usize,
        user_used: usize,
    ) -> String {
        match result {
            RouteCBudgetRecordResult::Recorded {
                event_id,
                total_used: actual_total,
                user_used: actual_user,
            } => {
                assert_eq!(actual_total, total_used);
                assert_eq!(actual_user, user_used);
                assert!(!event_id.trim().is_empty());
                event_id
            }
            other => panic!("expected recorded budget event, got {other:?}"),
        }
    }

    #[test]
    fn route_c_budget_records_and_blocks_platform_daily_limit() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("route-c-budget-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(2), None)
                .unwrap(),
            1,
            1,
        );
        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-2", "2026-06-23", Some(2), None)
                .unwrap(),
            2,
            2,
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-3", "2026-06-23", Some(2), None)
                .unwrap(),
            RouteCBudgetRecordResult::PlatformLimitReached {
                total_used: 2,
                user_used: 2
            }
        );
        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_blocks_per_user_daily_limit_without_blocking_other_users() {
        let (store, path) = temp_store();
        let user_a = store
            .create_user(
                &format!(
                    "route-c-budget-user-a-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let user_b = store
            .create_user(
                &format!(
                    "route-c-budget-user-b-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(
                    &user_a.id,
                    "fp-a1",
                    "2026-06-23",
                    Some(10),
                    Some(1),
                )
                .unwrap(),
            1,
            1,
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(
                    &user_a.id,
                    "fp-a2",
                    "2026-06-23",
                    Some(10),
                    Some(1)
                )
                .unwrap(),
            RouteCBudgetRecordResult::UserLimitReached {
                total_used: 1,
                user_used: 1
            }
        );
        recorded_event_id(
            store
                .route_c_budget_try_record_call(
                    &user_b.id,
                    "fp-b1",
                    "2026-06-23",
                    Some(10),
                    Some(1),
                )
                .unwrap(),
            2,
            1,
        );
        assert_eq!(
            store
                .route_c_budget_count_for_day_and_user("2026-06-23", &user_a.id)
                .unwrap(),
            1
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_completion_updates_outcome_without_prompt_text() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!(
                    "route-c-budget-completion-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        let event_id = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10), None)
                .unwrap(),
            1,
            1,
        );
        store
            .route_c_budget_mark_completed(
                &event_id,
                RouteCBudgetCompletion {
                    outcome: "provider_error".to_string(),
                    model: Some("route-c-model".to_string()),
                    total_tokens: Some(42),
                    error_summary: Some(
                        "rate_limit fingerprint=abc secret prompt text".to_string(),
                    ),
                },
            )
            .unwrap();

        let events = store
            .route_c_budget_recent_events(Some("2026-06-23"), Some(&user.id), 10)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, "provider_error");
        assert_eq!(events[0].model.as_deref(), Some("route-c-model"));
        assert_eq!(events[0].total_tokens, Some(42));
        assert_eq!(
            events[0].error_summary.as_deref(),
            Some("category=rate_limit, chars=45, fingerprint=d385d708e9876549")
        );
        assert!(!events[0]
            .error_summary
            .as_deref()
            .unwrap_or_default()
            .contains("secret prompt text"));
        assert!(events[0].completed_at.is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_count_is_day_scoped() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("route-c-budget-day-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10), None)
                .unwrap(),
            1,
            1,
        );

        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 1);
        assert_eq!(store.route_c_budget_count_for_day("2026-06-24").unwrap(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_admin_report_summarizes_and_filters_events() {
        let (store, path) = temp_store();
        let user_a = store
            .create_user(
                &format!(
                    "route-c-budget-admin-a-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let user_b = store
            .create_user(
                &format!(
                    "route-c-budget-admin-b-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_a.id, "fp-a1", "2026-06-22", None, None)
                .unwrap(),
            1,
            1,
        );
        let event_a2 = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_a.id, "fp-a2", "2026-06-23", None, None)
                .unwrap(),
            1,
            1,
        );
        let event_b1 = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_b.id, "fp-b1", "2026-06-23", None, None)
                .unwrap(),
            2,
            1,
        );
        store
            .route_c_budget_mark_completed(
                &event_a2,
                RouteCBudgetCompletion {
                    outcome: "success".to_string(),
                    model: Some("route-c-fast".to_string()),
                    total_tokens: Some(120),
                    error_summary: None,
                },
            )
            .unwrap();
        store
            .route_c_budget_mark_completed(
                &event_b1,
                RouteCBudgetCompletion {
                    outcome: "provider_error".to_string(),
                    model: Some("route-c-fast".to_string()),
                    total_tokens: None,
                    error_summary: Some("rate_limit fingerprint=def".to_string()),
                },
            )
            .unwrap();

        let summaries = store.route_c_budget_day_summaries(10).unwrap();
        assert_eq!(summaries[0].route_day, "2026-06-23");
        assert_eq!(summaries[0].total_calls, 2);
        assert_eq!(summaries[0].completed_calls, 2);
        assert_eq!(summaries[0].success_calls, 1);
        assert_eq!(summaries[0].failed_calls, 1);
        assert_eq!(summaries[0].unique_users, 2);
        assert_eq!(summaries[0].total_tokens, Some(120));
        assert_eq!(summaries[1].route_day, "2026-06-22");
        assert_eq!(summaries[1].total_calls, 1);
        assert_eq!(summaries[1].completed_calls, 0);

        let outcomes = store
            .route_c_budget_outcome_summaries(Some("2026-06-23"), None)
            .unwrap();
        assert_eq!(outcomes.len(), 2);
        let success = outcomes
            .iter()
            .find(|row| row.outcome == "success")
            .expect("success outcome summary");
        assert_eq!(success.total_calls, 1);
        assert_eq!(success.completed_calls, 1);
        assert_eq!(success.unique_users, 1);
        assert_eq!(success.total_tokens, Some(120));
        let provider_error = outcomes
            .iter()
            .find(|row| row.outcome == "provider_error")
            .expect("provider_error outcome summary");
        assert_eq!(provider_error.total_calls, 1);
        assert_eq!(provider_error.completed_calls, 1);
        assert_eq!(provider_error.unique_users, 1);
        assert_eq!(provider_error.total_tokens, None);

        let user_outcomes = store
            .route_c_budget_outcome_summaries(None, Some(&user_a.id))
            .unwrap();
        assert!(user_outcomes.iter().any(|row| row.outcome == "admitted"));
        assert!(user_outcomes.iter().any(|row| row.outcome == "success"));
        assert!(!user_outcomes
            .iter()
            .any(|row| row.outcome == "provider_error"));

        let day_events = store
            .route_c_budget_recent_events(Some("2026-06-23"), None, 10)
            .unwrap();
        assert_eq!(day_events.len(), 2);
        assert!(day_events
            .iter()
            .all(|event| event.route_day == "2026-06-23"));
        assert!(day_events.iter().any(|event| event.outcome == "success"));
        assert!(day_events
            .iter()
            .any(|event| event.outcome == "provider_error"));

        let user_events = store
            .route_c_budget_recent_events(None, Some(&user_a.id), 10)
            .unwrap();
        assert_eq!(user_events.len(), 2);
        assert!(user_events.iter().all(|event| event.user_id == user_a.id));

        let _ = std::fs::remove_file(path);
    }
}
