//! Transaction-local validation for externally verified terminal snapshots.

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Transaction};

use super::LocalTaskStore;

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

impl LocalTaskStore {
    /// Reopen only a locally trusted done result for durable outbox replay.
    /// This is used after historical receipt reconciliation, never to reinterpret
    /// an ordinary server rejection.
    pub(crate) fn mark_trusted_completion_pending(
        &self,
        task_id: &str,
        event_id: &str,
    ) -> Result<bool> {
        let conn = self.open()?;
        let row = conn
            .query_row(
                "SELECT status, error, workspace_status_json, sync_state, completion_event_id
                   FROM local_tasks WHERE task_id = ?1",
                [task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((status, error, encoded, sync_state, bound_event)) = row else {
            anyhow::bail!("historical terminal task disappeared before outbox replay");
        };
        let workspace_status = encoded
            .as_deref()
            .map(serde_json::from_str::<serde_json::Value>)
            .transpose()?
            .context("historical terminal task has no workspace snapshot")?;
        anyhow::ensure!(
            status == "done"
                && error.is_none()
                && bound_event.as_deref() == Some(event_id)
                && workspace_status
                    .get("terminal_snapshot_status")
                    .and_then(serde_json::Value::as_str)
                    == Some("trusted"),
            "historical terminal task is not a trusted done completion"
        );
        anyhow::ensure!(
            sync_state != "synced",
            "synced terminal completion cannot be reopened"
        );
        if sync_state == "pending" {
            return Ok(false);
        }
        Ok(conn.execute(
            "UPDATE local_tasks SET sync_state = 'pending', server_ack_at_ms = NULL
              WHERE task_id = ?1 AND completion_event_id = ?2
                AND status = 'done' AND sync_state <> 'synced'",
            rusqlite::params![task_id, event_id],
        )? > 0)
    }
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
