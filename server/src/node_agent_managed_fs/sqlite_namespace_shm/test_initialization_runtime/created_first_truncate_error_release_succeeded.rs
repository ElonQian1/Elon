//! Q14 event bridge from the production initialization path into the exact-target controller.

use super::super::{
    coordinator::{ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState},
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
};
use super::ManagedSqliteShmTestInitializationNativeReceiptV1;

impl ManagedSqliteShmCoordinator {
    pub(super) fn begin_test_initialization_created_first_truncate_outcome_unavailable_v1(
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
                selected = controller.begin_created_first_truncate_outcome_unavailable(target)?;
                Ok(())
            },
        )?;
        Ok(selected)
    }

    pub(super) fn record_test_initialization_created_first_truncate_receipt_unavailable_v1(
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
                controller
                    .record_created_first_truncate_return_receipt_unavailable(target, native)
            },
        )
    }

    pub(super) fn begin_test_initialization_created_first_truncate_cleanup_unlock_v1(
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
                controller.begin_created_first_truncate_cleanup_unlock(target)
            },
        )
    }

    pub(super) fn record_test_initialization_created_first_truncate_cleanup_unlock_succeeded_v1(
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
                controller.record_created_first_truncate_cleanup_unlock_succeeded(target)
            },
        )
    }

    pub(super) fn record_test_initialization_created_first_truncate_poisoned_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
            |controller, target| controller.record_poisoned(target),
        )
    }
}
