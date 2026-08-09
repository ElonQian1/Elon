//! Deterministic failure injection for custody tests. This module is absent from production builds.

use super::{coordinator::PinnedManagedSqliteWalRuntime, types::ManagedSqliteShmFailurePhase};

impl PinnedManagedSqliteWalRuntime {
    pub(crate) fn inject_terminal_gate_failure_for_test(&self) {
        let mut state = self
            .coordinator
            .state
            .lock()
            .expect("lock WAL coordinator before deterministic test failure");
        self.coordinator
            .mark_poisoned(&mut state, ManagedSqliteShmFailurePhase::Gate, true, true);
    }
}
