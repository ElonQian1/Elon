//! Atomic local-task transitions used by orphan ownership reconciliation.

use std::collections::HashSet;

use anyhow::Result;
use rusqlite::params;

use super::{now_ms, LocalTaskStore};

const ORPHAN_RESUME_REQUIRED_REASON: &str = "本机执行句柄、进程、sidecar 与 heartbeat 均已过期：工作区、journal 和 completion outbox 已保留，请点击 Resume 检查现场后续跑";

impl LocalTaskStore {
    /// Legacy bulk entry retained for callers that already hold a complete
    /// authoritative ownership set. New orphan reconciliation uses the
    /// guarded per-task transition below.
    pub(crate) fn mark_stale_without_runtime(
        &self,
        protected_req_ids: &HashSet<String>,
        started_before_ms: i64,
    ) -> Result<usize> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        let running_ids = {
            let mut stmt = tx.prepare(
                "SELECT task_id FROM local_tasks
                  WHERE status IN ('running','recovering','reattaching')
                    AND completion_event_id IS NULL
                    AND started_at_ms <= ?1",
            )?;
            let ids = stmt
                .query_map([started_before_ms], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids
        };
        let mut changed = 0;
        for task_id in running_ids {
            if protected_req_ids.contains(&task_id) {
                continue;
            }
            changed += tx.execute(
                "UPDATE local_tasks
                    SET status = 'resume_required', error = ?4,
                        sync_state = 'local_only', finished_at_ms = ?1
                  WHERE task_id = ?2
                    AND status IN ('running','recovering','reattaching')
                    AND completion_event_id IS NULL
                    AND started_at_ms <= ?3",
                params![
                    now_ms(),
                    task_id,
                    started_before_ms,
                    ORPHAN_RESUME_REQUIRED_REASON
                ],
            )?;
        }
        tx.commit()?;
        Ok(changed)
    }

    pub(crate) fn mark_one_stale_without_runtime(
        &self,
        task_id: &str,
        started_before_ms: i64,
    ) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks
                SET status = 'resume_required', error = ?2,
                    sync_state = 'local_only', finished_at_ms = ?3
              WHERE task_id = ?1
                AND status IN ('running','recovering','reattaching')
                AND completion_event_id IS NULL
                AND started_at_ms <= ?4",
            params![
                task_id,
                ORPHAN_RESUME_REQUIRED_REASON,
                now_ms(),
                started_before_ms
            ],
        )? > 0)
    }

    pub(crate) fn restore_running_after_orphan_claim(&self, task_id: &str) -> Result<bool> {
        Ok(self.open()?.execute(
            "UPDATE local_tasks
                SET status = 'running', error = NULL, finished_at_ms = NULL
              WHERE task_id = ?1 AND status = 'resume_required'
                AND completion_event_id IS NULL AND error = ?2",
            params![task_id, ORPHAN_RESUME_REQUIRED_REASON],
        )? > 0)
    }
}
