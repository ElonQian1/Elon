use std::{
    convert::Infallible,
    error::Error as StdError,
    fmt,
    time::{Duration, Instant},
};

use super::{
    lifecycle::{
        ManagedDeleteDisposition, ManagedExpectedIdentityMatchPresence,
        ManagedParentNamespaceCustody, ManagedParentRelativeAbsence,
        ManagedParentRelativeIdentityConflict, ManagedParentRelativeObservation,
        QuarantinedManagedNamespaceObject,
    },
    ManagedObjectBinding,
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
    filesystem_kind: &'static str,
    barrier_completed_at: Instant,
    post_absence_observed_at: Instant,
    completed_at: Instant,
}

/// Uninhabited until the platform supplies an OS-enforced child-namespace mutation fence. Keeping
/// this as an explicit parameter prevents pre/flush/post evidence from being minted around an ABA.
pub(crate) struct ManagedNamespaceMutationFence {
    _unavailable: Infallible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedNamespaceDurabilityFailurePhase {
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
}

/// The native flush completed, so this capability may only retry the post-barrier observation.
#[must_use = "post-barrier retry must not repeat the namespace durability barrier"]
pub(crate) struct ManagedNamespacePostBarrierObservationRetry {
    custody: ManagedParentNamespaceCustody,
    filesystem_kind: &'static str,
    barrier_completed_at: Instant,
}

/// Opaque terminal custody for an opened same-name object or an uncertain native barrier.
#[must_use = "terminal durability custody must remain retained for operator recovery"]
pub(crate) struct ManagedNamespaceDurabilityRetainedCustody {
    inner: ManagedNamespaceDurabilityRetainedInner,
}

enum ManagedNamespaceDurabilityRetainedInner {
    PreBarrierQuarantined {
        _disposition: ManagedDeleteDisposition,
        _observed_object: QuarantinedManagedNamespaceObject,
    },
    PreBarrierExpectedIdentity(ManagedExpectedIdentityMatchPresence),
    PreBarrierIdentityConflict(ManagedParentRelativeIdentityConflict),
    BarrierOutcomeUncertain(ManagedParentRelativeAbsence),
    BarrierUnsupported(ManagedParentRelativeAbsence),
    PostBarrierQuarantined {
        _disposition: ManagedDeleteDisposition,
        _observed_object: QuarantinedManagedNamespaceObject,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
    },
    PostBarrierExpectedIdentity {
        _presence: ManagedExpectedIdentityMatchPresence,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
    },
    PostBarrierIdentityConflict {
        _conflict: ManagedParentRelativeIdentityConflict,
        _filesystem_kind: &'static str,
        _barrier_completed_at: Instant,
    },
}

impl ManagedParentRelativeAbsence {
    pub(crate) fn make_namespace_durable(
        self,
        mutation_fence: &ManagedNamespaceMutationFence,
        not_before: Instant,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        execute_pre_barrier(
            ManagedNamespacePreBarrierRetry {
                disposition: self.into_disposition(),
            },
            not_before,
            mutation_fence,
        )
    }
}

impl ManagedNamespacePreBarrierRetry {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        self.disposition.object_binding()
    }

    pub(crate) fn retry_pre_barrier(
        self,
        mutation_fence: &ManagedNamespaceMutationFence,
        not_before: Instant,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        execute_pre_barrier(self, not_before, mutation_fence)
    }
}

impl ManagedNamespacePostBarrierObservationRetry {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn retry_post_barrier_observation(
        self,
        mutation_fence: &ManagedNamespaceMutationFence,
    ) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
        execute_post_barrier(self, mutation_fence)
    }
}

fn execute_pre_barrier(
    retry: ManagedNamespacePreBarrierRetry,
    not_before: Instant,
    mutation_fence: &ManagedNamespaceMutationFence,
) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
    let absence = match observe_absence(retry) {
        Ok(absence) => absence,
        Err(failure) => return Err(failure),
    };
    let pre_barrier_observed_at = Instant::now();
    let parent = match absence.custody.parent_handle() {
        Ok(parent) => parent,
        Err(error) => {
            return Err(failure(
                ManagedNamespaceDurabilityFailurePhase::BarrierOutcomeUncertain,
                error,
                ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                    ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain(absence),
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
                        },
                    ),
                ),
                PlatformNamespaceFlushFailureKind::OutcomeUncertain => failure(
                    ManagedNamespaceDurabilityFailurePhase::BarrierOutcomeUncertain,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain(absence),
                    )),
                ),
                PlatformNamespaceFlushFailureKind::PlatformUnsupported => failure(
                    ManagedNamespaceDurabilityFailurePhase::BarrierUnsupported,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                        ManagedNamespaceDurabilityRetainedInner::BarrierUnsupported(absence),
                    )),
                ),
            });
        }
    };
    execute_post_barrier(
        ManagedNamespacePostBarrierObservationRetry {
            custody: absence.custody,
            filesystem_kind: receipt.filesystem_kind(),
            barrier_completed_at: strictly_later_instant(std::cmp::max(
                not_before,
                pre_barrier_observed_at,
            )),
        },
        mutation_fence,
    )
}

fn observe_absence(
    retry: ManagedNamespacePreBarrierRetry,
) -> Result<ManagedParentRelativeAbsence, ManagedNamespaceDurabilityFailure> {
    let disposition = retry.disposition;
    match disposition.observe_parent_relative() {
        Ok(ManagedParentRelativeObservation::Absent(absence)) => Ok(absence),
        Ok(ManagedParentRelativeObservation::ExpectedIdentityMatch(presence)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PreBarrierExpectedIdentityPresent,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_PRE_BARRIER_IDENTITY_PRESENT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PreBarrierExpectedIdentity(presence),
            )),
        )),
        Ok(ManagedParentRelativeObservation::IdentityConflict(conflict)) => Err(failure(
            ManagedNamespaceDurabilityFailurePhase::PreBarrierIdentityConflict,
            std::io::Error::other("NODE_MANAGED_NAMESPACE_PRE_BARRIER_IDENTITY_CONFLICT"),
            ManagedNamespaceDurabilityFailureCustody::Retained(retained(
                ManagedNamespaceDurabilityRetainedInner::PreBarrierIdentityConflict(conflict),
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
                        },
                    )),
                )),
                None => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PreBarrierObservationFailed,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::RetryBeforeBarrier(
                        ManagedNamespacePreBarrierRetry { disposition },
                    ),
                )),
            }
        }
    }
}

fn execute_post_barrier(
    retry: ManagedNamespacePostBarrierObservationRetry,
    _mutation_fence: &ManagedNamespaceMutationFence,
) -> Result<ManagedNamespaceDurable, ManagedNamespaceDurabilityFailure> {
    let ManagedNamespacePostBarrierObservationRetry {
        custody,
        filesystem_kind,
        barrier_completed_at,
    } = retry;
    let disposition = ManagedDeleteDisposition { custody };
    match disposition.observe_parent_relative() {
        Ok(ManagedParentRelativeObservation::Absent(absence)) => {
            let post_absence_observed_at = strictly_later_instant(barrier_completed_at);
            Ok(ManagedNamespaceDurable {
                custody: absence.custody,
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
                        },
                    )),
                )),
                None => Err(failure(
                    ManagedNamespaceDurabilityFailurePhase::PostBarrierObservationFailed,
                    error,
                    ManagedNamespaceDurabilityFailureCustody::RetryAfterBarrier(
                        ManagedNamespacePostBarrierObservationRetry {
                            custody: disposition.custody,
                            filesystem_kind,
                            barrier_completed_at,
                        },
                    ),
                )),
            }
        }
    }
}

fn retained(
    inner: ManagedNamespaceDurabilityRetainedInner,
) -> ManagedNamespaceDurabilityRetainedCustody {
    ManagedNamespaceDurabilityRetainedCustody { inner }
}

fn failure(
    phase: ManagedNamespaceDurabilityFailurePhase,
    error: std::io::Error,
    custody: ManagedNamespaceDurabilityFailureCustody,
) -> ManagedNamespaceDurabilityFailure {
    ManagedNamespaceDurabilityFailure {
        phase,
        error,
        custody,
    }
}

fn strictly_later_instant(previous: Instant) -> Instant {
    let now = Instant::now();
    if now > previous {
        now
    } else {
        previous.checked_add(Duration::from_nanos(1)).unwrap_or(now)
    }
}

impl ManagedNamespaceDurable {
    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn namespace_durability_kind(&self) -> &'static str {
        WINDOWS_NAMESPACE_DURABILITY_KIND
    }

    pub(crate) fn filesystem_kind(&self) -> &'static str {
        self.filesystem_kind
    }

    pub(crate) fn barrier_completed_at(&self) -> Instant {
        self.barrier_completed_at
    }

    pub(crate) fn post_absence_observed_at(&self) -> Instant {
        self.post_absence_observed_at
    }

    pub(crate) fn completed_at(&self) -> Instant {
        self.completed_at
    }
}

impl ManagedNamespaceDurabilityFailure {
    pub(crate) fn phase(&self) -> ManagedNamespaceDurabilityFailurePhase {
        self.phase
    }

    pub(crate) fn into_parts(self) -> (std::io::Error, ManagedNamespaceDurabilityFailureCustody) {
        (self.error, self.custody)
    }
}

impl fmt::Debug for ManagedNamespaceDurable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurable")
            .field("parent_handle", &"<retained>")
            .field("durability_kind", &WINDOWS_NAMESPACE_DURABILITY_KIND)
            .field("filesystem_kind", &self.filesystem_kind)
            .field("barrier_completed_at", &self.barrier_completed_at)
            .field("post_absence_observed_at", &self.post_absence_observed_at)
            .field("completed_at", &self.completed_at)
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityRetainedCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let phase = match &self.inner {
            ManagedNamespaceDurabilityRetainedInner::PreBarrierQuarantined { .. } => {
                "pre_barrier_quarantined"
            }
            ManagedNamespaceDurabilityRetainedInner::PreBarrierExpectedIdentity(_) => {
                "pre_barrier_expected_identity"
            }
            ManagedNamespaceDurabilityRetainedInner::PreBarrierIdentityConflict(_) => {
                "pre_barrier_identity_conflict"
            }
            ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain(_) => {
                "barrier_outcome_uncertain"
            }
            ManagedNamespaceDurabilityRetainedInner::BarrierUnsupported(_) => "barrier_unsupported",
            ManagedNamespaceDurabilityRetainedInner::PostBarrierQuarantined { .. } => {
                "post_barrier_quarantined"
            }
            ManagedNamespaceDurabilityRetainedInner::PostBarrierExpectedIdentity { .. } => {
                "post_barrier_expected_identity"
            }
            ManagedNamespaceDurabilityRetainedInner::PostBarrierIdentityConflict { .. } => {
                "post_barrier_identity_conflict"
            }
        };
        formatter
            .debug_struct("ManagedNamespaceDurabilityRetainedCustody")
            .field("phase", &phase)
            .field("handles", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityFailureCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let custody = match self {
            Self::RetryBeforeBarrier(_) => "retry_before_barrier",
            Self::RetryAfterBarrier(_) => "retry_after_barrier",
            Self::Retained(_) => "retained_terminal",
        };
        formatter
            .debug_struct("ManagedNamespaceDurabilityFailureCustody")
            .field("custody", &custody)
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurabilityFailure")
            .field("phase", &self.phase)
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("custody", &self.custody)
            .finish()
    }
}

impl fmt::Display for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NODE_MANAGED_NAMESPACE_DURABILITY_FAILED: {}",
            self.error
        )
    }
}

impl StdError for ManagedNamespaceDurabilityFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
