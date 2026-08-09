use std::{io, sync::atomic};

use super::super::{
    platform, ManagedSqliteFileCloseFailure, PinnedManagedSqliteFile,
    PlatformManagedSqliteLockAttempt,
};
use super::{
    coordinator::{
        ManagedSqliteShmConnectionState, ManagedSqliteShmCoordinator,
        ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        ManagedSqliteShmFileCloseCustody, ManagedSqliteShmNode, PinnedManagedSqliteShmConnection,
    },
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        ManagedSqliteShmLockAction, ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest,
        SHM_DMS_OFFSET, SHM_LOCK_BASE,
    },
};

impl ManagedSqliteShmCoordinator {
    pub(super) fn ensure_node<'state>(
        &self,
        state: &'state mut ManagedSqliteShmCoordinatorState,
    ) -> Result<(&'state mut ManagedSqliteShmNode, bool), ManagedSqliteShmFailure> {
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        let opened_now = state.node.is_none();
        if opened_now {
            let node = self.open_node(state)?;
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
    ) -> Result<ManagedSqliteShmNode, ManagedSqliteShmFailure> {
        let mut file = match self.namespace.open_shm_for_wal() {
            Ok(file) => file,
            Err(failure) => return Err(self.consume_open_failure(state, failure)),
        };

        let file_created = file.was_created();
        let first_process =
            match platform::try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, true) {
                Ok(PlatformManagedSqliteLockAttempt::Acquired) => true,
                Ok(PlatformManagedSqliteLockAttempt::Contended) => false,
                Err(error) => {
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
            if let Err(error) = file.truncate(0) {
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
                state.node = Some(ManagedSqliteShmNode {
                    file,
                    dms,
                    initialization_mutated: true,
                    region_size: None,
                    regions: Vec::new(),
                    mapped_bytes: 0,
                });
                self.mark_poisoned(state, phase, true, lock_uncertain);
                return Err(ManagedSqliteShmFailure::poisoned(
                    phase,
                    retained_error,
                    true,
                    lock_uncertain,
                ));
            }
            truncated = true;
            if let Err(error) = platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1) {
                state.node = Some(ManagedSqliteShmNode {
                    file,
                    dms: ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain,
                    initialization_mutated: true,
                    region_size: None,
                    regions: Vec::new(),
                    mapped_bytes: 0,
                });
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
        }

        match platform::try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, false) {
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => Ok(ManagedSqliteShmNode {
                file,
                dms: ManagedSqliteShmDmsCustody::Shared,
                initialization_mutated: file_created || truncated,
                region_size: None,
                regions: Vec::new(),
                mapped_bytes: 0,
            }),
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

    pub(super) fn lock_connection(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<ManagedSqliteShmLockAttempt, ManagedSqliteShmFailure> {
        let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        let current = *state
            .connections
            .get(&connection_id)
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED"))?;
        let sibling = sibling_masks(&state, connection_id);
        let mask = request.mask();
        match request.action() {
            ManagedSqliteShmLockAction::LockShared => {
                require_unlocked(current, mask)?;
                if sibling.exclusive_mask & mask != 0 {
                    return Ok(ManagedSqliteShmLockAttempt::Contended);
                }
                if sibling.shared_mask & mask == 0 {
                    let attempt = self.try_os_lock(&mut state, request, false)?;
                    if attempt == ManagedSqliteShmLockAttempt::Contended {
                        return Ok(attempt);
                    }
                }
                let held = state
                    .connections
                    .get_mut(&connection_id)
                    .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED"))?;
                held.shared_mask |= mask;
            }
            ManagedSqliteShmLockAction::LockExclusive => {
                require_unlocked(current, mask)?;
                if (sibling.shared_mask | sibling.exclusive_mask) & mask != 0 {
                    return Ok(ManagedSqliteShmLockAttempt::Contended);
                }
                let attempt = self.try_os_lock(&mut state, request, true)?;
                if attempt == ManagedSqliteShmLockAttempt::Contended {
                    return Ok(attempt);
                }
                let held = state
                    .connections
                    .get_mut(&connection_id)
                    .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED"))?;
                held.exclusive_mask |= mask;
                held.exclusive_ranges[usize::from(request.first())] = request.count();
            }
            ManagedSqliteShmLockAction::UnlockShared => {
                if current.shared_mask & mask != mask || current.exclusive_mask & mask != 0 {
                    return Err(protocol("NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD"));
                }
                if sibling.shared_mask & mask == 0 {
                    self.unlock_os_range(&mut state, request)?;
                }
                let held = state
                    .connections
                    .get_mut(&connection_id)
                    .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED"))?;
                held.shared_mask &= !mask;
            }
            ManagedSqliteShmLockAction::UnlockExclusive => {
                if current.exclusive_mask & mask != mask || current.shared_mask & mask != 0 {
                    return Err(protocol(
                        "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_NOT_HELD",
                    ));
                }
                if current.exclusive_ranges[usize::from(request.first())] != request.count() {
                    return Err(protocol(
                        "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_UNLOCK_RANGE_MISMATCH",
                    ));
                }
                if (sibling.shared_mask | sibling.exclusive_mask) & mask != 0 {
                    return Err(protocol(
                        "NODE_MANAGED_SQLITE_SHM_EXCLUSIVE_SIBLING_OVERLAP",
                    ));
                }
                self.unlock_os_range(&mut state, request)?;
                let held = state
                    .connections
                    .get_mut(&connection_id)
                    .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED"))?;
                held.exclusive_mask &= !mask;
                held.exclusive_ranges[usize::from(request.first())] = 0;
            }
        }
        Ok(ManagedSqliteShmLockAttempt::Acquired)
    }

    fn try_os_lock(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        request: ManagedSqliteShmLockRequest,
        exclusive: bool,
    ) -> Result<ManagedSqliteShmLockAttempt, ManagedSqliteShmFailure> {
        let (node, initialization_mutated) = self.ensure_node(state)?;
        match platform::try_lock_sqlite_byte_range(
            &node.file.file,
            SHM_LOCK_BASE + u64::from(request.first()),
            u64::from(request.count()),
            exclusive,
        ) {
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                Ok(ManagedSqliteShmLockAttempt::Acquired)
            }
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                Ok(ManagedSqliteShmLockAttempt::Contended)
            }
            Err(error) => Err(ManagedSqliteShmFailure::new(
                ManagedSqliteShmFailurePhase::LockAcquire,
                mutation_class(initialization_mutated, &error),
                error,
            )),
        }
    }

    fn unlock_os_range(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let result = match state.node.as_mut() {
            Some(node) => platform::unlock_sqlite_byte_range(
                &node.file.file,
                SHM_LOCK_BASE + u64::from(request.first()),
                u64::from(request.count()),
            ),
            None => {
                self.mark_poisoned(
                    state,
                    ManagedSqliteShmFailurePhase::LockRelease,
                    false,
                    true,
                );
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::LockRelease,
                    "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_UNLOCK",
                    false,
                    true,
                ));
            }
        };
        if let Err(error) = result {
            self.mark_poisoned(
                state,
                ManagedSqliteShmFailurePhase::LockRelease,
                false,
                true,
            );
            return Err(ManagedSqliteShmFailure::poisoned(
                ManagedSqliteShmFailurePhase::LockRelease,
                error,
                false,
                true,
            ));
        }
        Ok(())
    }
}

impl PinnedManagedSqliteShmConnection {
    pub(crate) fn lock(
        &mut self,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<ManagedSqliteShmLockAttempt, ManagedSqliteShmFailure> {
        if !self.active {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_INACTIVE"));
        }
        self.coordinator
            .lock_connection(self.connection_id, request)
    }

    pub(crate) fn barrier(&self) {
        atomic::fence(atomic::Ordering::SeqCst);
        match self.coordinator.state.lock() {
            Ok(guard) => drop(guard),
            Err(_) => self.coordinator.mark_domain_terminal(),
        }
        atomic::fence(atomic::Ordering::SeqCst);
    }
}

fn sibling_masks(
    state: &ManagedSqliteShmCoordinatorState,
    connection_id: u64,
) -> ManagedSqliteShmConnectionState {
    state
        .connections
        .iter()
        .filter(|(id, _)| **id != connection_id)
        .fold(
            ManagedSqliteShmConnectionState::default(),
            |mut all, (_, held)| {
                all.shared_mask |= held.shared_mask;
                all.exclusive_mask |= held.exclusive_mask;
                all
            },
        )
}

fn require_unlocked(
    current: ManagedSqliteShmConnectionState,
    mask: u8,
) -> Result<(), ManagedSqliteShmFailure> {
    if (current.shared_mask | current.exclusive_mask) & mask != 0 {
        return Err(protocol(
            "NODE_MANAGED_SQLITE_SHM_LOCK_TRANSITION_NOT_UNLOCKED",
        ));
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

fn pinned_close_report(failure: &ManagedSqliteFileCloseFailure) -> io::Error {
    close_report(failure.error_kind(), failure.raw_os_error())
}

fn close_report(kind: io::ErrorKind, raw_os_error: Option<i32>) -> io::Error {
    raw_os_error.map_or_else(
        || io::Error::new(kind, "NODE_MANAGED_SQLITE_SHM_FILE_CLOSE_FAILED"),
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

fn mutation_class(already_mutated: bool, error: &io::Error) -> ManagedSqliteShmFailureClass {
    if already_mutated {
        ManagedSqliteShmFailureClass::MutatedButKnown
    } else if error.kind() == io::ErrorKind::Unsupported {
        ManagedSqliteShmFailureClass::PlatformUnsupported
    } else {
        ManagedSqliteShmFailureClass::IoBeforeMutation
    }
}
