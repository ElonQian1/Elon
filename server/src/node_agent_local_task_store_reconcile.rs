//! Atomic repair of a missing local-task row from already-validated evidence.

use anyhow::{bail, Context, Result};
use rusqlite::params;

use super::{read_record, LocalTaskRecord, LocalTaskStore};

pub(crate) struct RecoveredLocalTaskStart<'a> {
    pub(crate) task_id: &'a str,
    pub(crate) owner_user_id: &'a str,
    pub(crate) agent_id: &'a str,
    pub(crate) install_id: &'a str,
    pub(crate) project_id: &'a str,
    pub(crate) conversation_id: &'a str,
    pub(crate) workspace_path: &'a str,
    pub(crate) prompt: &'a str,
    pub(crate) cli: &'a str,
    pub(crate) runtime_permission: &'a str,
    pub(crate) status: &'a str,
    pub(crate) error: &'a str,
    pub(crate) workspace_status: &'a serde_json::Value,
    pub(crate) started_at_ms: i64,
}

impl LocalTaskStore {
    /// The caller must validate journal, supervision, workspace, grant, and
    /// exact root lease evidence before reaching this persistence boundary.
    pub(crate) fn reconcile_missing_supervised(
        &self,
        start: RecoveredLocalTaskStart<'_>,
    ) -> Result<LocalTaskRecord> {
        let mut conn = self.open()?;
        let encoded = serde_json::to_string(start.workspace_status)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO local_tasks (
               task_id, owner_user_id, agent_id, install_id, project_id,
               conversation_id, workspace_path, prompt, cli, runtime_permission,
               execution_origin, billing_source, status, error,
               workspace_status_json, sync_state, started_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,
                       'local_offline','own_codex',?11,?12,?13,'local_only',?14)",
            params![
                start.task_id,
                start.owner_user_id,
                start.agent_id,
                start.install_id,
                start.project_id,
                start.conversation_id,
                start.workspace_path,
                start.prompt,
                start.cli,
                start.runtime_permission,
                start.status,
                start.error,
                encoded,
                start.started_at_ms,
            ],
        )?;
        let record = tx
            .query_row(
                &format!("{} WHERE task_id = ?1", super::select_sql()),
                [start.task_id],
                read_record,
            )
            .context("reconciled local task row is not readable")?;
        tx.commit()?;
        if record.owner_user_id != start.owner_user_id
            || record.agent_id != start.agent_id
            || record.install_id != start.install_id
            || !crate::node_agent_full_access::project_ids_equivalent(
                &record.project_id,
                start.project_id,
            )
            || record.conversation_id != start.conversation_id
            || !crate::node_agent_update_checkpoint::same_path(
                std::path::Path::new(&record.workspace_path),
                std::path::Path::new(start.workspace_path),
            )
        {
            bail!("reconciled task id belongs to a different durable identity");
        }
        Ok(record)
    }
}
