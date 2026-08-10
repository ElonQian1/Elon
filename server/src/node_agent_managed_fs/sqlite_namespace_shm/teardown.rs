use std::io;

use super::super::{platform, ManagedSqliteFileCloseFailure, ManagedSqliteObservedLock};
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        ManagedSqliteShmFileCloseCustody, ManagedSqliteShmNode, PinnedManagedSqliteShmConnection,
        PinnedManagedSqliteWalMainFile,
    },
    types::{
        ManagedSqliteShmDeleteDisposition, ManagedSqliteShmFailure, ManagedSqliteShmFailureClass,
        ManagedSqliteShmFailurePhase, ManagedSqliteShmUnmapFailure, ManagedSqliteShmUnmapMode,
        ManagedSqliteWalMainUnmapFailure, SHM_DMS_OFFSET,
    },
};

impl PinnedManagedSqliteShmConnection {
    fn unmap(
        mut self,
        delete: ManagedSqliteShmDeleteDisposition<'_>,
    ) -> Result<(), ManagedSqliteShmUnmapFailure> {
        match self
            .coordinator
            .unmap_connection(self.connection_id, delete)
        {
            Ok(()) => {
                self.active = false;
                Ok(())
            }
            Err(failure) => Err(ManagedSqliteShmUnmapFailure {
                failure,
                connection: self,
            }),
        }
    }
}

impl PinnedManagedSqliteWalMainFile {
    pub(crate) fn unmap_shm(
        self,
        mode: ManagedSqliteShmUnmapMode,
    ) -> Result<Self, ManagedSqliteWalMainUnmapFailure> {
        let Self {
            shm,
            main,
            runtime_generation,
        } = self;
        let Some(connection) = shm else {
            return Err(ManagedSqliteWalMainUnmapFailure {
                failure: protocol("NODE_MANAGED_SQLITE_SHM_UNMAP_CONNECTION_MISSING"),
                wal_main: Self {
                    shm: None,
                    main,
                    runtime_generation,
                },
            });
        };

        let unmap = match mode {
            ManagedSqliteShmUnmapMode::Keep => {
                connection.unmap(ManagedSqliteShmDeleteDisposition::Keep)
            }
            ManagedSqliteShmUnmapMode::Delete => {
                connection.unmap(ManagedSqliteShmDeleteDisposition::Delete {
                    main: &main,
                    runtime_generation,
                })
            }
        };

        match unmap {
            Ok(()) => Ok(Self {
                shm: None,
                main,
                runtime_generation,
            }),
            Err(ManagedSqliteShmUnmapFailure {
                failure,
                connection,
            }) => Err(ManagedSqliteWalMainUnmapFailure {
                failure,
                wal_main: Self {
                    shm: Some(connection),
                    main,
                    runtime_generation,
                },
            }),
        }
    }
}

impl ManagedSqliteShmCoordinator {
    pub(super) fn unmap_connection(
        &self,
        connection_id: u64,
        delete: ManagedSqliteShmDeleteDisposition<'_>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        let held = state
            .connections
            .get(&connection_id)
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED"))?;
        if held.shared_mask != 0
            || held.exclusive_mask != 0
            || held.exclusive_ranges.iter().any(|count| *count != 0)
        {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_UNMAP_WITH_HELD_LOCKS"));
        }

        if state.connections.len() > 1 {
            // SQLite's delete flag applies only when this is the final local SHM reference.
            if state.connections.remove(&connection_id).is_none() {
                return Err(protocol(
                    "NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED_DURING_UNMAP",
                ));
            }
            return Ok(());
        }

        self.validate_delete_authority(&mut state, &delete)?;

        let mut prior_teardown_mutation = false;
        if state.node.is_some() {
            if let Err(failure) = teardown_and_close_live_node(self, &mut state, connection_id) {
                // Any injected after-success failure has already consumed a platform mutation.
                // Keep the whole domain terminal so a later unmap cannot retry a closed mapping,
                // released DMS byte or consumed file-close receipt as if it were a fresh action.
                if failure.class() == ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
                    || failure.mutation_may_have_occurred()
                {
                    self.mark_poisoned(
                        &mut state,
                        failure.phase(),
                        failure.mutation_may_have_occurred(),
                        failure.lock_outcome_uncertain(),
                    );
                }
                return Err(failure);
            }
            // A successful joint teardown has consumed at least the SHM file-close receipt.
            prior_teardown_mutation = true;
        }

        if matches!(delete, ManagedSqliteShmDeleteDisposition::Delete { .. }) {
            if let Err(failure) = self.namespace.delete_shm_for_wal() {
                return Err(self.consume_delete_failure(
                    &mut state,
                    failure,
                    prior_teardown_mutation,
                ));
            }
        }

        if state.connections.remove(&connection_id).is_none() {
            self.mark_poisoned(
                &mut state,
                ManagedSqliteShmFailurePhase::ConnectionDetach,
                true,
                false,
            );
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::ConnectionDetach,
                "NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED_DURING_DETACH",
                true,
                false,
            ));
        }
        Ok(())
    }

    fn validate_delete_authority(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        delete: &ManagedSqliteShmDeleteDisposition<'_>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let ManagedSqliteShmDeleteDisposition::Delete {
            main,
            runtime_generation,
        } = delete
        else {
            return Ok(());
        };
        let Some(main_identity_digest) = state.main_identity_digest.as_deref() else {
            return Err(ManagedSqliteShmFailure::code(
                ManagedSqliteShmFailurePhase::DeleteAuthorization,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_MAIN_IDENTITY_MISSING",
            ));
        };
        if *runtime_generation != self.generation || main.identity_digest() != main_identity_digest
        {
            return Err(ManagedSqliteShmFailure::code(
                ManagedSqliteShmFailurePhase::DeleteAuthorization,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_AUTHORITY_MISMATCH",
            ));
        }
        let lock_level = match main.lock_level() {
            Ok(lock_level) => lock_level,
            Err(_) => {
                self.mark_poisoned(
                    state,
                    ManagedSqliteShmFailurePhase::DeleteAuthorization,
                    false,
                    true,
                );
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::DeleteAuthorization,
                    "NODE_MANAGED_SQLITE_SHM_MAIN_LOCK_STATE_UNAVAILABLE",
                    false,
                    true,
                ));
            }
        };
        if lock_level != ManagedSqliteObservedLock::Exclusive {
            return Err(ManagedSqliteShmFailure::code(
                ManagedSqliteShmFailurePhase::DeleteAuthorization,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_REQUIRES_MAIN_EXCLUSIVE",
            ));
        }
        Ok(())
    }

    pub(super) fn best_effort_drop_connection(&self, connection_id: u64) {
        let Ok(mut state) = self.state.lock() else {
            self.mark_domain_terminal();
            return;
        };
        if state.poisoned.is_some() {
            // A prior explicit release reported an uncertain OS outcome. Retrying from Drop could
            // double-release an object whose kernel state is unknown, so retain the tombstone.
            return;
        }
        let Some(held) = state.connections.get(&connection_id) else {
            return;
        };
        if held.shared_mask != 0
            || held.exclusive_mask != 0
            || held.exclusive_ranges.iter().any(|count| *count != 0)
        {
            self.mark_poisoned(
                &mut state,
                ManagedSqliteShmFailurePhase::ConnectionDetach,
                false,
                true,
            );
            return;
        }
        if state.connections.len() > 1 || state.node.is_none() {
            state.connections.remove(&connection_id);
            return;
        }
        if let Err(failure) = teardown_and_close_live_node(self, &mut state, connection_id) {
            self.mark_poisoned(
                &mut state,
                failure.phase(),
                failure.mutation_may_have_occurred(),
                failure.lock_outcome_uncertain(),
            );
            return;
        }
        state.connections.remove(&connection_id);
    }
}

fn teardown_and_close_live_node(
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
        let failure = coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
                    let failure = coordinator.activate_test_fault(fault, true)?;
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
                    _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
                    let failure =
                        _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
            let failure = _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
                    _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
                    _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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
                    let failure =
                        _coordinator.activate_test_fault(fault, whole_teardown_known_mutation)?;
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

fn protocol(code: &'static str) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::code(
        ManagedSqliteShmFailurePhase::RequestValidation,
        ManagedSqliteShmFailureClass::ProtocolViolation,
        code,
    )
}
