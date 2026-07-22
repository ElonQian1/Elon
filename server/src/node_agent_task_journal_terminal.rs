//! Read-only terminal conflict checks used before any completion-side writes.

use anyhow::Result;

use super::TaskJournal;
use crate::{
    node_agent_task_journal_events::{is_completed_terminal_status, normalize_finish_status},
    node_agent_task_journal_lock::with_task_journal_io_lock,
};

impl TaskJournal {
    pub(crate) fn preflight_finished_with_outcome(
        &self,
        req_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let registry = self.load_registry()?;
            if let Some(record) = registry.get(req_id) {
                let requested = normalize_finish_status(status, error);
                anyhow::ensure!(
                    !is_completed_terminal_status(&record.status) || record.status == requested,
                    "task journal terminal status conflicts with durable completion"
                );
            }
            Ok(())
        })
    }
}
