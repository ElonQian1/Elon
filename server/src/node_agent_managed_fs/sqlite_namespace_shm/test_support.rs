//! Deterministic failure injection for custody tests. This module is absent from production builds.

use super::{coordinator::PinnedManagedSqliteWalRuntime, types::ManagedSqliteShmFailurePhase};
#[cfg(all(test, windows))]
use crate::node_agent_managed_fs::ManagedSqliteFileKind;

/// Ordered proof that the exact SHM sibling was created and closed before WAL initialization.
#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmExistingFilePrecreationReceiptV1 {
    ordered_values: [u64; 8],
}

#[cfg(all(test, windows))]
impl ManagedSqliteShmExistingFilePrecreationReceiptV1 {
    pub(crate) const fn ordered_values(self) -> [u64; 8] {
        self.ordered_values
    }
}

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

    /// Creates and closes the physical SHM sibling without attaching a coordinator node.
    #[cfg(all(test, windows))]
    pub(crate) fn precreate_existing_shm_for_initialization_test_v1(
        &self,
    ) -> Result<ManagedSqliteShmExistingFilePrecreationReceiptV1, &'static str> {
        let file = match self.coordinator.namespace.open_shm_for_wal() {
            Ok(file) => file,
            Err(failure) => {
                // An open failure may retain a live or terminal handle. The q13 child owns that
                // custody until process exit; dropping it here could retry an unsafe close.
                std::mem::forget(failure);
                return Err("NODE_MANAGED_SQLITE_SHM_TEST_PRECREATE_OPEN_FAILED");
            }
        };
        let was_created = file.was_created();
        let kind = file.kind();
        let identity_digest_present = !file.identity_digest().is_empty();
        let close = match file.close() {
            Ok(close) => close,
            Err(failure) => {
                // Preserve outcome-uncertain or still-live close custody for the isolated child.
                std::mem::forget(failure);
                return Err("NODE_MANAGED_SQLITE_SHM_TEST_PRECREATE_CLOSE_FAILED");
            }
        };
        if !was_created
            || kind != ManagedSqliteFileKind::Shm
            || !identity_digest_present
            || close.kind() != ManagedSqliteFileKind::Shm
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_PRECREATE_RECEIPT_INVALID");
        }
        Ok(ManagedSqliteShmExistingFilePrecreationReceiptV1 {
            ordered_values: [1, 1, 1, 4, 1, 1, 4, 1],
        })
    }
}
