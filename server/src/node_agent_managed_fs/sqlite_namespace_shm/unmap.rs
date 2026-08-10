use super::super::ManagedSqliteObservedLock;
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        PinnedManagedSqliteShmConnection, PinnedManagedSqliteWalMainFile,
    },
    teardown::teardown_and_close_live_node,
    types::{
        ManagedSqliteShmDeleteDisposition, ManagedSqliteShmFailure, ManagedSqliteShmFailureClass,
        ManagedSqliteShmFailurePhase, ManagedSqliteShmUnmapFailure, ManagedSqliteShmUnmapMode,
        ManagedSqliteWalMainUnmapFailure,
    },
    ManagedSqliteDeleteOutcome,
};

struct ManagedSqliteShmInnerUnmapFailure {
    failure: ManagedSqliteShmFailure,
    connection_detached: bool,
}

impl ManagedSqliteShmInnerUnmapFailure {
    fn attached(failure: ManagedSqliteShmFailure) -> Self {
        Self {
            failure,
            connection_detached: false,
        }
    }

    fn detached(failure: ManagedSqliteShmFailure) -> Self {
        Self {
            failure,
            connection_detached: true,
        }
    }
}

impl From<ManagedSqliteShmFailure> for ManagedSqliteShmInnerUnmapFailure {
    fn from(failure: ManagedSqliteShmFailure) -> Self {
        Self::attached(failure)
    }
}

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
            Err(failure) => {
                if failure.connection_detached {
                    self.active = false;
                }
                Err(ManagedSqliteShmUnmapFailure {
                    failure: failure.failure,
                    connection: self,
                })
            }
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
    fn unmap_connection(
        &self,
        connection_id: u64,
        delete: ManagedSqliteShmDeleteDisposition<'_>,
    ) -> Result<(), ManagedSqliteShmInnerUnmapFailure> {
        let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;
        if let Some(poison) = state.poisoned {
            return Err(poison.failure().into());
        }
        let held = state
            .connections
            .get(&connection_id)
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED"))?;
        if held.shared_mask != 0
            || held.exclusive_mask != 0
            || held.exclusive_ranges.iter().any(|count| *count != 0)
        {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_UNMAP_WITH_HELD_LOCKS").into());
        }

        if state.connections.len() > 1 {
            // SQLite's delete flag applies only when this is the final local SHM reference.
            #[cfg(test)]
            let detach_fault = self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::ConnectionDetach,
                false,
            )?;
            self.detach_connection(&mut state, connection_id)?;
            #[cfg(test)]
            if let Some(failure) = self.finish_test_fault(&mut state, detach_fault, true) {
                return Err(ManagedSqliteShmInnerUnmapFailure::detached(failure));
            }
            return Ok(());
        }

        self.validate_delete_authority(&mut state, &delete)?;

        let mut prior_unmap_mutation = false;
        if state.node.is_some() {
            if let Err(failure) = teardown_and_close_live_node(self, &mut state, connection_id) {
                // Any post-success or uncertain teardown failure retains terminal custody. A
                // mutation-free before-call fault remains retryable with the connection attached.
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
                return Err(failure.into());
            }
            // A successful joint teardown has consumed at least the SHM file-close receipt.
            prior_unmap_mutation = true;
        }

        if matches!(delete, ManagedSqliteShmDeleteDisposition::Delete { .. }) {
            #[cfg(test)]
            let delete_fault = self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                prior_unmap_mutation,
            )?;
            let outcome = match self.namespace.delete_shm_for_wal() {
                Ok(outcome) => outcome,
                Err(failure) => {
                    return Err(self
                        .consume_delete_failure(&mut state, failure, prior_unmap_mutation)
                        .into());
                }
            };
            let deleted = outcome == ManagedSqliteDeleteOutcome::Deleted;
            #[cfg(test)]
            {
                prior_unmap_mutation |= deleted;
                if deleted {
                    if let Some(failure) =
                        self.finish_test_fault(&mut state, delete_fault, prior_unmap_mutation)
                    {
                        return Err(failure.into());
                    }
                }
            }
            #[cfg(not(test))]
            let _ = deleted;
        }

        #[cfg(test)]
        let detach_fault = self.begin_test_fault(
            &mut state,
            connection_id,
            ManagedSqliteShmFailurePhase::ConnectionDetach,
            prior_unmap_mutation,
        )?;
        self.detach_connection(&mut state, connection_id)?;
        #[cfg(test)]
        if let Some(failure) = self.finish_test_fault(&mut state, detach_fault, true) {
            return Err(ManagedSqliteShmInnerUnmapFailure::detached(failure));
        }
        Ok(())
    }

    fn detach_connection(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmInnerUnmapFailure> {
        if state.connections.remove(&connection_id).is_some() {
            return Ok(());
        }
        self.mark_poisoned(
            state,
            ManagedSqliteShmFailurePhase::ConnectionDetach,
            true,
            false,
        );
        Err(ManagedSqliteShmInnerUnmapFailure::detached(
            ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::ConnectionDetach,
                "NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED_DURING_DETACH",
                true,
                false,
            ),
        ))
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
            // Retrying uncertain teardown from Drop could double-release a kernel object.
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

fn protocol(code: &'static str) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::code(
        ManagedSqliteShmFailurePhase::RequestValidation,
        ManagedSqliteShmFailureClass::ProtocolViolation,
        code,
    )
}
