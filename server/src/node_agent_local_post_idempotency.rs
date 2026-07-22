//! Atomic binding for retry-safe ordinary localhost POST requests.

use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::{now_ms, LocalTaskStore};

const CLAIM_LEASE_MS: i64 = 30_000;

#[derive(Debug, PartialEq)]
pub(crate) enum IdempotencyClaim {
    Claimed {
        task_id: String,
    },
    InFlight {
        task_id: String,
    },
    Completed {
        task_id: String,
        status: u16,
        body: serde_json::Value,
    },
    Conflict,
}

impl LocalTaskStore {
    pub(crate) fn claim_local_post(
        &self,
        owner_user_id: &str,
        key: &str,
        method: &str,
        path: &str,
        body_sha256: &str,
        proposed_task_id: &str,
    ) -> Result<IdempotencyClaim> {
        self.claim_local_post_at(
            owner_user_id,
            key,
            method,
            path,
            body_sha256,
            proposed_task_id,
            now_ms(),
        )
    }

    fn claim_local_post_at(
        &self,
        owner_user_id: &str,
        key: &str,
        method: &str,
        path: &str,
        body_sha256: &str,
        proposed_task_id: &str,
        now: i64,
    ) -> Result<IdempotencyClaim> {
        let mut conn = self.open()?;
        ensure_schema(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                "SELECT owner_user_id, method, path, body_sha256, task_id,
                        response_status, response_json, claim_until_ms
                   FROM local_post_idempotency WHERE idempotency_key = ?1",
                params![key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<u16>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )
            .optional()?;
        let claim = match existing {
            None => {
                tx.execute(
                    "INSERT INTO local_post_idempotency
                     (idempotency_key, owner_user_id, method, path, body_sha256, task_id,
                      response_status, response_json, claim_until_ms, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, NULL, ?7, ?8)",
                    params![
                        key,
                        owner_user_id,
                        method,
                        path,
                        body_sha256,
                        proposed_task_id,
                        now.saturating_add(CLAIM_LEASE_MS),
                        now
                    ],
                )?;
                IdempotencyClaim::Claimed {
                    task_id: proposed_task_id.to_string(),
                }
            }
            Some((owner, bound_method, bound_path, digest, task_id, status, body, until)) => {
                if owner != owner_user_id
                    || bound_method != method
                    || bound_path != path
                    || digest != body_sha256
                {
                    IdempotencyClaim::Conflict
                } else if let (Some(status), Some(body)) = (status, body) {
                    IdempotencyClaim::Completed {
                        task_id,
                        status,
                        body: serde_json::from_str(&body)
                            .context("decode cached idempotent response")?,
                    }
                } else if until > now {
                    IdempotencyClaim::InFlight { task_id }
                } else {
                    tx.execute(
                        "UPDATE local_post_idempotency
                            SET claim_until_ms = ?2, updated_at_ms = ?3
                          WHERE idempotency_key = ?1",
                        params![key, now.saturating_add(CLAIM_LEASE_MS), now],
                    )?;
                    IdempotencyClaim::Claimed { task_id }
                }
            }
        };
        tx.commit()?;
        Ok(claim)
    }

    pub(crate) fn complete_local_post(
        &self,
        owner_user_id: &str,
        key: &str,
        task_id: &str,
        status: u16,
        body: &serde_json::Value,
    ) -> Result<bool> {
        let mut conn = self.open()?;
        ensure_schema(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = tx.execute(
            "UPDATE local_post_idempotency
                SET response_status = COALESCE(response_status, ?4),
                    response_json = COALESCE(response_json, ?5), updated_at_ms = ?6
              WHERE idempotency_key = ?1 AND owner_user_id = ?2 AND task_id = ?3",
            params![
                key,
                owner_user_id,
                task_id,
                status,
                serde_json::to_string(body)?,
                now_ms()
            ],
        )?;
        tx.commit()?;
        Ok(changed > 0)
    }
}

fn ensure_schema(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS local_post_idempotency (
            idempotency_key TEXT PRIMARY KEY,
            owner_user_id TEXT NOT NULL,
            method TEXT NOT NULL,
            path TEXT NOT NULL,
            body_sha256 TEXT NOT NULL,
            task_id TEXT NOT NULL,
            response_status INTEGER,
            response_json TEXT,
            claim_until_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_survives_restart_replays_result_and_rejects_body_change() {
        let root = std::env::temp_dir().join(format!(
            "local-post-idempotency-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let path = root.join("tasks.sqlite3");
        let first = LocalTaskStore::new(&path);
        assert_eq!(
            first
                .claim_local_post_at(
                    "owner",
                    "key",
                    "POST",
                    "/api/local-tasks",
                    "aaa",
                    "task-1",
                    10
                )
                .unwrap(),
            IdempotencyClaim::Claimed {
                task_id: "task-1".into()
            }
        );
        let body = serde_json::json!({"ok": true, "task_id": "task-1"});
        assert!(first
            .complete_local_post("owner", "key", "task-1", 202, &body)
            .unwrap());
        drop(first);

        let reopened = LocalTaskStore::new(&path);
        assert_eq!(
            reopened
                .claim_local_post_at(
                    "owner",
                    "key",
                    "POST",
                    "/api/local-tasks",
                    "aaa",
                    "other",
                    20
                )
                .unwrap(),
            IdempotencyClaim::Completed {
                task_id: "task-1".into(),
                status: 202,
                body
            }
        );
        assert_eq!(
            reopened
                .claim_local_post_at(
                    "owner",
                    "key",
                    "POST",
                    "/api/local-tasks",
                    "bbb",
                    "other",
                    20
                )
                .unwrap(),
            IdempotencyClaim::Conflict
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn expired_claim_is_recovered_with_the_original_task_identity() {
        let root = std::env::temp_dir().join(format!(
            "local-post-claim-recovery-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
        store
            .claim_local_post_at("owner", "key", "POST", "/path", "aaa", "task-1", 10)
            .unwrap();
        assert_eq!(
            store
                .claim_local_post_at(
                    "owner",
                    "key",
                    "POST",
                    "/path",
                    "aaa",
                    "task-2",
                    10 + CLAIM_LEASE_MS + 1,
                )
                .unwrap(),
            IdempotencyClaim::Claimed {
                task_id: "task-1".into()
            }
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
