use std::{mem::ManuallyDrop, ptr};

use super::*;
use crate::node_agent_managed_fs::ManagedSqliteUnlockTarget;

#[cfg(all(test, windows))]
#[path = "main_close_test_native.rs"]
mod test_native;
#[cfg(all(test, windows))]
#[path = "main_close_tests.rs"]
mod tests;
#[cfg(all(test, windows))]
use test_native::{
    close_main_file_for_test_native, main_file_native_protocol_failure,
    unlock_main_for_close_test_native,
};

struct ManagedSqliteMainFileParts {
    file: PinnedManagedSqliteFile,
    owner: ManagedSqliteLockOwner,
    #[cfg(test)]
    close_test_faults: Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
}

impl PinnedManagedSqliteMainFile {
    pub(crate) fn close(
        mut self,
    ) -> Result<ManagedSqliteMainFileCloseReceipt, ManagedSqliteMainFileCloseFailure> {
        #[cfg(test)]
        if test_faults::triggered(
            &self.close_test_faults,
            ManagedSqliteMainCloseTestFaultPhase::Unlock,
            ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
        ) {
            return Err(injected_main_close_failure(
                self,
                ManagedSqliteMainCloseTestFaultPhase::Unlock,
                ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
            ));
        }
        #[cfg(all(test, windows))]
        let (unlock_result, unlock_test_native_requested, unlock_test_protocol_failure) =
            match unlock_main_for_close_test_native(&mut self) {
                Ok(dispatch) => dispatch,
                Err(()) => {
                    return Err(injected_main_close_failure(
                        self,
                        ManagedSqliteMainCloseTestFaultPhase::Unlock,
                        ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
                    ));
                }
            };
        #[cfg(not(all(test, windows)))]
        let unlock_result = self.unlock_to(ManagedSqliteUnlockTarget::None);
        #[cfg(all(test, windows))]
        if unlock_result.is_ok() {
            if let Some(protocol_failure) = unlock_test_protocol_failure {
                return Err(native_observation_rejected_after_unlock(
                    self,
                    protocol_failure,
                ));
            }
        }
        if let Err(lock_failure) = unlock_result {
            #[cfg(all(test, windows))]
            if !unlock_test_native_requested {
                if let Some(faults) = &self.close_test_faults {
                    faults.native_failure(ManagedSqliteMainCloseTestFaultPhase::Unlock);
                }
            }
            #[cfg(all(test, not(windows)))]
            if let Some(faults) = &self.close_test_faults {
                faults.native_failure(ManagedSqliteMainCloseTestFaultPhase::Unlock);
            }
            let terminal = lock_failure.is_terminal();
            #[cfg(test)]
            let (terminal_main_file, terminal_owner, main, close_test_faults) = if terminal {
                let parts = into_main_parts(self);
                (
                    Some(ManuallyDrop::new(parts.file)),
                    Some(parts.owner.into_terminal_tombstone()),
                    None,
                    parts.close_test_faults,
                )
            } else {
                (None, None, Some(self), None)
            };
            #[cfg(not(test))]
            let (terminal_main_file, terminal_owner, main) = if terminal {
                let parts = into_main_parts(self);
                (
                    Some(ManuallyDrop::new(parts.file)),
                    Some(parts.owner.into_terminal_tombstone()),
                    None,
                )
            } else {
                (None, None, Some(self))
            };
            return Err(ManagedSqliteMainFileCloseFailure {
                phase: ManagedSqliteMainFileCloseFailurePhase::LockRelease,
                lock_failure: Some(lock_failure),
                file_failure: None,
                terminal_main_file,
                terminal_owner,
                live_owner: None,
                main,
                #[cfg(test)]
                test_fault: None,
                #[cfg(test)]
                _completed_unlock_main: None,
                #[cfg(test)]
                completed_file: None,
                #[cfg(test)]
                close_test_faults,
                #[cfg(all(test, windows))]
                test_protocol_failure: unlock_test_protocol_failure,
            });
        }

        #[cfg(test)]
        if test_faults::triggered(
            &self.close_test_faults,
            ManagedSqliteMainCloseTestFaultPhase::Unlock,
            ManagedSqliteMainCloseTestFaultTiming::AfterSuccess,
        ) {
            return Err(injected_main_close_failure(
                self,
                ManagedSqliteMainCloseTestFaultPhase::Unlock,
                ManagedSqliteMainCloseTestFaultTiming::AfterSuccess,
            ));
        }
        #[cfg(test)]
        if test_faults::triggered(
            &self.close_test_faults,
            ManagedSqliteMainCloseTestFaultPhase::FileClose,
            ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
        ) {
            return Err(injected_main_close_failure(
                self,
                ManagedSqliteMainCloseTestFaultPhase::FileClose,
                ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
            ));
        }

        #[cfg(all(test, windows))]
        let file_close_test_native = match test_faults::claim_test_native(
            &self.close_test_faults,
            ManagedSqliteMainCloseTestFaultPhase::FileClose,
        ) {
            Ok(None) => None,
            Ok(Some(
                request @ ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable,
            ))
            | Ok(Some(
                request @ ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeUncertain,
            )) => Some(request),
            Ok(Some(
                ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared
                | ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainReserved,
            ))
            | Err(()) => {
                return Err(injected_main_close_failure(
                    self,
                    ManagedSqliteMainCloseTestFaultPhase::FileClose,
                    ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
                ));
            }
        };

        let parts = into_main_parts(self);
        #[cfg(all(test, windows))]
        let (file_close_result, file_close_test_native_requested, file_close_test_protocol_failure) =
            close_main_file_for_test_native(
                parts.file,
                &parts.close_test_faults,
                file_close_test_native,
            );
        #[cfg(not(all(test, windows)))]
        let file_close_result = parts.file.close();
        match file_close_result {
            Ok(file) => {
                drop(parts.owner);
                let receipt = ManagedSqliteMainFileCloseReceipt { file };
                #[cfg(all(test, windows))]
                if let Some(protocol_failure) = file_close_test_protocol_failure {
                    return Err(native_observation_rejected_after_file_close(
                        receipt,
                        parts.close_test_faults,
                        protocol_failure,
                    ));
                }
                #[cfg(test)]
                if test_faults::triggered(
                    &parts.close_test_faults,
                    ManagedSqliteMainCloseTestFaultPhase::FileClose,
                    ManagedSqliteMainCloseTestFaultTiming::AfterSuccess,
                ) {
                    return Err(injected_completed_main_close_failure(
                        receipt,
                        parts.close_test_faults,
                    ));
                }
                Ok(receipt)
            }
            Err(file_failure) => {
                #[cfg(all(test, windows))]
                if !file_close_test_native_requested {
                    if let Some(faults) = &parts.close_test_faults {
                        faults.native_failure(ManagedSqliteMainCloseTestFaultPhase::FileClose);
                    }
                }
                #[cfg(all(test, not(windows)))]
                if let Some(faults) = &parts.close_test_faults {
                    faults.native_failure(ManagedSqliteMainCloseTestFaultPhase::FileClose);
                }
                let outcome_uncertain = file_failure.close_outcome_uncertain();
                let (terminal_owner, live_owner) = if outcome_uncertain {
                    (Some(parts.owner.into_terminal_tombstone()), None)
                } else {
                    (None, Some(parts.owner))
                };
                Err(ManagedSqliteMainFileCloseFailure {
                    phase: ManagedSqliteMainFileCloseFailurePhase::FileClose,
                    lock_failure: None,
                    file_failure: Some(file_failure),
                    terminal_main_file: None,
                    terminal_owner,
                    live_owner,
                    main: None,
                    #[cfg(test)]
                    test_fault: None,
                    #[cfg(test)]
                    _completed_unlock_main: None,
                    #[cfg(test)]
                    completed_file: None,
                    #[cfg(test)]
                    close_test_faults: parts.close_test_faults,
                    #[cfg(all(test, windows))]
                    test_protocol_failure: file_close_test_protocol_failure,
                })
            }
        }
    }
}

fn into_main_parts(main: PinnedManagedSqliteMainFile) -> ManagedSqliteMainFileParts {
    let main = ManuallyDrop::new(main);
    // SAFETY: ManuallyDrop suppresses Drop. Every field is read exactly once into unique custody.
    unsafe {
        ManagedSqliteMainFileParts {
            file: ptr::read(&main.file),
            owner: ptr::read(&main.lock_owner),
            #[cfg(test)]
            close_test_faults: ptr::read(&main.close_test_faults),
        }
    }
}

#[cfg(test)]
fn injected_main_close_failure(
    main: PinnedManagedSqliteMainFile,
    phase: ManagedSqliteMainCloseTestFaultPhase,
    timing: ManagedSqliteMainCloseTestFaultTiming,
) -> ManagedSqliteMainFileCloseFailure {
    let (main, completed_unlock_main) = if phase == ManagedSqliteMainCloseTestFaultPhase::Unlock
        && timing == ManagedSqliteMainCloseTestFaultTiming::AfterSuccess
    {
        (None, Some(ManuallyDrop::new(main)))
    } else {
        (Some(main), None)
    };
    ManagedSqliteMainFileCloseFailure {
        phase: match phase {
            ManagedSqliteMainCloseTestFaultPhase::Unlock => {
                ManagedSqliteMainFileCloseFailurePhase::LockRelease
            }
            ManagedSqliteMainCloseTestFaultPhase::FileClose => {
                ManagedSqliteMainFileCloseFailurePhase::FileClose
            }
        },
        lock_failure: None,
        file_failure: None,
        terminal_main_file: None,
        terminal_owner: None,
        live_owner: None,
        main,
        test_fault: Some(ManagedSqliteMainCloseTestFault { phase, timing }),
        _completed_unlock_main: completed_unlock_main,
        completed_file: None,
        close_test_faults: None,
        #[cfg(all(test, windows))]
        test_protocol_failure: None,
    }
}

#[cfg(test)]
fn injected_completed_main_close_failure(
    receipt: ManagedSqliteMainFileCloseReceipt,
    close_test_faults: Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
) -> ManagedSqliteMainFileCloseFailure {
    ManagedSqliteMainFileCloseFailure {
        phase: ManagedSqliteMainFileCloseFailurePhase::FileClose,
        lock_failure: None,
        file_failure: None,
        terminal_main_file: None,
        terminal_owner: None,
        live_owner: None,
        main: None,
        test_fault: Some(ManagedSqliteMainCloseTestFault {
            phase: ManagedSqliteMainCloseTestFaultPhase::FileClose,
            timing: ManagedSqliteMainCloseTestFaultTiming::AfterSuccess,
        }),
        _completed_unlock_main: None,
        completed_file: Some(receipt),
        close_test_faults,
        #[cfg(all(test, windows))]
        test_protocol_failure: None,
    }
}

#[cfg(all(test, windows))]
fn native_observation_rejected_after_unlock(
    main: PinnedManagedSqliteMainFile,
    protocol_failure: ManagedSqliteMainCloseTestProtocolFailure,
) -> ManagedSqliteMainFileCloseFailure {
    let mut failure = injected_main_close_failure(
        main,
        ManagedSqliteMainCloseTestFaultPhase::Unlock,
        ManagedSqliteMainCloseTestFaultTiming::AfterSuccess,
    );
    failure.test_fault = None;
    failure.test_protocol_failure = Some(protocol_failure);
    failure
}

#[cfg(all(test, windows))]
fn native_observation_rejected_after_file_close(
    receipt: ManagedSqliteMainFileCloseReceipt,
    close_test_faults: Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    protocol_failure: ManagedSqliteMainCloseTestProtocolFailure,
) -> ManagedSqliteMainFileCloseFailure {
    let mut failure = injected_completed_main_close_failure(receipt, close_test_faults);
    failure.test_fault = None;
    failure.test_protocol_failure = Some(protocol_failure);
    failure
}

impl ManagedSqliteMainFileCloseFailure {
    pub(crate) fn phase(&self) -> ManagedSqliteMainFileCloseFailurePhase {
        self.phase
    }

    pub(crate) fn close_outcome_uncertain(&self) -> bool {
        self.terminal_main_file.is_some()
            || self
                .lock_failure
                .as_ref()
                .is_some_and(ManagedSqliteLockFailure::is_terminal)
            || self
                .file_failure
                .as_ref()
                .is_some_and(ManagedSqliteFileCloseFailure::close_outcome_uncertain)
    }

    #[cfg(all(test, windows))]
    pub(crate) fn test_protocol_failure(
        &self,
    ) -> Option<ManagedSqliteMainCloseTestProtocolFailure> {
        self.test_protocol_failure
    }

    pub(crate) fn into_main(mut self) -> Result<PinnedManagedSqliteMainFile, Self> {
        #[cfg(test)]
        if self.test_fault.is_some_and(|fault| {
            fault.timing == ManagedSqliteMainCloseTestFaultTiming::AfterSuccess
        }) {
            return Err(self);
        }
        if let Some(main) = self.main.take() {
            return Ok(main);
        }
        if self.terminal_owner.is_some() {
            return Err(self);
        }
        let Some(file_failure) = self.file_failure.take() else {
            return Err(self);
        };
        match file_failure.into_file() {
            Ok(file) => {
                let Some(lock_owner) = self.live_owner.take() else {
                    self.file_failure = Some(ManagedSqliteFileCloseFailure {
                        error: std::io::Error::other(
                            "NODE_MANAGED_SQLITE_CLOSE_LIVE_OWNER_MISSING",
                        ),
                        class: ManagedSqliteFileCloseFailureClass::PlatformUnsupported,
                        custody: ManagedSqliteFileCloseCustody::Live(file),
                    });
                    return Err(self);
                };
                Ok(PinnedManagedSqliteMainFile {
                    file,
                    lock_owner,
                    #[cfg(test)]
                    close_test_faults: self.close_test_faults.take(),
                })
            }
            Err(file_failure) => {
                self.file_failure = Some(file_failure);
                Err(self)
            }
        }
    }
}
