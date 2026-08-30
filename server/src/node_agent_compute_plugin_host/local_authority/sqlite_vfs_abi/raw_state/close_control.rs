//! Exact-allocation, test-only control for rejecting xClose before typed state take.

use rusqlite::ffi;

use super::close_witness::HandleBoundSqliteAbiRawCloseWitness;

enum RawStateTakeRejectionState {
    Observing,
    Armed,
    Rejected,
}

/// Sidecar owned by one exact live SQLite file allocation.
///
/// The raw allocation stores the sole Box pointer. Ordinary close/abandonment clears and drops it;
/// the armed rejection path leaves the original typed state and methods installed while the
/// caller retains the owning allocation for process lifetime. No global selector is involved.
pub(super) struct HandleBoundSqliteAbiRawCloseControl {
    allocation: usize,
    witness: HandleBoundSqliteAbiRawCloseWitness,
    state: RawStateTakeRejectionState,
}

impl HandleBoundSqliteAbiRawCloseControl {
    pub(super) fn new(file: *mut ffi::sqlite3_file) -> Self {
        Self {
            allocation: file as usize,
            witness: HandleBoundSqliteAbiRawCloseWitness::new(),
            state: RawStateTakeRejectionState::Observing,
        }
    }

    pub(super) fn matches_allocation(&self, file: *mut ffi::sqlite3_file) -> bool {
        self.allocation == file as usize
    }

    pub(super) fn witness(&self) -> HandleBoundSqliteAbiRawCloseWitness {
        self.witness.clone()
    }

    pub(super) fn arm_state_take_rejection(
        &mut self,
        file: *mut ffi::sqlite3_file,
    ) -> Option<HandleBoundSqliteAbiRawCloseWitness> {
        if !self.matches_allocation(file)
            || !matches!(self.state, RawStateTakeRejectionState::Observing)
        {
            return None;
        }
        self.state = RawStateTakeRejectionState::Armed;
        Some(self.witness())
    }

    pub(super) fn record_raw_close_entry(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_raw_close_entry();
        }
    }

    pub(super) fn record_state_take_attempt(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_state_take_attempt();
        }
    }

    pub(super) fn record_methods_clear(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_methods_clear();
        }
    }

    pub(super) fn record_state_take_success(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_state_take_success();
        }
    }

    pub(super) fn record_state_abandon(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_state_abandon();
        }
    }

    pub(super) fn record_state_close_attempt(&self, file: *mut ffi::sqlite3_file) {
        if self.matches_allocation(file) {
            self.witness.record_state_close_attempt();
        }
    }

    pub(super) fn rejects_state_take(&mut self, file: *mut ffi::sqlite3_file) -> bool {
        if !self.matches_allocation(file) {
            return false;
        }
        match self.state {
            RawStateTakeRejectionState::Observing => false,
            RawStateTakeRejectionState::Armed => {
                // Consume the selected one-shot. The terminal state keeps subsequent saved-callback
                // retries fail-closed without claiming the selected fault a second time.
                self.state = RawStateTakeRejectionState::Rejected;
                true
            }
            RawStateTakeRejectionState::Rejected => true,
        }
    }
}
