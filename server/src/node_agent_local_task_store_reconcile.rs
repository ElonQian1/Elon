//! Atomic repair of a missing local-task row from already-validated evidence.

use anyhow::{bail, Context, Result};
use homecli_proto::CliCompletionEnvelope;
use rusqlite::{params, OptionalExtension, Transaction};

use super::{read_record, LocalTaskRecord, LocalTaskStore};

pub(super) fn ensure_same_event_is_immutable(
    tx: &Transaction<'_>,
    completion: &CliCompletionEnvelope,
) -> Result<()> {
    let existing = tx
        .query_row(
            "SELECT status, error, final_reply, completion_event_id, finished_at_ms
               FROM local_tasks WHERE task_id = ?1",
            [&completion.req_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((status, error, outcome, event_id, finished_at_ms)) = existing else {
        return Ok(());
    };
    let Some(event_id) = event_id.as_deref() else {
        return Ok(());
    };
    anyhow::ensure!(
        event_id == completion.event_id,
        "local task already binds a different completion event"
    );
    let expected_status = crate::node_agent_task_journal_events::completion_terminal_status(
        completion.exit_ok,
        completion.error.as_deref(),
    );
    let expected_outcome =
        (!completion.final_output.trim().is_empty()).then_some(completion.final_output.as_str());
    let expected_finished = completion.created_at_ms.min(i64::MAX as u64) as i64;
    anyhow::ensure!(
        status == expected_status
            && error.as_deref() == completion.error.as_deref()
            && outcome.as_deref() == expected_outcome
            && finished_at_ms == Some(expected_finished),
        "same local completion event conflicts with status, outcome, or finished time"
    );
    Ok(())
}

impl LocalTaskStore {
    /// Repair local display state from the durable outbox during startup.
    pub(crate) fn reconcile_completion(&self, completion: &CliCompletionEnvelope) -> Result<bool> {
        self.reconcile_completion_trusted(completion, None)
    }

    pub(crate) fn reconcile_completion_trusted(
        &self,
        completion: &CliCompletionEnvelope,
        trusted: Option<
            &crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity,
        >,
    ) -> Result<bool> {
        anyhow::ensure!(
            completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN,
            "only local_offline completions can reconcile local tasks"
        );
        let context = completion
            .project_context
            .as_ref()
            .context("local completion is missing project context")?;
        let producer = completion
            .producer_identity
            .as_ref()
            .context("local completion is missing producer identity")?;
        self.finish_scoped_with_context(
            Some(producer.owner_user_id.as_str()),
            Some(producer.agent_id.as_str()),
            Some(producer.install_id.as_str()),
            Some(context.project_id.as_str()),
            Some(context.conversation_id.as_str()),
            completion,
            trusted,
        )
    }

    pub(crate) fn preflight_completion(
        &self,
        completion: &CliCompletionEnvelope,
        trusted: Option<
            &crate::node_agent_supervision_terminal_lease_safety::VerifiedTerminalLeaseIdentity,
        >,
    ) -> Result<()> {
        let conn = self.open()?;
        let tx = conn.unchecked_transaction()?;
        ensure_same_event_is_immutable(&tx, completion)?;
        super::terminal::preflight_trusted_terminal_snapshot(&tx, &completion.req_id, trusted)?;
        tx.rollback()?;
        Ok(())
    }
}

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
