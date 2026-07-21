//! Unbounded local-task reads reserved for fail-closed safety decisions.

use anyhow::Result;
use rusqlite::params;

use super::{read_record, select_sql, LocalTaskRecord, LocalTaskStore};

impl LocalTaskStore {
    /// Safety checks must not inherit the user-facing list endpoint's 100-row cap.
    pub(crate) fn list_all_for_owner_for_safety(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<LocalTaskRecord>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE owner_user_id = ?1 ORDER BY started_at_ms DESC, task_id DESC",
            select_sql()
        ))?;
        let records = stmt
            .query_map(params![owner_user_id], read_record)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_local_task_store::LocalTaskStart;

    #[test]
    fn safety_listing_finds_tasks_older_than_the_public_hundred_row_window() {
        let path = std::env::temp_dir().join(format!(
            "elon-local-task-safety-{}-{}.sqlite3",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let store = LocalTaskStore::new(path);
        create_task(&store, "local-old-resume-child");
        std::thread::sleep(std::time::Duration::from_millis(5));
        for index in 0..101 {
            create_task(&store, &format!("local-new-{index:03}"));
        }

        assert!(!store
            .list_for_owner("usr-a", 100)
            .unwrap()
            .iter()
            .any(|task| task.task_id == "local-old-resume-child"));
        assert!(store
            .list_all_for_owner_for_safety("usr-a")
            .unwrap()
            .iter()
            .any(|task| task.task_id == "local-old-resume-child"));
    }

    fn create_task(store: &LocalTaskStore, task_id: &str) {
        store
            .create(LocalTaskStart {
                task_id,
                owner_user_id: "usr-a",
                agent_id: "node-a",
                install_id: "install-a",
                project_id: "prj-a",
                channel_id: None,
                conversation_id: task_id,
                workspace_path: "D:/demo",
                prompt: "test",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
    }
}
