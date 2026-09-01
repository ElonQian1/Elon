//! Q14 production initialization branch: unread truncate receipt, then known cleanup release.

use std::io;

use super::super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        ManagedSqliteShmDmsCustody,
    },
    test_initialization_runtime::{
        ManagedSqliteShmTestInitializationNativeObservationV1,
        ManagedSqliteShmTestInitializationNativeReceiptV1,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, SHM_DMS_OFFSET},
};
use super::super::super::{platform, PinnedManagedSqliteFile};
use super::new_node;

impl ManagedSqliteShmCoordinator {
    pub(super) fn execute_q14_truncate_release_ok_test_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        mut file: PinnedManagedSqliteFile,
    ) -> Result<PinnedManagedSqliteFile, ManagedSqliteShmFailure> {
        if !self.begin_test_initialization_created_first_truncate_outcome_unavailable_v1(
            state,
            connection_id,
        )? {
            return Ok(file);
        }

        let platform_receipt =
            match file.truncate_outcome_unavailable_for_initialization_test_v1(0) {
                Ok(receipt) if receipt.native_attempts() == 1 => receipt,
                Ok(_) | Err(_) => {
                    return Err(self.retain_after_q14_controller_rejection(
                        state,
                        connection_id,
                        file,
                        ManagedSqliteShmDmsCustody::ExclusiveKnown,
                        ManagedSqliteShmFailurePhase::DmsTruncate,
                        false,
                        "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q14_TRUNCATE_SEAM_FAILED",
                    ));
                }
            };
        let native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
            observation:
                ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
            offset: platform_receipt.requested_size(),
            length: 0,
            exact_call_occurrence: platform_receipt.exact_call_occurrence(),
        };
        if let Err(failure) = self
            .record_test_initialization_created_first_truncate_receipt_unavailable_v1(
                state,
                connection_id,
                native,
            )
        {
            state.node = Some(new_node(
                file,
                ManagedSqliteShmDmsCustody::ExclusiveKnown,
                true,
            ));
            return Err(failure);
        }
        if let Err(failure) = self
            .begin_test_initialization_created_first_truncate_cleanup_unlock_v1(
                state,
                connection_id,
            )
        {
            state.node = Some(new_node(
                file,
                ManagedSqliteShmDmsCustody::ExclusiveKnown,
                true,
            ));
            return Err(failure);
        }

        if platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1).is_err() {
            return Err(self.retain_after_q14_controller_rejection(
                state,
                connection_id,
                file,
                ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                true,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q14_CLEANUP_UNLOCK_FAILED",
            ));
        }
        if let Err(failure) = self
            .record_test_initialization_created_first_truncate_cleanup_unlock_succeeded_v1(
                state,
                connection_id,
            )
        {
            state.node = Some(new_node(
                file,
                ManagedSqliteShmDmsCustody::Released,
                true,
            ));
            return Err(failure);
        }

        state.node = Some(new_node(
            file,
            ManagedSqliteShmDmsCustody::Released,
            true,
        ));
        self.mark_poisoned(
            state,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
        );
        self.record_test_initialization_created_first_truncate_poisoned_v1(
            state,
            connection_id,
        )?;
        Err(ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::DmsTruncate,
            io::Error::other(
                "NODE_MANAGED_SQLITE_SHM_TRUNCATE_RETURN_RECEIPT_UNAVAILABLE",
            ),
            true,
            false,
        ))
    }

    fn retain_after_q14_controller_rejection(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        file: PinnedManagedSqliteFile,
        dms: ManagedSqliteShmDmsCustody,
        phase: ManagedSqliteShmFailurePhase,
        lock_outcome_uncertain: bool,
        code: &'static str,
    ) -> ManagedSqliteShmFailure {
        let controller_failure = self.reject_test_initialization_path_v1(
            state,
            connection_id,
            phase,
            true,
            lock_outcome_uncertain,
            code,
        );
        state.node = Some(new_node(file, dms, true));
        match controller_failure {
            Err(failure) => failure,
            Ok(()) => {
                self.mark_poisoned(state, phase, true, lock_outcome_uncertain);
                ManagedSqliteShmFailure::poisoned_code(
                    phase,
                    code,
                    true,
                    lock_outcome_uncertain,
                )
            }
        }
    }
}
