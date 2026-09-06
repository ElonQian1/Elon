//! Shared SHM callback admission, dispatch, retention and completion.

use super::*;

#[cfg(all(test, windows))]
use super::super::super::types::ManagedSqliteRegistryTransitionRejection;
#[cfg(all(test, windows))]
use super::super::test_faults::{
    ManagedSqliteRegistryPreManagedLockAdmissionOutcome as Admission,
    ManagedSqliteRegistryPreManagedLockCompletionOutcome as Completion,
    ManagedSqliteRegistryPreManagedLockCustody as ObservedCustody,
    ManagedSqliteRegistryPreManagedLockEvent as Event,
    ManagedSqliteRegistryPreManagedLockRejection as PreManagedRejection,
    ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
};

#[derive(Debug, Clone, Copy)]
struct ManagedSqliteRegistryUnsafeShmFailureMarker {
    phase: crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase,
    class: crate::node_agent_managed_fs::ManagedSqliteShmFailureClass,
    mutation_may_have_occurred: bool,
    lock_outcome_uncertain: bool,
}

#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy)]
struct ManagedSqliteRegistryUnsafeShmRoutePreemptionMarker(
    ManagedSqliteRegistryUnsafeShmFailureMarker,
);

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn with_shm<T>(
        &mut self,
        _lock_request: Option<ManagedSqliteShmLockRequest>,
        operation: impl FnOnce(
            &mut crate::node_agent_managed_fs::PinnedManagedSqliteShmConnection,
        ) -> Result<T, ManagedSqliteShmFailure>,
        _ordinary_lock_result: impl FnOnce(
            &T,
        ) -> Option<(
            ManagedSqliteShmLockRequest,
            ManagedSqliteShmLockAttempt,
        )>,
    ) -> Result<T, ManagedSqliteRegistryPinnedFileOperationRejection> {
        #[cfg(all(test, windows))]
        let lock_request = _lock_request;
        #[cfg(all(test, windows))]
        if let Some(request) = lock_request {
            self.observe_pre_managed_lock(Event::Entry { request });
        }
        let callback = match self
            .owner
            .begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)
        {
            Ok(callback) => {
                #[cfg(all(test, windows))]
                if let Some(request) = lock_request {
                    self.observe_pre_managed_lock(Event::Admission {
                        request,
                        outcome: Admission::Succeeded,
                    });
                }
                callback
            }
            Err(rejection) => {
                #[cfg(all(test, windows))]
                if let Some(request) = lock_request {
                    self.observe_pre_managed_lock(Event::Admission {
                        request,
                        outcome: admission_outcome(&rejection),
                    });
                }
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry(
                    rejection,
                ));
            }
        };
        #[cfg(all(test, windows))]
        let mut observed_custody = ObservedCustody::Sidecar;
        #[cfg(all(test, windows))]
        let mut observed_shm_present = false;
        let result = (|| {
            let custody = self
                .custody
                .as_mut()
                .expect("live pinned file operation must retain exact custody");
            let file = match custody {
                ManagedSqliteRegistryPinnedFileCustody::Main { .. } => {
                    #[cfg(all(test, windows))]
                    {
                        observed_custody = ObservedCustody::Main;
                    }
                    return Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole,
                    );
                }
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { .. } => {
                    return Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole,
                    );
                }
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    #[cfg(all(test, windows))]
                    {
                        observed_custody = ObservedCustody::WalMain;
                    }
                    file
                }
            };
            let Some(shm) = file.shm_mut() else {
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached);
            };
            #[cfg(all(test, windows))]
            {
                observed_shm_present = true;
            }
            operation(shm).map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm)
        })();
        #[cfg(all(test, windows))]
        if let Some(request) = lock_request {
            self.observe_pre_managed_lock(Event::Dispatch {
                request,
                custody: observed_custody,
                shm_present: observed_shm_present,
                rejection: pre_managed_rejection(&result),
                managed_reached: matches!(
                    &result,
                    Ok(_) | Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(_))
                ),
            });
        }
        #[cfg(all(test, windows))]
        let pre_managed_preemption = lock_request.and_then(|request| {
            pre_managed_rejection(&result)
                .and_then(|rejection| self.preempt_pre_managed_lock_route(request, rejection))
        });
        #[cfg(all(test, windows))]
        let mut unsafe_preemption = None;
        if let Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(failure)) = &result {
            #[cfg(all(test, windows))]
            let preemption_retained = unsafe_shm_failure_marker(failure).and_then(|marker| {
                self.close_faults
                    .as_ref()
                    .is_some_and(|faults| {
                        faults.claim_unsafe_shm_route_preemption().unwrap_or(false)
                    })
                    .then(|| {
                        self.owner
                            .retain_terminal_custody(
                                self.route,
                                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                                ManagedSqliteRegistryUnsafeShmRoutePreemptionMarker(marker),
                            )
                            .is_ok()
                    })
            });
            let _unsafe_retention = self.quarantine_unsafe_shm_failure(failure);
            #[cfg(all(test, windows))]
            if let Some(retained) = preemption_retained {
                unsafe_preemption = Some((
                    retained,
                    _unsafe_retention.as_ref().is_some_and(route_was_unknown),
                ));
            }
        }
        #[cfg(all(test, windows))]
        let ordinary_preemption = result
            .as_ref()
            .ok()
            .and_then(_ordinary_lock_result)
            .and_then(|(request, outcome)| self.preempt_ordinary_shm_lock_route(request, outcome));
        let callback_completion = callback.complete();
        #[cfg(all(test, windows))]
        if let Some(request) = lock_request {
            self.observe_pre_managed_lock(Event::Completion {
                request,
                outcome: completion_outcome(&callback_completion),
            });
            self.record_pre_managed_lock_preemption(pre_managed_preemption, &callback_completion);
        }
        #[cfg(all(test, windows))]
        if let Some((retained, unsafe_route_unknown)) = unsafe_preemption {
            if let Some(faults) = self.close_faults.as_ref() {
                let _ = faults.record_unsafe_shm_route_preemption_receipt(
                    ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt::new(
                        retained,
                        unsafe_route_unknown,
                        route_was_unknown(&callback_completion),
                    ),
                );
            }
        }
        #[cfg(all(test, windows))]
        self.record_ordinary_shm_lock_route_preemption(ordinary_preemption, &callback_completion);
        match (result, callback_completion) {
            (Err(rejection), _) => Err(rejection),
            (Ok(value), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileOperationRejection::Registry(rejection),
            ),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    pub(super) fn quarantine_unsafe_shm_failure(
        &self,
        failure: &ManagedSqliteShmFailure,
    ) -> Option<Result<(), ManagedSqliteRegistryProcessRouteRejection>> {
        let marker = unsafe_shm_failure_marker(failure)?;
        Some(self.owner.retain_terminal_custody(
            self.route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            marker,
        ))
    }
}

fn unsafe_shm_failure_marker(
    failure: &ManagedSqliteShmFailure,
) -> Option<ManagedSqliteRegistryUnsafeShmFailureMarker> {
    if failure.class()
        != crate::node_agent_managed_fs::ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
        && !failure.mutation_may_have_occurred()
        && !failure.lock_outcome_uncertain()
    {
        return None;
    }
    Some(ManagedSqliteRegistryUnsafeShmFailureMarker {
        phase: failure.phase(),
        class: failure.class(),
        mutation_may_have_occurred: failure.mutation_may_have_occurred(),
        lock_outcome_uncertain: failure.lock_outcome_uncertain(),
    })
}

#[cfg(all(test, windows))]
fn pre_managed_rejection<T>(
    result: &Result<T, ManagedSqliteRegistryPinnedFileOperationRejection>,
) -> Option<PreManagedRejection> {
    match result {
        Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole) => {
            Some(PreManagedRejection::UnsupportedFileRole)
        }
        Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached) => {
            Some(PreManagedRejection::ShmDetached)
        }
        _ => None,
    }
}

#[cfg(all(test, windows))]
fn admission_outcome(rejection: &ManagedSqliteRegistryProcessRouteRejection) -> Admission {
    match rejection {
        ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ) => Admission::RouteUnknown,
        ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::State(
                ManagedSqliteRegistryTransitionRejection::CounterOverflow,
            ),
        ) => Admission::CounterOverflow,
        _ => Admission::OtherRejection,
    }
}

#[cfg(all(test, windows))]
fn completion_outcome(
    result: &Result<(), ManagedSqliteRegistryProcessRouteRejection>,
) -> Completion {
    if result.is_ok() {
        Completion::Succeeded
    } else if route_was_unknown(result) {
        Completion::RouteUnknown
    } else {
        Completion::OtherRejection
    }
}
