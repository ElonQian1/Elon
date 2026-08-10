use std::io;

use super::super::{platform, ManagedSqliteFileCloseFailure};
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        ManagedSqliteShmFileCloseCustody, ManagedSqliteShmNode,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, SHM_DMS_OFFSET},
};

pub(super) fn teardown_and_close_live_node(
    coordinator: &ManagedSqliteShmCoordinator,
    state: &mut ManagedSqliteShmCoordinatorState,
    connection_id: u64,
) -> Result<(), ManagedSqliteShmFailure> {
    let whole_teardown_known_mutation = teardown_live_node(coordinator, state, connection_id)?;
    #[cfg(test)]
    let test_fault = coordinator.observe_test_fault(
        connection_id,
        ManagedSqliteShmFailurePhase::FileClose,
        whole_teardown_known_mutation,
    )?;
    #[cfg(test)]
    if let Some(fault) = test_fault.filter(|fault| fault.is_before_call()) {
        let failure =
            coordinator.trigger_before_test_fault(fault, whole_teardown_known_mutation)?;
        return Err(failure);
    }
    let node = state.node.take().ok_or_else(|| {
        ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::ConnectionDetach,
            io::Error::other("NODE_MANAGED_SQLITE_SHM_NODE_DISAPPEARED_DURING_TEARDOWN"),
            true,
            false,
        )
    })?;
    let ManagedSqliteShmNode {
        regions,
        file,
        dms: _,
        initialization_mutated,
        region_size: _,
        mapped_bytes: _,
    } = node;
    drop(regions);
    match file.close() {
        Ok(receipt) => {
            let _kind = receipt.kind();
            #[cfg(test)]
            {
                if let Some(fault) = test_fault {
                    // The real receipt is consumed here and never exposed as a successful joint
                    // close. The outer coordinator makes every post-success fault terminal.
                    let failure = coordinator.trigger_after_test_fault(fault, true)?;
                    return Err(failure);
                }
            }
            Ok(())
        }
        Err(failure) => {
            let error = close_failure_report(&failure);
            state
                .quarantined_file_close
                .push(ManagedSqliteShmFileCloseCustody::Pinned(failure));
            Err(ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::FileClose,
                error,
                initialization_mutated || whole_teardown_known_mutation,
                false,
            ))
        }
    }
}

fn close_failure_report(failure: &ManagedSqliteFileCloseFailure) -> io::Error {
    failure.raw_os_error().map_or_else(
        || {
            io::Error::new(
                failure.error_kind(),
                "NODE_MANAGED_SQLITE_SHM_FILE_CLOSE_FAILED",
            )
        },
        io::Error::from_raw_os_error,
    )
}

fn teardown_live_node(
    _coordinator: &ManagedSqliteShmCoordinator,
    state: &mut ManagedSqliteShmCoordinatorState,
    _connection_id: u64,
) -> Result<bool, ManagedSqliteShmFailure> {
    let mut whole_teardown_known_mutation = false;
    let node = state.node.as_mut().ok_or_else(|| {
        ManagedSqliteShmFailure::poisoned(
            ManagedSqliteShmFailurePhase::ConnectionDetach,
            io::Error::new(
                io::ErrorKind::NotFound,
                "NODE_MANAGED_SQLITE_SHM_TEARDOWN_NODE_MISSING",
            ),
            false,
            false,
        )
    })?;

    for region in node.regions.iter_mut().rev() {
        if let Some(view) = region.view.as_mut() {
            #[cfg(test)]
            let test_fault = _coordinator.observe_test_fault(
                _connection_id,
                ManagedSqliteShmFailurePhase::ViewUnmap,
                whole_teardown_known_mutation,
            )?;
            #[cfg(test)]
            if let Some(fault) = test_fault.filter(|fault| fault.is_before_call()) {
                let failure =
                    _coordinator.trigger_before_test_fault(fault, whole_teardown_known_mutation)?;
                return Err(failure);
            }
            view.unmap_explicit().map_err(|error| {
                ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::ViewUnmap,
                    error,
                    true,
                    false,
                )
            })?;
            region.view = None;
            region.logical_pointer = None;
            whole_teardown_known_mutation = true;
            #[cfg(test)]
            {
                if let Some(fault) = test_fault {
                    let failure = _coordinator
                        .trigger_after_test_fault(fault, whole_teardown_known_mutation)?;
                    return Err(failure);
                }
            }
        }
        #[cfg(test)]
        let test_fault = _coordinator.observe_test_fault(
            _connection_id,
            ManagedSqliteShmFailurePhase::MappingClose,
            whole_teardown_known_mutation,
        )?;
        #[cfg(test)]
        if let Some(fault) = test_fault.filter(|fault| fault.is_before_call()) {
            let failure =
                _coordinator.trigger_before_test_fault(fault, whole_teardown_known_mutation)?;
            return Err(failure);
        }
        region.mapping.close_explicit().map_err(|error| {
            ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::MappingClose,
                error,
                true,
                false,
            )
        })?;
        whole_teardown_known_mutation = true;
        #[cfg(test)]
        {
            if let Some(fault) = test_fault {
                let failure =
                    _coordinator.trigger_after_test_fault(fault, whole_teardown_known_mutation)?;
                return Err(failure);
            }
        }
    }

    match node.dms {
        ManagedSqliteShmDmsCustody::Shared => {
            #[cfg(test)]
            let test_fault = _coordinator.observe_test_fault(
                _connection_id,
                ManagedSqliteShmFailurePhase::DmsSharedRelease,
                whole_teardown_known_mutation,
            )?;
            #[cfg(test)]
            if let Some(fault) = test_fault.filter(|fault| fault.is_before_call()) {
                let failure =
                    _coordinator.trigger_before_test_fault(fault, whole_teardown_known_mutation)?;
                return Err(failure);
            }
            platform::unlock_sqlite_byte_range(&node.file.file, SHM_DMS_OFFSET, 1).map_err(
                |error| {
                    ManagedSqliteShmFailure::poisoned(
                        ManagedSqliteShmFailurePhase::DmsSharedRelease,
                        error,
                        whole_teardown_known_mutation,
                        true,
                    )
                },
            )?;
            node.dms = ManagedSqliteShmDmsCustody::Released;
            whole_teardown_known_mutation = true;
            #[cfg(test)]
            {
                if let Some(fault) = test_fault {
                    let failure = _coordinator
                        .trigger_after_test_fault(fault, whole_teardown_known_mutation)?;
                    return Err(failure);
                }
            }
        }
        ManagedSqliteShmDmsCustody::ExclusiveKnown => {
            return Err(ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_DMS_EXCLUSIVE_RETAINED"),
                true,
                false,
            ));
        }
        ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain => {
            return Err(ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_DMS_EXCLUSIVE_UNCERTAIN"),
                true,
                true,
            ));
        }
        ManagedSqliteShmDmsCustody::Released => {}
    }
    Ok(whole_teardown_known_mutation)
}
