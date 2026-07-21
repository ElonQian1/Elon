//! Early persistence of the isolated workspace identity used by supervised recovery.

use anyhow::Result;
use rusqlite::params;

use super::{read_record, select_sql, LocalTaskRecord, LocalTaskStore};

impl LocalTaskStore {
    pub(crate) fn record_initial_workspace_status(
        &self,
        task_id: &str,
        workspace_status: &serde_json::Value,
    ) -> Result<bool> {
        let encoded = serde_json::to_string(workspace_status)?;
        Ok(self.open()?.execute(
            "UPDATE local_tasks SET workspace_status_json = ?2
              WHERE task_id = ?1 AND workspace_status_json IS NULL
                AND completion_event_id IS NULL",
            params![task_id, encoded],
        )? > 0)
    }

    pub(crate) fn list_identity_candidates(&self) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!("{} ORDER BY started_at_ms DESC", select_sql()))?;
        let records = stmt
            .query_map([], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }
}
