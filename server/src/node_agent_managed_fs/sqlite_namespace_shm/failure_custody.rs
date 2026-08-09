use std::io;

use super::super::{
    types::ManagedSqliteFailureHandleCustody, ManagedSqliteDeleteFailure,
    ManagedSqliteFileOpenFailure, ManagedSqliteQuarantinedFileCloseFailure,
};
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        ManagedSqliteShmFileCloseCustody,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
};

impl ManagedSqliteShmCoordinator {
    pub(super) fn consume_open_failure(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        failure: ManagedSqliteFileOpenFailure,
    ) -> ManagedSqliteShmFailure {
        let parts = failure.into_shm_parts();
        if let Some(close_error) = self.retain_handle_custody(state, parts.custody) {
            self.mark_poisoned(state, ManagedSqliteShmFailurePhase::FileClose, true, false);
            return ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::FileClose,
                close_error,
                true,
                false,
            );
        }
        self.mark_poisoned(
            state,
            ManagedSqliteShmFailurePhase::ExactSiblingOpen,
            true,
            false,
        );
        ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::ExactSiblingOpen,
            parts.error,
            true,
            false,
        )
    }

    pub(super) fn consume_delete_failure(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        failure: ManagedSqliteDeleteFailure,
    ) -> ManagedSqliteShmFailure {
        let parts = failure.into_shm_parts();
        if let Some(close_error) = self.retain_handle_custody(state, parts.custody) {
            self.mark_poisoned(
                state,
                ManagedSqliteShmFailurePhase::FileClose,
                parts.mutation_may_have_occurred,
                false,
            );
            return ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::FileClose,
                close_error,
                parts.mutation_may_have_occurred,
                false,
            );
        }
        self.mark_poisoned(
            state,
            ManagedSqliteShmFailurePhase::ExactSiblingDelete,
            parts.mutation_may_have_occurred,
            false,
        );
        ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::ExactSiblingDelete,
            parts.error,
            parts.mutation_may_have_occurred,
            false,
        )
    }

    fn retain_handle_custody(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        custody: ManagedSqliteFailureHandleCustody,
    ) -> Option<io::Error> {
        let mut report = None;
        if let Some(file) = custody.live {
            if let Err(failure) = file.close() {
                report = Some(close_failure_report(&failure));
                state
                    .quarantined_file_close
                    .push(ManagedSqliteShmFileCloseCustody::Rejected(failure));
            }
        }
        if let Some(failure) = custody.close_failure {
            if report.is_none() {
                report = Some(close_failure_report(&failure));
            }
            state
                .quarantined_file_close
                .push(ManagedSqliteShmFileCloseCustody::Rejected(failure));
        }
        report
    }
}

fn close_failure_report(failure: &ManagedSqliteQuarantinedFileCloseFailure) -> io::Error {
    failure.raw_os_error().map_or_else(
        || {
            io::Error::new(
                failure.error_kind(),
                "NODE_MANAGED_SQLITE_SHM_REJECTED_HANDLE_CLOSE_FAILED",
            )
        },
        io::Error::from_raw_os_error,
    )
}
