use std::io;

use super::super::{
    platform, ManagedSqliteFileCloseFailure, PinnedManagedSqliteFile,
    PlatformManagedSqliteLockAttempt,
};
use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        ManagedSqliteShmFileCloseCustody, ManagedSqliteShmNode,
    },
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        SHM_DMS_OFFSET,
    },
};

#[cfg(all(test, windows))]
#[path = "node_initialization/created_first_truncate_error_release_failed.rs"]
mod created_first_truncate_error_release_failed;
#[cfg(all(test, windows))]
#[path = "node_initialization/created_first_truncate_error_release_succeeded.rs"]
mod created_first_truncate_error_release_succeeded;
#[cfg(all(test, windows))]
#[path = "node_initialization/existing_first_truncate_error_release_succeeded.rs"]
mod existing_first_truncate_error_release_succeeded;
#[cfg(all(test, windows))]
#[path = "node_initialization/existing_first_truncate_error_release_failed.rs"]
mod existing_first_truncate_error_release_failed;
impl ManagedSqliteShmCoordinator {
    pub(super) fn ensure_node<'state>(
        &self,
        state: &'state mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(&'state mut ManagedSqliteShmNode, bool), ManagedSqliteShmFailure> {
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        let opened_now = state.node.is_none();
        if opened_now {
            let node = self.open_node(state, connection_id)?;
            state.node = Some(node);
        }
        if state.node.is_none() {
            self.mark_poisoned(state, ManagedSqliteShmFailurePhase::Gate, false, false);
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Gate,
                "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_OPEN",
                false,
                false,
            ));
        }
        match state.node.as_mut() {
            Some(node) => {
                let initialization_mutated = opened_now && node.initialization_mutated;
                Ok((node, initialization_mutated))
            }
            None => Err(self.poisoned_failure()),
        }
    }

    fn open_node(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        _connection_id: u64,
    ) -> Result<ManagedSqliteShmNode, ManagedSqliteShmFailure> {
        #[cfg(all(test, windows))]
        self.record_test_initialization_open_attempt_v1(state, _connection_id)?;
        #[cfg(test)]
        let open_fault = self.begin_test_fault(
            state,
            _connection_id,
            ManagedSqliteShmFailurePhase::ExactSiblingOpen,
            false,
        )?;
        let mut file = match self.namespace.open_shm_for_wal() {
            Ok(file) => file,
            Err(failure) => {
                let open_failure = self.consume_open_failure(state, failure);
                #[cfg(all(test, windows))]
                if let Err(controller_failure) = self.reject_test_initialization_path_v1(
                    state,
                    _connection_id,
                    ManagedSqliteShmFailurePhase::ExactSiblingOpen,
                    true,
                    false,
                    "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_OPEN_FAILED",
                ) {
                    return Err(controller_failure);
                }
                return Err(open_failure);
            }
        };

        let file_created = file.was_created();
        #[cfg(all(test, windows))]
        if let Err(failure) =
            self.record_test_initialization_open_created_v1(state, _connection_id, file_created)
        {
            return Err(self.close_failed_open_file(state, file, failure));
        }
        #[cfg(test)]
        if let Some(fault) = open_fault {
            state.node = Some(new_node(
                file,
                ManagedSqliteShmDmsCustody::Released,
                file_created,
            ));
            return Err(self.activate_after_test_fault(state, fault, true));
        }

        #[cfg(test)]
        let exclusive_acquire_fault = match self.begin_test_fault(
            state,
            _connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
            file_created,
        ) {
            Ok(fault) => fault,
            Err(failure) => return Err(self.close_failed_open_file(state, file, failure)),
        };
        #[cfg(all(test, windows))]
        if let Err(failure) =
            self.record_test_initialization_dms_lock_attempt_v1(state, _connection_id)
        {
            return Err(self.close_failed_open_file(state, file, failure));
        }
        let first_process =
            match platform::try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, true) {
                Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                    #[cfg(all(test, windows))]
                    if let Err(failure) =
                        self.record_test_initialization_dms_acquired_v1(state, _connection_id)
                    {
                        state.node = Some(new_node(
                            file,
                            ManagedSqliteShmDmsCustody::ExclusiveKnown,
                            file_created,
                        ));
                        return Err(failure);
                    }
                    #[cfg(test)]
                    if let Some(fault) = exclusive_acquire_fault {
                        state.node = Some(new_node(
                            file,
                            ManagedSqliteShmDmsCustody::ExclusiveKnown,
                            file_created,
                        ));
                        return Err(self.activate_after_test_fault(state, fault, true));
                    }
                    true
                }
                Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                    #[cfg(all(test, windows))]
                    if let Err(failure) = self.reject_test_initialization_path_v1(
                        state,
                        _connection_id,
                        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
                        file_created,
                        false,
                        "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_DMS_LOCK_CONTENDED",
                    ) {
                        return Err(self.close_failed_open_file(state, file, failure));
                    }
                    false
                }
                Err(error) => {
                    #[cfg(all(test, windows))]
                    if let Err(failure) = self.reject_test_initialization_path_v1(
                        state,
                        _connection_id,
                        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
                        file_created,
                        false,
                        "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_DMS_LOCK_FAILED",
                    ) {
                        return Err(self.close_failed_open_file(state, file, failure));
                    }
                    let failure = ManagedSqliteShmFailure::new(
                        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
                        if file_created {
                            ManagedSqliteShmFailureClass::MutatedButKnown
                        } else {
                            classify_platform(&error)
                        },
                        error,
                    );
                    return Err(self.close_failed_open_file(state, file, failure));
                }
            };

        let mut truncated = false;
        if first_process {
            #[cfg(test)]
            let truncate_fault = match self.begin_test_fault(
                state,
                _connection_id,
                ManagedSqliteShmFailurePhase::DmsTruncate,
                true,
            ) {
                Ok(fault) => fault,
                Err(failure)
                    if failure.class()
                        == ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned =>
                {
                    state.node = Some(new_node(
                        file,
                        ManagedSqliteShmDmsCustody::ExclusiveKnown,
                        file_created,
                    ));
                    return Err(failure);
                }
                Err(failure) => {
                    if let Err(release_error) =
                        platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1)
                    {
                        state.node = Some(new_node(
                            file,
                            ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
                            file_created,
                        ));
                        self.mark_poisoned(
                            state,
                            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                            true,
                            true,
                        );
                        return Err(ManagedSqliteShmFailure::poisoned(
                            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                            release_error,
                            true,
                            true,
                        ));
                    }
                    return Err(self.close_failed_open_file(state, file, failure));
                }
            };
            #[cfg(all(test, windows))]
            if let Err(failure) =
                self.record_test_initialization_truncate_attempt_v1(state, _connection_id)
            {
                state.node = Some(new_node(
                    file,
                    ManagedSqliteShmDmsCustody::ExclusiveKnown,
                    file_created,
                ));
                return Err(failure);
            }
            #[cfg(all(test, windows))]
            let file = self.execute_q14_truncate_release_ok_test_v1(state, _connection_id, file)?;
            #[cfg(all(test, windows))]
            let file = self.execute_q15_truncate_release_ok_test_v1(state, _connection_id, file)?;
            #[cfg(all(test, windows))]
            let file =
                self.execute_q16_truncate_release_failed_test_v1(state, _connection_id, file)?;
            #[cfg(all(test, windows))]
            let mut file =
                self.execute_q17_truncate_release_failed_test_v1(state, _connection_id, file)?;
            if let Err(error) = file.truncate(0) {
                #[cfg(all(test, windows))]
                if let Err(failure) = self.reject_test_initialization_path_v1(
                    state,
                    _connection_id,
                    ManagedSqliteShmFailurePhase::DmsTruncate,
                    true,
                    false,
                    "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TRUNCATE_FAILED",
                ) {
                    state.node = Some(new_node(
                        file,
                        ManagedSqliteShmDmsCustody::ExclusiveKnown,
                        file_created,
                    ));
                    return Err(failure);
                }
                let truncate_error = io::Error::other(error);
                let release = platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1);
                let (phase, dms, retained_error, lock_uncertain) = match release {
                    Ok(()) => (
                        ManagedSqliteShmFailurePhase::DmsTruncate,
                        ManagedSqliteShmDmsCustody::Released,
                        truncate_error,
                        false,
                    ),
                    Err(release_error) => (
                        ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                        ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
                        release_error,
                        true,
                    ),
                };
                state.node = Some(new_node(file, dms, true));
                self.mark_poisoned(state, phase, true, lock_uncertain);
                return Err(ManagedSqliteShmFailure::poisoned(
                    phase,
                    retained_error,
                    true,
                    lock_uncertain,
                ));
            }
            #[cfg(all(test, windows))]
            if let Err(failure) =
                self.record_test_initialization_truncated_v1(state, _connection_id)
            {
                state.node = Some(new_node(
                    file,
                    ManagedSqliteShmDmsCustody::ExclusiveKnown,
                    true,
                ));
                return Err(failure);
            }
            truncated = true;
            #[cfg(test)]
            if let Some(fault) = truncate_fault {
                state.node = Some(new_node(
                    file,
                    ManagedSqliteShmDmsCustody::ExclusiveKnown,
                    true,
                ));
                return Err(self.activate_after_test_fault(state, fault, true));
            }
            #[cfg(test)]
            let exclusive_release_fault = match self.begin_test_fault(
                state,
                _connection_id,
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                true,
            ) {
                Ok(fault) => fault,
                Err(failure) => {
                    state.node = Some(new_node(
                        file,
                        ManagedSqliteShmDmsCustody::ExclusiveKnown,
                        true,
                    ));
                    self.terminalize_test_fault(state, &failure);
                    return Err(failure);
                }
            };
            #[cfg(all(test, windows))]
            match self.execute_test_initialization_dms_unlock_v1(state, _connection_id, &file.file)
            {
                Ok(Some(error)) => {
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
                    self.record_test_initialization_poisoned_v1(state, _connection_id)?;
                    return Err(ManagedSqliteShmFailure::poisoned(
                        ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                        error,
                        true,
                        true,
                    ));
                }
                Ok(None) => {}
                Err(failure) => {
                    let dms = if failure.lock_outcome_uncertain() {
                        ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain
                    } else {
                        ManagedSqliteShmDmsCustody::ExclusiveKnown
                    };
                    state.node = Some(new_node(file, dms, true));
                    return Err(failure);
                }
            }
            if let Err(error) = platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1) {
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
                return Err(ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                    error,
                    true,
                    true,
                ));
            }
            #[cfg(test)]
            if let Some(fault) = exclusive_release_fault {
                state.node = Some(new_node(file, ManagedSqliteShmDmsCustody::Released, true));
                return Err(self.activate_after_test_fault(state, fault, true));
            }
        }

        #[cfg(test)]
        let shared_acquire_fault = match self.begin_test_fault(
            state,
            _connection_id,
            ManagedSqliteShmFailurePhase::DmsSharedAcquire,
            file_created || truncated,
        ) {
            Ok(fault) => fault,
            Err(failure) => return Err(self.close_failed_open_file(state, file, failure)),
        };
        match platform::try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, false) {
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                let node = new_node(
                    file,
                    ManagedSqliteShmDmsCustody::Shared,
                    file_created || truncated,
                );
                #[cfg(test)]
                if let Some(fault) = shared_acquire_fault {
                    state.node = Some(node);
                    return Err(self.activate_after_test_fault(state, fault, true));
                }
                Ok(node)
            }
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                let failure = ManagedSqliteShmFailure::code(
                    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                    if file_created || truncated {
                        ManagedSqliteShmFailureClass::BusyAfterKnownMutation
                    } else {
                        ManagedSqliteShmFailureClass::BusyNoMutation
                    },
                    "NODE_MANAGED_SQLITE_SHM_DMS_BUSY",
                );
                Err(self.close_failed_open_file(state, file, failure))
            }
            Err(error) => {
                let failure = ManagedSqliteShmFailure::new(
                    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                    if file_created || truncated {
                        ManagedSqliteShmFailureClass::MutatedButKnown
                    } else {
                        classify_platform(&error)
                    },
                    error,
                );
                Err(self.close_failed_open_file(state, file, failure))
            }
        }
    }

    fn close_failed_open_file(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        file: PinnedManagedSqliteFile,
        original: ManagedSqliteShmFailure,
    ) -> ManagedSqliteShmFailure {
        let mutation = original.mutation_may_have_occurred();
        match file.close() {
            Ok(_) => original,
            Err(close_failure) => {
                let report = pinned_close_report(&close_failure);
                state
                    .quarantined_file_close
                    .push(ManagedSqliteShmFileCloseCustody::Pinned(close_failure));
                self.mark_poisoned(
                    state,
                    ManagedSqliteShmFailurePhase::FileClose,
                    mutation,
                    false,
                );
                ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::FileClose,
                    report,
                    mutation,
                    false,
                )
            }
        }
    }
}

fn new_node(
    file: PinnedManagedSqliteFile,
    dms: ManagedSqliteShmDmsCustody,
    initialization_mutated: bool,
) -> ManagedSqliteShmNode {
    ManagedSqliteShmNode {
        file,
        dms,
        initialization_mutated,
        region_size: None,
        regions: Vec::new(),
        mapped_bytes: 0,
    }
}

fn pinned_close_report(failure: &ManagedSqliteFileCloseFailure) -> io::Error {
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

fn classify_platform(error: &io::Error) -> ManagedSqliteShmFailureClass {
    if error.kind() == io::ErrorKind::Unsupported {
        ManagedSqliteShmFailureClass::PlatformUnsupported
    } else {
        ManagedSqliteShmFailureClass::IoBeforeMutation
    }
}
