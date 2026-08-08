use std::time::Instant;

mod accessors;
mod debug;

use accessors::{failure, retained, strictly_later_instant};

use super::{
    lifecycle::{
        ManagedDeleteDisposition, ManagedExpectedIdentityMatchPresence,
        ManagedParentNamespaceCustody, ManagedParentRelativeAbsence,
        ManagedParentRelativeIdentityConflict, ManagedParentRelativeObservation,
        QuarantinedManagedNamespaceObject,
    },
    ManagedNamespaceMutationFence, ManagedObjectBinding,
};
use crate::node_agent_managed_fs::{platform, PlatformNamespaceFlushFailureKind};

const WINDOWS_NAMESPACE_DURABILITY_KIND: &str =
    "windows_nt_flush_buffers_file_ex_normal_parent_directory_v1";

/// A successful native parent-directory barrier followed by a same-handle relative absence proof.
/// This primitive does not itself exclude an out-of-band writer from performing a same-name ABA;
/// callers must retain their mutation-authority fence and must not expose it to Host yet.
#[must_use = "durable namespace evidence must be bound to a later trusted-time observation"]
pub(crate) struct ManagedNamespaceDurable {
    custody: ManagedParentNamespaceCustody,
    _mutation_fence: ManagedNamespaceMutationFence,
    filesystem_kind: &'static str,
    barrier_completed_at: Instant,
    post_absence_observed_at: Instant,
    completed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedNamespaceDurabilityFailurePhase {
    FenceBindingChanged,
    FenceOutcomeUncertain,
    PreBarrierObservationFailed,
    PreBarrierObservationQuarantined,
    PreBarrierExpectedIdentityPresent,
    PreBarrierIdentityConflict,
    BarrierFailed,
    BarrierUnsupported,
    BarrierOutcomeUncertain,
    PostBarrierObservationFailed,
    PostBarrierObservationQuarantined,
    PostBarrierExpectedIdentityPresent,
    PostBarrierIdentityConflict,
}

/// Only the two explicit retry variants expose a continuation. `Retained` permanently owns every
/// handle from an inconclusive or name-present outcome and has no conversion back to absence.
#[must_use = "failed durability custody must be retried only through its typed continuation"]
pub(crate) enum ManagedNamespaceDurabilityFailureCustody {
    RetryBeforeBarrier(ManagedNamespacePreBarrierRetry),
    RetryAfterBarrier(ManagedNamespacePostBarrierObservationRetry),
    Retained(ManagedNamespaceDurabilityRetainedCustody),
}

#[must_use = "failed durability retains exact namespace custody"]
pub(crate) struct ManagedNamespaceDurabilityFailure {
    phase: ManagedNamespaceDurabilityFailurePhase,
    error: std::io::Error,
    custody: ManagedNamespaceDurabilityFailureCustody,
}

/// The retained parent must be observed again before a failed or not-yet-run barrier can retry.
#[must_use = "pre-barrier retry must re-prove absence before flushing"]
pub(crate) struct ManagedNamespacePreBarrierRetry {
    disposition: ManagedDeleteDisposition,
    mutation_fence: ManagedNamespaceMutationFence,
}

/// The native flush completed, so this capability may only retry the post-barrier observation.
#[must_use = "post-barrier retry must not repeat the namespace durability barrier"]
pub(crate) struct ManagedNamespacePostBarrierObservationRetry {
    custody: ManagedParentNamespaceCustody,
    mutation_fence: ManagedNamespaceMutationFence,
    filesystem_kind: &'static str,
    barrier_completed_at: Instant,
}

/// Opaque terminal custody for an opened same-name object or an uncertain native barrier.
#[must_use = "terminal durability custody must remain retained for operator recovery"]
pub(crate) struct ManagedNamespaceDurabilityRetainedCustody {
    inner: ManagedNamespaceDurabilityRetainedInner,
}

enum ManagedNamespaceDurabilityRetainedInner {
    FenceBindingChanged {
        _absence: ManagedParentRelativeAbsence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    FenceOutcomeUncertainBeforeBarrier {
        _absence: ManagedParentRelativeAbsence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    FenceOutcomeUncertainAfterBarrier {
        _disposition: ManagedDeleteDisposition,
        _mutation_fence: ManagedNamespaceMutationFence,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
    },
    PreBarrierQuarantined {
        _disposition: ManagedDeleteDisposition,
        _observed_object: QuarantinedManagedNamespaceObject,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    PreBarrierExpectedIdentity {
        _presence: ManagedExpectedIdentityMatchPresence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    PreBarrierIdentityConflict {
        _conflict: ManagedParentRelativeIdentityConflict,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    BarrierOutcomeUncertain {
        _absence: ManagedParentRelativeAbsence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    BarrierUnsupported {
        _absence: ManagedParentRelativeAbsence,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    PostBarrierQuarantined {
        _disposition: ManagedDeleteDisposition,
        _observed_object: QuarantinedManagedNamespaceObject,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    PostBarrierExpectedIdentity {
        _presence: ManagedExpectedIdentityMatchPresence,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
    PostBarrierIdentityConflict {
        _conflict: ManagedParentRelativeIdentityConflict,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
        _mutation_fence: ManagedNamespaceMutationFence,
    },
}

impl ManagedParentRelativeAbsence {
    pub(crate) fn make_namespace_durable(
        self,
        mutation_fence: ManagedNamespaceMutationFence,
        cleanup_id: &str,
        execution_plan_digest: &str,
        authorization_receipt_digest: &str,
        expected_object_digest: &str,
        installation_id_digest: &str,
        authority_epoch: i64,
        process_owner_epoch: i64,
        step_ordinal: u64,
        not_before: Instant,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        if let Err(error) = mutation_fence.validate_binding(
            self.object_binding(),
            cleanup_id,
            execution_plan_digest,
            authorization_receipt_digest,
            expected_object_digest,
            installation_id_digest,
            authority_epoch,
            process_owner_epoch,
            step_ordinal,
        ) {
            return Err(failure(
                ManagedNamespaceDurabilityFailurePhase::FenceBindingChanged,
                error,
                ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                    ManagedNamespaceDurabilityRetainedInner::FenceBindingChanged {
                        _absence: self,
                        _mutation_fence: mutation_fence,
                    },
                )),
            ));
        }
        if let Err(error) = mutation_fence.ensure_active() {
            return Err(failure(
                ManagedNamespaceDurabilityFailurePhase::FenceOutcomeUncertain,
                error,
                ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                    ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainBeforeBarrier {
                        _absence: self,
                        _mutation_fence: mutation_fence,
                    },
                )),
            ));
        }
        execute_pre_barrier(
            ManagedNamespacePreBarrierRetry {
                disposition: self.into_disposition(),
                mutation_fence,
            },
            not_before,
        )
    }
}

impl ManagedNamespacePreBarrierRetry {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        self.disposition.object_binding()
    }

    pub(crate) fn retry_pre_barrier(
        self,
        not_before: Instant,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        execute_pre_barrier(self, not_before)
    }
}

impl ManagedNamespacePostBarrierObservationRetry {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn retry_post_barrier_observation(
        self,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        execute_post_barrier(self)
    }
}

fn execute_pre_barrier(
    retry: ManagedNamespacePreBarrierRetry,
    not_before: Instant,
) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
    let (absence, mutation_fence) = match observe_absence(retry) {
        Ok(absence) => absence,
        Err(failure) => return Err(failure),
    };
    if let Err(error) = mutation_fence.ensure_active() {
        return Err(failure(
            ManagedNamespaceDurabilityFailurePhase::FenceOutcomeUncertain,
            error,
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainBeforeBarrier {
                    _absence: absence,
                    _mutation_fence: mutation_fence,
                },
            )),
        ));
    }
    let pre_barrier_observed_at = Instant::now();
    let parent = match absence.custody.parent_handle() {
        Ok(parent) => parent,
        Err(error) => {
            return Err(failure(
                ManagedNamespaceDurabilityFailurePhase::BarrierOutcomeUncertain,
                error,
                ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                    ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain {
                        _absence: absence,
                        _mutation_fence: mutation_fence,
                    },
                )),
            ));
        }
    };
    let receipt = match platform::flush_namespace_directory(parent) {
        Ok(receipt) => receipt,
        Err(flush_failure) => {
            let (error, kind) = flush_failure.into_parts();
            return Err(match kind {
                PlatformNamespaceFlushFailureKind::RetryableBeforeBarrier => failure(
                    ManagedNamespaceDurabilityFailurePhase::BarrierFailed,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::RetryBeforeBarrier(
                        ManagedNamespacePreBarrierRetry {
                            disposition: absence.into_disposition(),
                            mutation_fence,
                        },
                    ),
                ),
                PlatformNamespaceFlushFailureKind::OutcomeUncertain => failure(
                    ManagedNamespaceDurabilityFailurePhase::BarrierOutcomeUncertain,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain {
                            _absence: absence,
                            _mutation_fence: mutation_fence,
                        },
                    )),
                ),
                PlatformNamespaceFlushFailureKind::PlatformUnsupported => failure(
                    ManagedNamespaceDurabilityFailurePhase::BarrierUnsupported,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::BarrierUnsupported {
                            _absence: absence,
                            _mutation_fence: mutation_fence,
                        },
                    )),
                ),
            });
        }
    };
    execute_post_barrier(ManagedNamespacePostBarrierObservationRetry {
        custody: absence.custody,
        mutation_fence,
        filesystem_kind: receipt.filesystem_kind(),
        barrier_completed_at: strictly_later_instant(std::cmp::max(
            not_before,
            pre_barrier_observed_at,
        )),
    })
}

fn observe_absence(
    retry: ManagedNamespacePreBarrierRetry,
) -> Result<
    (ManagedParentRelativeAbsence, ManagedNamespaceMutationFence),
    ManagedNamespaceDurabilityFailure,
> {
    let ManagedNamespacePreBarrierRetry {
        disposition,
        mutation_fence,
    } = retry;
    match disposition.observe_parent_relative() {
        Ok(ManagedParentRelativeObservation::Absent(absence)) => Ok((absence, mutation_fence)),
        Ok(ManagedParentRelativeObservation::ExpectedIdentityMatch(presence)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PreBarrierExpectedIdentityPresent,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_PRE_BARRIER_IDENTITY_PRESENT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PreBarrierExpectedIdentity {
                    _presence: presence,
                    _mutation_fence: mutation_fence,
                },
            )),
        )),
        Ok(ManagedParentRelativeObservation::IdentityConflict(conflict)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PreBarrierIdentityConflict,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_PRE_BARRIER_IDENTITY_CONFLICT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PreBarrierIdentityConflict {
                    _conflict: conflict,
                    _mutation_fence: mutation_fence,
                },
            )),
        )),
        Err(observation_failure) => {
            let (error, disposition, observed_object) = observation_failure.into_parts();
            match observed_object {
                Some(observed_object) => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PreBarrierObservationQuarantined,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::PreBarrierQuarantined {
                            _disposition: disposition,
                            _observed_object: observed_object,
                            _mutation_fence: mutation_fence,
                        },
                    )),
                )),
                None => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PreBarrierObservationFailed,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::RetryBeforeBarrier(
                        ManagedNamespacePreBarrierRetry {
                            disposition,
                            mutation_fence,
                        },
                    ),
                )),
            }
        }
    }
}

fn execute_post_barrier(
    retry: ManagedNamespacePostBarrierObservationRetry,
) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
    let ManagedNamespacePostBarrierObservationRetry {
        custody,
        mutation_fence,
        filesystem_kind,
        barrier_completed_at,
    } = retry;
    if let Err(error) = mutation_fence.ensure_active() {
        return Err(failure(
            ManagedNamespaceDurabilityFailurePhase::FenceOutcomeUncertain,
            error,
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainAfterBarrier {
                    _disposition: ManagedDeleteDisposition { custody },
                    _mutation_fence: mutation_fence,
                    _filesystem_kind: filesystem_kind,
                    _barrier_completed_at: barrier_completed_at,
                },
            )),
        ));
    }
    let disposition = ManagedDeleteDisposition { custody };
    match disposition.observe_parent_relative() {
        Ok(ManagedParentRelativeObservation::Absent(absence)) => {
            let post_absence_observed_at = strictly_later_instant(barrier_completed_at);
            if let Err(error) = mutation_fence.ensure_active() {
                return Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::FenceOutcomeUncertain,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainAfterBarrier {
                            _disposition: absence.into_disposition(),
                            _mutation_fence: mutation_fence,
                            _filesystem_kind: filesystem_kind,
                            _barrier_completed_at: barrier_completed_at,
                        },
                    )),
                ));
            }
            Ok(ManagedNamespaceDurable {
                custody: absence.custody,
                _mutation_fence: mutation_fence,
                filesystem_kind,
                barrier_completed_at,
                post_absence_observed_at,
                completed_at: strictly_later_instant(post_absence_observed_at),
            })
        }
        Ok(ManagedParentRelativeObservation::ExpectedIdentityMatch(presence)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PostBarrierExpectedIdentityPresent,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_POST_BARRIER_IDENTITY_PRESENT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PostBarrierExpectedIdentity {
                    _presence: presence,
                    _filesystem_kind: filesystem_kind,
                    _barrier_completed_at: barrier_completed_at,
                    _mutation_fence: mutation_fence,
                },
            )),
        )),
        Ok(ManagedParentRelativeObservation::IdentityConflict(conflict)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PostBarrierIdentityConflict,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_POST_BARRIER_IDENTITY_CONFLICT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PostBarrierIdentityConflict {
                    _conflict: conflict,
                    _filesystem_kind: filesystem_kind,
                    _barrier_completed_at: barrier_completed_at,
                    _mutation_fence: mutation_fence,
                },
            )),
        )),
        Err(observation_failure) => {
            let (error, disposition, observed_object) = observation_failure.into_parts();
            match observed_object {
                Some(observed_object) => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PostBarrierObservationQuarantined,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::PostBarrierQuarantined {
                            _disposition: disposition,
                            _observed_object: observed_object,
                            _filesystem_kind: filesystem_kind,
                            _barrier_completed_at: barrier_completed_at,
                            _mutation_fence: mutation_fence,
                        },
                    )),
                )),
                None => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PostBarrierObservationFailed,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::RetryAfterBarrier(
                        ManagedNamespacePostBarrierObservationRetry {
                            custody: disposition.custody,
                            mutation_fence,
                            filesystem_kind,
                            barrier_completed_at,
                        },
                    ),
                )),
            }
        }
    }
}
