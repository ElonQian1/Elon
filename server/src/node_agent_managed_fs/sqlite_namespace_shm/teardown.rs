use std::io;

use super::super::{platform, ManagedSqliteObservedLock};
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        PinnedManagedSqliteShmConnection, PinnedManagedSqliteWalMainFile,
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

        if state.node.is_some() {
            if let Err((phase, error, mutation, lock_uncertain)) = teardown_live_node(&mut state) {
                self.mark_poisoned(&mut state, phase, mutation, lock_uncertain);
                return Err(ManagedSqliteShmFailure::poisoned(
                    phase,
                    error,
                    mutation,
                    lock_uncertain,
                ));
            }
            let Some(node) = state.node.take() else {
                self.mark_poisoned(
                    &mut state,
                    ManagedSqliteShmFailurePhase::ConnectionDetach,
                    true,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::ConnectionDetach,
                    "NODE_MANAGED_SQLITE_SHM_NODE_DISAPPEARED_DURING_TEARDOWN",
                    true,
                    false,
                ));
            };
            drop(node);
        }

        if matches!(delete, ManagedSqliteShmDeleteDisposition::Delete { .. }) {
            if let Err(failure) = self.namespace.delete_shm_for_wal() {
                let mutation = failure.mutation_may_have_occurred();
                self.mark_poisoned(
                    &mut state,
                    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                    mutation,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                    io::Error::other(failure),
                    mutation,
                    false,
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
        if let Err((phase, _error, mutation, lock_uncertain)) = teardown_live_node(&mut state) {
            self.mark_poisoned(&mut state, phase, mutation, lock_uncertain);
            return;
        }
        if let Some(node) = state.node.take() {
            drop(node);
        }
        state.connections.remove(&connection_id);
    }
}

fn teardown_live_node(
    state: &mut ManagedSqliteShmCoordinatorState,
) -> Result<(), (ManagedSqliteShmFailurePhase, io::Error, bool, bool)> {
    let node = state.node.as_mut().ok_or_else(|| {
        (
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
            view.unmap_explicit()
                .map_err(|error| (ManagedSqliteShmFailurePhase::ViewUnmap, error, true, false))?;
            region.view = None;
            region.logical_pointer = None;
        }
        region.mapping.close_explicit().map_err(|error| {
            (
                ManagedSqliteShmFailurePhase::MappingClose,
                error,
                true,
                false,
            )
        })?;
    }

    match node.dms {
        ManagedSqliteShmDmsCustody::Shared => {
            platform::unlock_sqlite_byte_range(&node.file.file, SHM_DMS_OFFSET, 1).map_err(
                |error| {
                    (
                        ManagedSqliteShmFailurePhase::DmsSharedRelease,
                        error,
                        false,
                        true,
                    )
                },
            )?;
            node.dms = ManagedSqliteShmDmsCustody::Released;
        }
        ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain => {
            return Err((
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_DMS_EXCLUSIVE_UNCERTAIN"),
                true,
                true,
            ));
        }
        ManagedSqliteShmDmsCustody::Released => {}
    }
    Ok(())
}

fn protocol(code: &'static str) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::code(
        ManagedSqliteShmFailurePhase::RequestValidation,
        ManagedSqliteShmFailureClass::ProtocolViolation,
        code,
    )
}
