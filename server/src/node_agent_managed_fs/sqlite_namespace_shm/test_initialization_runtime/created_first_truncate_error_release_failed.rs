//! Q16 event bridge for unread truncate and cleanup-release receipts.

use super::super::{
    coordinator::{ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState},
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
};
use super::ManagedSqliteShmTestInitializationNativeReceiptV1;

impl ManagedSqliteShmCoordinator {
    pub(super) fn begin_test_initialization_created_first_truncate_error_release_failed_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<bool, ManagedSqliteShmFailure> {
        let mut selected = false;
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
            |controller, target| {
                selected =
                    controller.begin_created_first_truncate_error_release_failed(target)?;
                Ok(())
            },
        )?;
        Ok(selected)
    }

    pub(super) fn record_test_initialization_created_first_truncate_error_release_failed_truncate_receipt_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
            |controller, target| {
                controller.record_created_first_truncate_error_release_failed_truncate_receipt(
                    target, native,
                )
            },
        )
    }

    pub(super) fn begin_test_initialization_created_first_truncate_error_release_failed_cleanup_unlock_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            false,
            |controller, target| {
                controller
                    .begin_created_first_truncate_error_release_failed_cleanup_unlock(target)
            },
        )
    }

    pub(super) fn record_test_initialization_created_first_truncate_error_release_failed_cleanup_receipt_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            true,
            |controller, target| {
                controller.record_created_first_truncate_error_release_failed_cleanup_receipt(
                    target, native,
                )
            },
        )
    }

    pub(super) fn record_test_initialization_created_first_truncate_error_release_failed_poisoned_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            true,
            |controller, target| controller.record_poisoned(target),
        )
    }
}
