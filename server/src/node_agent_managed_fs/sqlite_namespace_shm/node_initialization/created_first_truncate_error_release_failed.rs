//! Q16 production initialization branch with unread truncate and cleanup-release receipts.

use super::super::super::{platform, PinnedManagedSqliteFile};
use super::super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
    },
    test_initialization_runtime::{
        ManagedSqliteShmTestInitializationNativeObservationV1,
        ManagedSqliteShmTestInitializationNativeReceiptV1,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, SHM_DMS_OFFSET},
};
use super::new_node;

impl ManagedSqliteShmCoordinator {
    pub(super) fn execute_q16_truncate_release_failed_test_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        mut file: PinnedManagedSqliteFile,
    ) -> Result<PinnedManagedSqliteFile, ManagedSqliteShmFailure> {
        if !self.begin_test_initialization_created_first_truncate_error_release_failed_v1(
            state,
            connection_id,
        )? {
            return Ok(file);
        }

        let truncate_receipt = match file.truncate_outcome_unavailable_for_initialization_test_v1(0)
        {
            Ok(receipt) if receipt.native_attempts() == 1 => receipt,
            Ok(_) | Err(_) => {
                return Err(self.retain_after_q16_controller_rejection(
                    state,
                    connection_id,
                    file,
                    ManagedSqliteShmDmsCustody::ExclusiveKnown,
                    ManagedSqliteShmFailurePhase::DmsTruncate,
                    false,
                    "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q16_TRUNCATE_SEAM_FAILED",
                ));
            }
        };
        let truncate_native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
            observation:
                ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
            offset: truncate_receipt.requested_size(),
            length: 0,
            exact_call_occurrence: truncate_receipt.exact_call_occurrence(),
        };
        if let Err(failure) = self
            .record_test_initialization_created_first_truncate_error_release_failed_truncate_receipt_v1(
                state,
                connection_id,
                truncate_native,
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
            .begin_test_initialization_created_first_truncate_error_release_failed_cleanup_unlock_v1(
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

        let release_receipt =
            platform::unlock_sqlite_byte_range_outcome_uncertain_for_initialization_test(
                &file.file,
                SHM_DMS_OFFSET,
                1,
            );
        let release_native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
            observation:
                ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
            offset: release_receipt.offset,
            length: release_receipt.length,
            exact_call_occurrence: release_receipt.exact_call_occurrence.get(),
        };
        if let Err(failure) = self
            .record_test_initialization_created_first_truncate_error_release_failed_cleanup_receipt_v1(
                state,
                connection_id,
                release_native,
            )
        {
            state.node = Some(new_node(
                file,
                ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
                true,
            ));
            return Err(failure);
        }

        let release_error = release_receipt.error;
        state.node = Some(new_node(
            file,
            ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
            true,
        ));
        self.mark_poisoned(
            state,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            true,
        );
        self.record_test_initialization_created_first_truncate_error_release_failed_poisoned_v1(
            state,
            connection_id,
        )?;
        Err(ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            release_error,
            true,
            true,
        ))
    }

    fn retain_after_q16_controller_rejection(
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
                ManagedSqliteShmFailure::poisoned_code(phase, code, true, lock_outcome_uncertain)
            }
        }
    }
}
