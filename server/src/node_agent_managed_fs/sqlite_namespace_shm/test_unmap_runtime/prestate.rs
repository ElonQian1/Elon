use super::super::ManagedSqliteDeleteOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestUnmapDeletePrestate {
    MissingIdentity,
    IdentityMismatch,
    LockQueryUnavailable,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestUnmapDeletePrestateReceipt {
    pub(crate) prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    pub(crate) consumed: bool,
    pub(crate) applied: bool,
    pub(crate) setup_delete_attempts: u8,
    pub(crate) setup_delete_outcome: Option<ManagedSqliteDeleteOutcome>,
}

#[derive(Default)]
pub(super) struct ManagedSqliteShmTestUnmapDeletePrestateControl {
    installed: Option<ManagedSqliteShmTestUnmapDeletePrestate>,
    consumed: bool,
    applied: bool,
    setup_delete_attempts: u8,
    setup_delete_outcome: Option<ManagedSqliteDeleteOutcome>,
}

impl ManagedSqliteShmTestUnmapDeletePrestateControl {
    pub(super) fn install(
        &mut self,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    ) -> Result<(), &'static str> {
        if self.installed.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_PRESTATE_ALREADY_INSTALLED");
        }
        self.installed = Some(prestate);
        Ok(())
    }

    pub(super) fn take_authority(&mut self) -> Option<ManagedSqliteShmTestUnmapDeletePrestate> {
        let prestate = self.installed?;
        if self.consumed || prestate == ManagedSqliteShmTestUnmapDeletePrestate::NotFound {
            return None;
        }
        self.consumed = true;
        Some(prestate)
    }

    pub(super) fn take_not_found(&mut self) -> Option<ManagedSqliteShmTestUnmapDeletePrestate> {
        let prestate = self.installed?;
        if self.consumed || prestate != ManagedSqliteShmTestUnmapDeletePrestate::NotFound {
            return None;
        }
        self.consumed = true;
        Some(prestate)
    }

    pub(super) fn mark_applied(&mut self, prestate: ManagedSqliteShmTestUnmapDeletePrestate) {
        if self.installed == Some(prestate) && self.consumed {
            self.applied = true;
        }
    }

    pub(super) fn record_setup_delete(
        &mut self,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
        outcome: ManagedSqliteDeleteOutcome,
    ) -> Result<(), &'static str> {
        if self.installed != Some(prestate)
            || prestate != ManagedSqliteShmTestUnmapDeletePrestate::NotFound
            || !self.consumed
            || self.setup_delete_attempts != 0
            || self.setup_delete_outcome.is_some()
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_PRESTATE_SETUP_INVALID");
        }
        self.setup_delete_attempts = 1;
        self.setup_delete_outcome = Some(outcome);
        self.applied = true;
        Ok(())
    }

    pub(super) fn receipt(&self) -> Option<ManagedSqliteShmTestUnmapDeletePrestateReceipt> {
        Some(ManagedSqliteShmTestUnmapDeletePrestateReceipt {
            prestate: self.installed?,
            consumed: self.consumed,
            applied: self.applied,
            setup_delete_attempts: self.setup_delete_attempts,
            setup_delete_outcome: self.setup_delete_outcome,
        })
    }

    pub(super) fn pending(&self) -> usize {
        usize::from(self.installed.is_some() && !self.consumed)
    }
}
