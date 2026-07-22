//! Transaction-local validation for externally verified terminal snapshots.

use anyhow::Result;
use rusqlite::{OptionalExtension, Transaction};

pub(super) fn preflight_trusted_terminal_snapshot(
    tx: &Transaction<'_>,
    task_id: &str,
    trusted: Option<
        &crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity,
    >,
) -> Result<()> {
    let row = tx
        .query_row(
            "SELECT project_id, workspace_path, workspace_status_json
               FROM local_tasks WHERE task_id = ?1",
            [task_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((project, workspace_path, encoded)) = row else {
        anyhow::bail!("durable local completion has no matching local task row");
    };
    let status = encoded
        .as_deref()
        .map(serde_json::from_str::<serde_json::Value>)
        .transpose()?;
    let supervised = status.as_ref().is_some_and(|value| {
        value
            .get("platform_provenance")
            .and_then(serde_json::Value::as_str)
            == Some("elon.conversation_worktree.v1")
    });
    match (supervised, trusted, status.as_ref()) {
        (true, Some(trusted), Some(status)) => {
            trusted.trusted_workspace_status(task_id, &project, &workspace_path, status)?;
        }
        (true, Some(_), None) => {
            anyhow::bail!("supervised completion has no durable workspace snapshot")
        }
        (true, None, _) => anyhow::bail!("supervised completion lacks a trusted terminal snapshot"),
        (false, Some(_), _) => {
            anyhow::bail!("ordinary completion received a supervised terminal snapshot")
        }
        (false, None, _) => {}
    }
    Ok(())
}

#[cfg(test)]
impl super::LocalTaskStore {
    pub(crate) fn replace_workspace_status_for_test(
        &self,
        task_id: &str,
        workspace_status: &serde_json::Value,
    ) -> Result<()> {
        let encoded = serde_json::to_string(workspace_status)?;
        let changed = self.open()?.execute(
            "UPDATE local_tasks SET workspace_status_json = ?2 WHERE task_id = ?1",
            rusqlite::params![task_id, encoded],
        )?;
        anyhow::ensure!(changed == 1, "fixture task is missing");
        Ok(())
    }
}
