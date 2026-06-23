//! Persistent Route C runtime call budget ledger.
//!
//! This records Route C admission attempts before the provider call starts, so
//! the platform-level daily budget survives server restarts.

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{new_id, now, Store};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCBudgetDaySummary {
    pub route_day: String,
    pub total_calls: i64,
    pub unique_users: i64,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteCBudgetRecordResult {
    Recorded { total_used: usize, user_used: usize },
    PlatformLimitReached { total_used: usize, user_used: usize },
    UserLimitReached { total_used: usize, user_used: usize },
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

        tx.execute(
            "INSERT INTO route_c_runtime_budget_events (
               id, user_id, request_fingerprint, route_day, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                new_id("rcb"),
                user_id,
                request_fingerprint,
                route_day,
                now(),
            ],
        )?;
        tx.commit()?;
        Ok(RouteCBudgetRecordResult::Recorded {
            total_used: total_used + 1,
            user_used: user_used + 1,
        })
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
                    COUNT(DISTINCT user_id) AS unique_users,
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
                    unique_users: row.get(2)?,
                    first_created_at: row.get(3)?,
                    last_created_at: row.get(4)?,
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
            "SELECT id, user_id, request_fingerprint, route_day, created_at
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
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
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

        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(2), None)
                .unwrap(),
            RouteCBudgetRecordResult::Recorded {
                total_used: 1,
                user_used: 1
            }
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-2", "2026-06-23", Some(2), None)
                .unwrap(),
            RouteCBudgetRecordResult::Recorded {
                total_used: 2,
                user_used: 2
            }
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

        assert_eq!(
            store
                .route_c_budget_try_record_call(
                    &user_a.id,
                    "fp-a1",
                    "2026-06-23",
                    Some(10),
                    Some(1)
                )
                .unwrap(),
            RouteCBudgetRecordResult::Recorded {
                total_used: 1,
                user_used: 1
            }
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
        assert_eq!(
            store
                .route_c_budget_try_record_call(
                    &user_b.id,
                    "fp-b1",
                    "2026-06-23",
                    Some(10),
                    Some(1)
                )
                .unwrap(),
            RouteCBudgetRecordResult::Recorded {
                total_used: 2,
                user_used: 1
            }
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

        store
            .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10), None)
            .unwrap();

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

        store
            .route_c_budget_try_record_call(&user_a.id, "fp-a1", "2026-06-22", None, None)
            .unwrap();
        store
            .route_c_budget_try_record_call(&user_a.id, "fp-a2", "2026-06-23", None, None)
            .unwrap();
        store
            .route_c_budget_try_record_call(&user_b.id, "fp-b1", "2026-06-23", None, None)
            .unwrap();

        let summaries = store.route_c_budget_day_summaries(10).unwrap();
        assert_eq!(summaries[0].route_day, "2026-06-23");
        assert_eq!(summaries[0].total_calls, 2);
        assert_eq!(summaries[0].unique_users, 2);
        assert_eq!(summaries[1].route_day, "2026-06-22");
        assert_eq!(summaries[1].total_calls, 1);

        let day_events = store
            .route_c_budget_recent_events(Some("2026-06-23"), None, 10)
            .unwrap();
        assert_eq!(day_events.len(), 2);
        assert!(day_events
            .iter()
            .all(|event| event.route_day == "2026-06-23"));

        let user_events = store
            .route_c_budget_recent_events(None, Some(&user_a.id), 10)
            .unwrap();
        assert_eq!(user_events.len(), 2);
        assert!(user_events.iter().all(|event| event.user_id == user_a.id));

        let _ = std::fs::remove_file(path);
    }
}
