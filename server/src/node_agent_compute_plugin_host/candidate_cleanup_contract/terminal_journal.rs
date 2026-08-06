use std::fmt;

use super::PhysicallyExecutedCandidateCleanup;

/// Opaque proof that a future journal Store has sealed the expected topology and durably advanced
/// every object through intent, disposition, parent-relative absence and namespace durability.
/// No constructor exists until that Store can produce this capability from exact readback.
#[must_use = "terminal cleanup journal retains physical custody and the root lock"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateCleanupTerminalJournal {
    pub(super) physical: PhysicallyExecutedCandidateCleanup,
    pub(super) execution_plan_digest: String,
    pub(super) terminal_journal_digest: String,
}

impl DurableCandidateCleanupTerminalJournal {
    pub(in crate::node_agent_compute_plugin_host) fn physical(
        &self,
    ) -> &PhysicallyExecutedCandidateCleanup {
        &self.physical
    }
    pub(in crate::node_agent_compute_plugin_host) fn execution_plan_digest(&self) -> &str {
        &self.execution_plan_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn terminal_journal_digest(&self) -> &str {
        &self.terminal_journal_digest
    }
}

impl fmt::Debug for DurableCandidateCleanupTerminalJournal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableCandidateCleanupTerminalJournal")
            .field("execution_plan_digest", &self.execution_plan_digest)
            .field("terminal_journal_digest", &self.terminal_journal_digest)
            .field("physical", &"<retained-custody>")
            .finish()
    }
}
