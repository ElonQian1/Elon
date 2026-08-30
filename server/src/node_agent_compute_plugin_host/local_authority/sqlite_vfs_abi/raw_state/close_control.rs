//! Exact-allocation, test-only control for retaining a successfully taken xClose state.

use std::mem::ManuallyDrop;

use rusqlite::ffi;

use super::{
    super::file_state::HandleBoundSqliteFileState,
    close_witness::HandleBoundSqliteAbiRawCloseWitness,
};

enum RawStateTakeRejectionState {
    Observing,
    Armed,
    Retained {
        // This is deliberate process-lifetime custody. Dropping the control must never run the
        // file state's Drop after raw slots were cleared without completing physical xClose.
        _state: ManuallyDrop<Box<HandleBoundSqliteFileState>>,
    },
}

/// Sidecar owned by one exact live SQLite file allocation.
///
/// The raw allocation stores the sole Box pointer. Ordinary close/abandonment clears and drops it;
/// the armed rejection path instead keeps this typed owner beside the allocation that the caller
/// must retain for process lifetime. No process-global selector or address registry is involved.
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

    pub(super) fn retain_taken_state_if_armed(
        &mut self,
        file: *mut ffi::sqlite3_file,
        state: Box<HandleBoundSqliteFileState>,
    ) -> Option<Box<HandleBoundSqliteFileState>> {
        if !self.matches_allocation(file)
            || !matches!(self.state, RawStateTakeRejectionState::Armed)
        {
            return Some(state);
        }
        self.witness.record_state_close_custody_retention();
        self.state = RawStateTakeRejectionState::Retained {
            _state: ManuallyDrop::new(state),
        };
        None
    }

    pub(super) fn retains_process_lifetime_custody(&self) -> bool {
        matches!(self.state, RawStateTakeRejectionState::Retained { .. })
    }
}
