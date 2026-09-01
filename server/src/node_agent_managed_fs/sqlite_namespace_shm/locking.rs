use std::io;

use super::super::{platform, PlatformManagedSqliteLockAttempt};
#[cfg(all(test, windows))]
use super::test_lock_runtime::{
    ManagedSqliteShmTestNativeLockOutcome, ManagedSqliteShmTestNativeUnlockOutcome,
};
use super::{
    coordinator::{
        ManagedSqliteShmConnectionState, ManagedSqliteShmCoordinator,
        ManagedSqliteShmCoordinatorState, PinnedManagedSqliteShmConnection,
    },
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        ManagedSqliteShmLockAction, ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest,
        SHM_DMS_OFFSET, SHM_LOCK_BASE,
    },
};

impl ManagedSqliteShmCoordinator {
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
        #[cfg(all(test, windows))]
        self.begin_test_lock_action(connection_id, request)?;
        match request.action() {
            ManagedSqliteShmLockAction::LockShared => {
                require_unlocked(current, mask)?;
                if sibling.exclusive_mask & mask != 0 {
                    #[cfg(all(test, windows))]
                    self.record_test_local_lock_contention(connection_id, request)?;
                    return Ok(ManagedSqliteShmLockAttempt::Contended);
                }
                if sibling.shared_mask & mask == 0 {
                    let attempt = self.try_os_lock(&mut state, connection_id, request, false)?;
                    if attempt == ManagedSqliteShmLockAttempt::Contended {
                        return Ok(attempt);
                    }
                } else {
                    let held = state.connections.get_mut(&connection_id).ok_or_else(|| {
                        protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED")
                    })?;
                    #[cfg(all(test, windows))]
                    self.record_test_local_lock_transition(connection_id, request)?;
                    held.shared_mask |= mask;
                }
            }
            ManagedSqliteShmLockAction::LockExclusive => {
                require_unlocked(current, mask)?;
                if (sibling.shared_mask | sibling.exclusive_mask) & mask != 0 {
                    #[cfg(all(test, windows))]
                    self.record_test_local_lock_contention(connection_id, request)?;
                    return Ok(ManagedSqliteShmLockAttempt::Contended);
                }
                let attempt = self.try_os_lock(&mut state, connection_id, request, true)?;
                if attempt == ManagedSqliteShmLockAttempt::Contended {
                    return Ok(attempt);
                }
            }
            ManagedSqliteShmLockAction::UnlockShared => {
                if current.shared_mask & mask != mask || current.exclusive_mask & mask != 0 {
                    return Err(protocol("NODE_MANAGED_SQLITE_SHM_SHARED_UNLOCK_NOT_HELD"));
                }
                if sibling.shared_mask & mask == 0 {
                    self.unlock_os_range(&mut state, connection_id, request)?;
                } else {
                    let held = state.connections.get_mut(&connection_id).ok_or_else(|| {
                        protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_DISAPPEARED")
                    })?;
                    #[cfg(all(test, windows))]
                    self.record_test_local_lock_transition(connection_id, request)?;
                    held.shared_mask &= !mask;
                }
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
                self.unlock_os_range(&mut state, connection_id, request)?;
            }
        }
        #[cfg(all(test, windows))]
        self.finish_test_lock_action(connection_id, request)?;
        Ok(ManagedSqliteShmLockAttempt::Acquired)
    }

    fn try_os_lock(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
        exclusive: bool,
    ) -> Result<ManagedSqliteShmLockAttempt, ManagedSqliteShmFailure> {
        if !matches!(
            (request.action(), exclusive),
            (ManagedSqliteShmLockAction::LockShared, false)
                | (ManagedSqliteShmLockAction::LockExclusive, true)
        ) {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_LOCK_ACTION_CHANGED"));
        }
        let initialization_mutated = {
            let (_, initialization_mutated) = self.ensure_node(state, connection_id)?;
            initialization_mutated
        };
        #[cfg(test)]
        let fault = self.begin_test_fault(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::LockAcquire,
            initialization_mutated,
        )?;
        #[cfg(all(test, windows))]
        self.begin_test_native_lock_action(connection_id, request)?;
        let attempt = {
            let node = state
                .node
                .as_ref()
                .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_LOCK"))?;
            platform::try_lock_sqlite_byte_range(
                &node.file.file,
                SHM_LOCK_BASE + u64::from(request.first()),
                u64::from(request.count()),
                exclusive,
            )
        };
        match attempt {
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                let Some(held) = state.connections.get_mut(&connection_id) else {
                    self.mark_poisoned(
                        state,
                        ManagedSqliteShmFailurePhase::LockAcquire,
                        true,
                        true,
                    );
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        ManagedSqliteShmFailurePhase::LockAcquire,
                        "NODE_MANAGED_SQLITE_SHM_CONNECTION_MISSING_AFTER_LOCK",
                        true,
                        true,
                    ));
                };
                if exclusive {
                    held.exclusive_mask |= request.mask();
                    held.exclusive_ranges[usize::from(request.first())] = request.count();
                } else {
                    held.shared_mask |= request.mask();
                }
                #[cfg(all(test, windows))]
                self.finish_test_native_lock_action(
                    connection_id,
                    request,
                    ManagedSqliteShmTestNativeLockOutcome::Acquired,
                    true,
                )?;
                #[cfg(test)]
                if let Some(failure) = self.finish_test_fault(state, fault, true) {
                    return Err(failure);
                }
                Ok(ManagedSqliteShmLockAttempt::Acquired)
            }
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                #[cfg(all(test, windows))]
                self.finish_test_native_lock_action(
                    connection_id,
                    request,
                    ManagedSqliteShmTestNativeLockOutcome::Contended,
                    initialization_mutated,
                )?;
                Ok(ManagedSqliteShmLockAttempt::Contended)
            }
            Err(error) => {
                #[cfg(all(test, windows))]
                self.finish_test_native_lock_action(
                    connection_id,
                    request,
                    ManagedSqliteShmTestNativeLockOutcome::Error,
                    initialization_mutated,
                )?;
                Err(ManagedSqliteShmFailure::new(
                    ManagedSqliteShmFailurePhase::LockAcquire,
                    mutation_class(initialization_mutated, &error),
                    error,
                ))
            }
        }
    }

    fn unlock_os_range(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let exclusive = match request.action() {
            ManagedSqliteShmLockAction::UnlockShared => false,
            ManagedSqliteShmLockAction::UnlockExclusive => true,
            ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::LockExclusive => {
                return Err(protocol("NODE_MANAGED_SQLITE_SHM_UNLOCK_ACTION_CHANGED"));
            }
        };
        #[cfg(test)]
        let fault = self.begin_test_fault(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::LockRelease,
            false,
        )?;
        #[cfg(all(test, windows))]
        self.begin_test_native_unlock_action(connection_id, request)?;
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
            #[cfg(all(test, windows))]
            self.finish_test_native_unlock_action(
                connection_id,
                request,
                ManagedSqliteShmTestNativeUnlockOutcome::Error,
                false,
            )?;
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
        let Some(held) = state.connections.get_mut(&connection_id) else {
            self.mark_poisoned(state, ManagedSqliteShmFailurePhase::LockRelease, true, true);
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::LockRelease,
                "NODE_MANAGED_SQLITE_SHM_CONNECTION_MISSING_AFTER_UNLOCK",
                true,
                true,
            ));
        };
        if exclusive {
            held.exclusive_mask &= !request.mask();
            held.exclusive_ranges[usize::from(request.first())] = 0;
        } else {
            held.shared_mask &= !request.mask();
        }
        #[cfg(all(test, windows))]
        self.finish_test_native_unlock_action(
            connection_id,
            request,
            ManagedSqliteShmTestNativeUnlockOutcome::Success,
            true,
        )?;
        #[cfg(test)]
        if let Some(failure) = self.finish_test_fault(state, fault, true) {
            return Err(failure);
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

fn mutation_class(already_mutated: bool, error: &io::Error) -> ManagedSqliteShmFailureClass {
    if already_mutated {
        ManagedSqliteShmFailureClass::MutatedButKnown
    } else if error.kind() == io::ErrorKind::Unsupported {
        ManagedSqliteShmFailureClass::PlatformUnsupported
    } else {
        ManagedSqliteShmFailureClass::IoBeforeMutation
    }
}
