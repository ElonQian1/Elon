use super::super::ManagedSqliteObservedLock;
#[cfg(all(test, windows))]
use super::test_unmap_runtime::{
    ManagedSqliteShmTestUnmapDeleteAuthorityReceipt, ManagedSqliteShmTestUnmapDeletePrestate,
    ManagedSqliteShmTestUnmapNativeOperation,
};
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
    #[cfg(test)]
    pub(crate) fn unmap_shm_connection_active_for_test(&self) -> bool {
        self.shm
            .as_ref()
            .is_some_and(|connection| connection.active)
    }

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
            #[cfg(all(test, windows))]
            self.begin_test_connection_detach_action(connection_id, false)?;
            self.detach_connection(&mut state, connection_id)?;
            #[cfg(all(test, windows))]
            if let Err(failure) = self.finish_test_connection_detach_action(connection_id, true) {
                return Err(ManagedSqliteShmInnerUnmapFailure::detached(failure));
            }
            #[cfg(test)]
            if let Some(failure) = self.finish_test_fault(&mut state, detach_fault, true) {
                return Err(ManagedSqliteShmInnerUnmapFailure::detached(failure));
            }
            return Ok(());
        }

        self.validate_delete_authority(&mut state, connection_id, &delete)?;

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
            #[cfg(all(test, windows))]
            if let Some(prestate @ ManagedSqliteShmTestUnmapDeletePrestate::NotFound) =
                self.take_test_unmap_not_found_prestate(connection_id)?
            {
                match self.namespace.delete_shm_for_wal() {
                    Ok(outcome) => self.record_test_unmap_prestate_setup_delete(
                        connection_id,
                        prestate,
                        outcome,
                    )?,
                    Err(failure) => {
                        return Err(self
                            .consume_delete_failure(&mut state, failure, true)
                            .into());
                    }
                }
            }
            #[cfg(test)]
            let delete_fault = self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                prior_unmap_mutation,
            )?;
            #[cfg(all(test, windows))]
            let test_native = self.begin_test_unmap_action(
                connection_id,
                ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                prior_unmap_mutation,
            )?;
            #[cfg(all(test, windows))]
            let delete_result = match test_native {
                Some(
                    operation @ (ManagedSqliteShmTestUnmapNativeOperation::ExactSiblingDeleteRetryable
                    | ManagedSqliteShmTestUnmapNativeOperation::ExactSiblingDeleteOutcomeUncertain),
                ) => self.namespace.delete_shm_for_wal_with_test_native(
                    operation,
                    || {
                        self.trigger_test_unmap_native(
                            connection_id,
                            operation,
                            prior_unmap_mutation,
                        )
                        .map_err(|_| {
                            std::io::Error::other(
                                "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DELETE_NATIVE_TRIGGER_FAILED",
                            )
                        })
                    },
                    |observation| {
                        self.witness_test_unmap_native(
                            connection_id,
                            operation,
                            observation,
                            true,
                        )
                            .map_err(|_| {
                                std::io::Error::other(
                                    "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DELETE_NATIVE_WITNESS_FAILED",
                                )
                            })
                    },
                ),
                Some(_) => {
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                        "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DELETE_NATIVE_INVALID",
                        prior_unmap_mutation,
                        false,
                    )
                    .into());
                }
                None => self.namespace.delete_shm_for_wal(),
            };
            #[cfg(not(all(test, windows)))]
            let delete_result = self.namespace.delete_shm_for_wal();
            let outcome = match delete_result {
                Ok(outcome) => outcome,
                Err(failure) => {
                    return Err(self
                        .consume_delete_failure(&mut state, failure, prior_unmap_mutation)
                        .into());
                }
            };
            #[cfg(all(test, windows))]
            if self.observes_test_unmap_target(connection_id)? {
                self.record_test_unmap_delete_outcome(connection_id, outcome)?;
                self.finish_test_unmap_action(
                    connection_id,
                    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                    true,
                )?;
            }
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
        #[cfg(all(test, windows))]
        self.begin_test_connection_detach_action(connection_id, prior_unmap_mutation)?;
        self.detach_connection(&mut state, connection_id)?;
        #[cfg(all(test, windows))]
        if let Err(failure) = self.finish_test_connection_detach_action(connection_id, true) {
            return Err(ManagedSqliteShmInnerUnmapFailure::detached(failure));
        }
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
        connection_id: u64,
        delete: &ManagedSqliteShmDeleteDisposition<'_>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let ManagedSqliteShmDeleteDisposition::Delete {
            main,
            runtime_generation,
        } = delete
        else {
            return Ok(());
        };
        #[cfg(all(test, windows))]
        if self.observes_test_unmap_target(connection_id)? {
            return self.validate_delete_authority_for_test(
                state,
                connection_id,
                main,
                *runtime_generation,
            );
        }
        self.validate_delete_authority_request(
            state,
            main,
            Some(main.identity_digest()),
            *runtime_generation,
            || main.lock_level().map_err(drop),
        )
    }

    /// The single production-shaped Delete authority evaluator. Windows dynamic evidence calls
    /// this same function first with its projected bad request and then with the untouched request.
    fn validate_delete_authority_request(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        main: &super::super::PinnedManagedSqliteMainFile,
        request_identity: Option<&str>,
        runtime_generation: std::num::NonZeroU64,
        query_lock: impl FnOnce() -> Result<ManagedSqliteObservedLock, ()>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = ManagedSqliteShmFailurePhase::DeleteAuthorization;
        let Some(main_identity_digest) = state.main_identity_digest.as_deref() else {
            return Err(ManagedSqliteShmFailure::code(
                phase,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_MAIN_IDENTITY_MISSING",
            ));
        };
        let Some(request_identity) = request_identity else {
            return Err(ManagedSqliteShmFailure::code(
                phase,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_MAIN_IDENTITY_MISSING",
            ));
        };
        if runtime_generation != self.generation
            || request_identity != main_identity_digest
            || main.identity_digest() != request_identity
        {
            return Err(ManagedSqliteShmFailure::code(
                phase,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_AUTHORITY_MISMATCH",
            ));
        }
        let lock_level = match query_lock() {
            Ok(lock_level) => lock_level,
            Err(()) => {
                self.mark_poisoned(state, phase, false, true);
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    phase,
                    "NODE_MANAGED_SQLITE_SHM_MAIN_LOCK_STATE_UNAVAILABLE",
                    false,
                    true,
                ));
            }
        };
        if lock_level != ManagedSqliteObservedLock::Exclusive {
            return Err(ManagedSqliteShmFailure::code(
                phase,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_DELETE_REQUIRES_MAIN_EXCLUSIVE",
            ));
        }
        Ok(())
    }

    #[cfg(all(test, windows))]
    fn validate_delete_authority_for_test(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        main: &super::super::PinnedManagedSqliteMainFile,
        runtime_generation: std::num::NonZeroU64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = ManagedSqliteShmFailurePhase::DeleteAuthorization;
        let stored_before = state.main_identity_digest.clone();
        let test_prestate = self.take_test_unmap_authority_prestate(connection_id)?;
        let mut request_identity = Some(main.identity_digest());
        let mut request_generation = runtime_generation;
        let lock_query_unavailable =
            test_prestate == Some(ManagedSqliteShmTestUnmapDeletePrestate::LockQueryUnavailable);
        match test_prestate {
            Some(prestate @ ManagedSqliteShmTestUnmapDeletePrestate::MissingIdentity) => {
                request_identity = None;
                self.mark_test_unmap_prestate_applied(connection_id, prestate, phase, false)?;
            }
            Some(prestate @ ManagedSqliteShmTestUnmapDeletePrestate::IdentityMismatch) => {
                let mismatched = if self.generation.get() == u64::MAX {
                    1
                } else {
                    self.generation.get() + 1
                };
                request_generation = std::num::NonZeroU64::new(mismatched)
                    .expect("mismatched generation is nonzero");
                self.mark_test_unmap_prestate_applied(connection_id, prestate, phase, false)?;
            }
            Some(prestate @ ManagedSqliteShmTestUnmapDeletePrestate::LockQueryUnavailable) => {
                self.mark_test_unmap_prestate_applied(connection_id, prestate, phase, false)?;
            }
            Some(ManagedSqliteShmTestUnmapDeletePrestate::NotFound) => {
                unreachable!("NotFound prestate is consumed only after teardown")
            }
            None => {}
        }

        let stored_identity = state.main_identity_digest.clone();
        let identity_matches = matches!(
            (request_identity, stored_identity.as_deref()),
            (Some(request), Some(stored)) if request == stored
        );
        let generation_matches = request_generation == self.generation;
        let lock_level = if lock_query_unavailable {
            None
        } else {
            match main.lock_level() {
                Ok(level) => Some(level),
                Err(_) => {
                    self.record_test_unmap_delete_authority(
                        connection_id,
                        ManagedSqliteShmTestUnmapDeleteAuthorityReceipt {
                            stored_identity_present: stored_identity.is_some(),
                            request_identity_present: request_identity.is_some(),
                            identity_matches,
                            generation_matches,
                            lock_level: None,
                            lock_query_unavailable: true,
                            stored_identity_unchanged: state.main_identity_digest == stored_before,
                            selected_request_validation_attempted: false,
                            selected_request_validation_succeeded: false,
                            correct_request_recheck_attempted: false,
                            correct_request_recheck_succeeded: false,
                        },
                    )?;
                    self.mark_poisoned(state, phase, false, true);
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        phase,
                        "NODE_MANAGED_SQLITE_SHM_MAIN_LOCK_STATE_UNAVAILABLE",
                        false,
                        true,
                    ));
                }
            }
        };
        let selected_validation = if lock_query_unavailable {
            self.validate_delete_authority_request(
                state,
                main,
                request_identity,
                request_generation,
                || Err(()),
            )
        } else {
            self.validate_delete_authority_request(
                state,
                main,
                request_identity,
                request_generation,
                || main.lock_level().map_err(drop),
            )
        };
        // Every installed prestate is one-shot. The recheck always uses the untouched request and
        // the real main-file lock query, including after the selected lock-query-unavailable seam.
        let correct_recheck = self.validate_delete_authority_request(
            state,
            main,
            Some(main.identity_digest()),
            runtime_generation,
            || main.lock_level().map_err(drop),
        );
        self.record_test_unmap_delete_authority(
            connection_id,
            ManagedSqliteShmTestUnmapDeleteAuthorityReceipt {
                stored_identity_present: stored_identity.is_some(),
                request_identity_present: request_identity.is_some(),
                identity_matches,
                generation_matches,
                lock_level,
                lock_query_unavailable,
                stored_identity_unchanged: state.main_identity_digest == stored_before,
                selected_request_validation_attempted: true,
                selected_request_validation_succeeded: selected_validation.is_ok(),
                correct_request_recheck_attempted: true,
                correct_request_recheck_succeeded: correct_recheck.is_ok(),
            },
        )?;
        selected_validation
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
