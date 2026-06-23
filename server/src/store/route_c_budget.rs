//! Persistent Route C runtime call budget ledger.
//!
//! This records Route C admission attempts before the provider call starts, so
//! the platform-level daily budget survives server restarts.

use anyhow::Result;
use rusqlite::params;

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn route_c_budget_count_for_day(&self, route_day: &str) -> Result<usize> {
        let count: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events WHERE route_day = ?1",
            params![route_day],
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
    ) -> Result<(bool, usize)> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let current: i64 = tx.query_row(
            "SELECT COUNT(*) FROM route_c_runtime_budget_events WHERE route_day = ?1",
            params![route_day],
            |row| row.get(0),
        )?;
        let current = current.max(0) as usize;
        if daily_call_limit.is_some_and(|limit| current >= limit) {
            tx.commit()?;
            return Ok((false, current));
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
        Ok((true, current + 1))
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
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(2))
                .unwrap(),
            (true, 1)
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-2", "2026-06-23", Some(2))
                .unwrap(),
            (true, 2)
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-3", "2026-06-23", Some(2))
                .unwrap(),
            (false, 2)
        );
        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 2);

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
            .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10))
            .unwrap();

        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 1);
        assert_eq!(store.route_c_budget_count_for_day("2026-06-24").unwrap(), 0);

        let _ = std::fs::remove_file(path);
    }
}
