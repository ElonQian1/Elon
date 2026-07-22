//! Strict outbox transitions used only after historical terminal verification.

use anyhow::{Context, Result};
use homecli_proto::CliCompletionEnvelope;
use rusqlite::{params, OptionalExtension};

use super::{
    support::{required_id, validate_completion},
    CliCompletionOutbox, STATUS_DEAD_LETTER, STATUS_PENDING,
};

impl CliCompletionOutbox {
    /// Validate that historical terminal recovery may create or revive this
    /// exact durable envelope. An acknowledged row is never reopened.
    pub(crate) fn preflight_restore_pending(
        &self,
        completion: &CliCompletionEnvelope,
    ) -> Result<()> {
        validate_completion(completion)?;
        let payload_json = serde_json::to_string(completion).context("序列化 CLI completion")?;
        let conn = self.connect()?;
        let existing = conn
            .query_row(
                "SELECT event_id, req_id, payload_json, status
                   FROM cli_completion_outbox
                  WHERE event_id = ?1 OR req_id = ?2
                  ORDER BY CASE WHEN event_id = ?1 THEN 0 ELSE 1 END
                  LIMIT 1",
                params![completion.event_id, completion.req_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((event_id, req_id, payload, status)) = existing else {
            return Ok(());
        };
        anyhow::ensure!(
            event_id == completion.event_id
                && req_id == completion.req_id
                && payload == payload_json,
            "historical CLI completion conflicts with the durable outbox binding"
        );
        anyhow::ensure!(
            matches!(status.as_str(), STATUS_PENDING | STATUS_DEAD_LETTER),
            "acknowledged CLI completion cannot be reopened by historical recovery"
        );
        Ok(())
    }

    /// Make an exact, already validated completion replayable. Missing rows are
    /// inserted by `enqueue`; this transition only revives a retained rejection.
    pub(crate) fn restore_pending(&self, event_id: &str, req_id: &str) -> Result<bool> {
        let event_id = required_id(event_id, "event_id")?;
        let req_id = required_id(req_id, "req_id")?;
        let conn = self.connect()?;
        let changed = conn.execute(
            "UPDATE cli_completion_outbox
                SET status = ?3, last_error = NULL, last_attempt_at_ms = NULL
              WHERE event_id = ?1 AND req_id = ?2 AND status = ?4",
            params![event_id, req_id, STATUS_PENDING, STATUS_DEAD_LETTER],
        )?;
        if changed > 0 {
            return Ok(true);
        }
        let status = conn
            .query_row(
                "SELECT status FROM cli_completion_outbox
                  WHERE event_id = ?1 AND req_id = ?2",
                params![event_id, req_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        anyhow::ensure!(
            status.as_deref() == Some(STATUS_PENDING),
            "historical CLI completion is missing or no longer replayable"
        );
        Ok(false)
    }
}
