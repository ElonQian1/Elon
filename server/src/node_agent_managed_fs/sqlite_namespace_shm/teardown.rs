use std::io;

#[cfg(all(test, windows))]
use super::super::ManagedSqliteFileCloseReceipt;
use super::super::{platform, ManagedSqliteFileCloseFailure};
#[cfg(all(test, windows))]
use super::test_unmap_runtime::{
    ManagedSqliteShmTestUnmapNativeObservation, ManagedSqliteShmTestUnmapNativeOperation,
};
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
    #[cfg(all(test, windows))]
    let test_native = coordinator.begin_test_unmap_action(
        connection_id,
        ManagedSqliteShmFailurePhase::FileClose,
        whole_teardown_known_mutation,
    )?;
    #[cfg(all(test, windows))]
    let close = match test_native {
        Some(operation @ ManagedSqliteShmTestUnmapNativeOperation::FileCloseRetryable) => {
            coordinator.trigger_test_unmap_native(
                connection_id,
                operation,
                whole_teardown_known_mutation,
            )?;
            let native = file.close_for_unmap_test_native(
                platform::PlatformManagedSqliteCloseTestNative::Retryable,
            );
            witness_test_native_file_close(
                coordinator,
                state,
                connection_id,
                operation,
                native.observation,
                native.result,
            )?
        }
        Some(operation @ ManagedSqliteShmTestUnmapNativeOperation::FileCloseOutcomeUncertain) => {
            coordinator.trigger_test_unmap_native(
                connection_id,
                operation,
                whole_teardown_known_mutation,
            )?;
            let native = file.close_for_unmap_test_native(
                platform::PlatformManagedSqliteCloseTestNative::OutcomeUncertain,
            );
            witness_test_native_file_close(
                coordinator,
                state,
                connection_id,
                operation,
                native.observation,
                native.result,
            )?
        }
        Some(_) => {
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::FileClose,
                "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_FILE_NATIVE_INVALID",
                whole_teardown_known_mutation,
                false,
            ));
        }
        None => file.close(),
    };
    #[cfg(not(all(test, windows)))]
    let close = file.close();
    match close {
        Ok(receipt) => {
            let _kind = receipt.kind();
            #[cfg(all(test, windows))]
            coordinator.finish_test_unmap_action(
                connection_id,
                ManagedSqliteShmFailurePhase::FileClose,
                true,
            )?;
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

#[cfg(all(test, windows))]
fn witness_test_native_file_close(
    coordinator: &ManagedSqliteShmCoordinator,
    state: &mut ManagedSqliteShmCoordinatorState,
    connection_id: u64,
    operation: ManagedSqliteShmTestUnmapNativeOperation,
    observation: Option<ManagedSqliteShmTestUnmapNativeObservation>,
    close: Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure>,
) -> Result<
    Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure>,
    ManagedSqliteShmFailure,
> {
    let Some(observation) = observation else {
        return Ok(close);
    };
    if let Err(witness_failure) =
        coordinator.witness_test_unmap_native(connection_id, operation, observation, true)
    {
        match close {
            Err(close_failure) => state
                .quarantined_file_close
                .push(ManagedSqliteShmFileCloseCustody::Pinned(close_failure)),
            Ok(receipt) => {
                let _kind = receipt.kind();
            }
        }
        return Err(witness_failure);
    }
    Ok(close)
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
            #[cfg(all(test, windows))]
            let test_native = _coordinator.begin_test_unmap_action(
                _connection_id,
                ManagedSqliteShmFailurePhase::ViewUnmap,
                whole_teardown_known_mutation,
            )?;
            #[cfg(all(test, windows))]
            let unmap = match test_native {
                Some(
                    operation @ ManagedSqliteShmTestUnmapNativeOperation::ViewUnmapOutcomeUncertain,
                ) => {
                    _coordinator.trigger_test_unmap_native(
                        _connection_id,
                        operation,
                        whole_teardown_known_mutation,
                    )?;
                    let (unmap, observation) = view.unmap_explicit_outcome_uncertain_for_test();
                    if let Some(observation) = observation {
                        _coordinator.witness_test_unmap_native(
                            _connection_id,
                            operation,
                            observation,
                            true,
                        )?;
                    }
                    unmap
                }
                Some(_) => {
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        ManagedSqliteShmFailurePhase::ViewUnmap,
                        "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_VIEW_NATIVE_INVALID",
                        whole_teardown_known_mutation,
                        false,
                    ));
                }
                None => view.unmap_explicit(),
            };
            #[cfg(not(all(test, windows)))]
            let unmap = view.unmap_explicit();
            unmap.map_err(|error| {
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
            #[cfg(all(test, windows))]
            _coordinator.finish_test_unmap_action(
                _connection_id,
                ManagedSqliteShmFailurePhase::ViewUnmap,
                whole_teardown_known_mutation,
            )?;
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
        #[cfg(all(test, windows))]
        let test_native = _coordinator.begin_test_unmap_action(
            _connection_id,
            ManagedSqliteShmFailurePhase::MappingClose,
            whole_teardown_known_mutation,
        )?;
        #[cfg(all(test, windows))]
        let close = match test_native {
            Some(
                operation @ ManagedSqliteShmTestUnmapNativeOperation::MappingCloseOutcomeUncertain,
            ) => {
                _coordinator.trigger_test_unmap_native(
                    _connection_id,
                    operation,
                    whole_teardown_known_mutation,
                )?;
                let (close, observation) =
                    region.mapping.close_explicit_outcome_uncertain_for_test();
                if let Some(observation) = observation {
                    _coordinator.witness_test_unmap_native(
                        _connection_id,
                        operation,
                        observation,
                        true,
                    )?;
                }
                close
            }
            Some(_) => {
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::MappingClose,
                    "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_MAPPING_NATIVE_INVALID",
                    whole_teardown_known_mutation,
                    false,
                ));
            }
            None => region.mapping.close_explicit(),
        };
        #[cfg(not(all(test, windows)))]
        let close = region.mapping.close_explicit();
        close.map_err(|error| {
            ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::MappingClose,
                error,
                true,
                false,
            )
        })?;
        whole_teardown_known_mutation = true;
        #[cfg(all(test, windows))]
        _coordinator.finish_test_unmap_action(
            _connection_id,
            ManagedSqliteShmFailurePhase::MappingClose,
            whole_teardown_known_mutation,
        )?;
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
            #[cfg(all(test, windows))]
            let test_native = _coordinator.begin_test_unmap_action(
                _connection_id,
                ManagedSqliteShmFailurePhase::DmsSharedRelease,
                whole_teardown_known_mutation,
            )?;
            #[cfg(all(test, windows))]
            let release = match test_native {
                Some(
                    operation @ ManagedSqliteShmTestUnmapNativeOperation::DmsSharedReleaseOutcomeUncertain,
                ) => {
                    _coordinator.trigger_test_unmap_native(
                        _connection_id,
                        operation,
                        whole_teardown_known_mutation,
                    )?;
                    let (release, observation) =
                        platform::unlock_sqlite_byte_range_outcome_uncertain_for_test(
                        &node.file.file,
                        SHM_DMS_OFFSET,
                        1,
                    );
                    // The exact UnlockFileEx return was deliberately not observed. Seal terminal
                    // lock custody before the fallible evidence write, so a witness failure cannot
                    // make this range look safely Shared again.
                    node.dms = ManagedSqliteShmDmsCustody::SharedOutcomeUncertain;
                    if let Some(observation) = observation {
                        _coordinator.witness_test_unmap_native(
                            _connection_id,
                            operation,
                            observation,
                            true,
                        )?;
                    }
                    release
                }
                Some(_) => {
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        ManagedSqliteShmFailurePhase::DmsSharedRelease,
                        "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DMS_NATIVE_INVALID",
                        whole_teardown_known_mutation,
                        true,
                    ));
                }
                None => platform::unlock_sqlite_byte_range(&node.file.file, SHM_DMS_OFFSET, 1),
            };
            #[cfg(not(all(test, windows)))]
            let release = platform::unlock_sqlite_byte_range(&node.file.file, SHM_DMS_OFFSET, 1);
            if let Err(error) = release {
                node.dms = ManagedSqliteShmDmsCustody::SharedOutcomeUncertain;
                return Err(ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::DmsSharedRelease,
                    error,
                    true,
                    true,
                ));
            }
            node.dms = ManagedSqliteShmDmsCustody::Released;
            whole_teardown_known_mutation = true;
            #[cfg(all(test, windows))]
            _coordinator.finish_test_unmap_action(
                _connection_id,
                ManagedSqliteShmFailurePhase::DmsSharedRelease,
                whole_teardown_known_mutation,
            )?;
            #[cfg(test)]
            {
                if let Some(fault) = test_fault {
                    let failure = _coordinator
                        .trigger_after_test_fault(fault, whole_teardown_known_mutation)?;
                    return Err(failure);
                }
            }
        }
        ManagedSqliteShmDmsCustody::SharedOutcomeUncertain => {
            return Err(ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::DmsSharedRelease,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_DMS_SHARED_UNCERTAIN"),
                true,
                true,
            ));
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
