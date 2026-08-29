//! Sealed SharedNonFinal outcome identities derived from runtime event shapes.

use super::super::{
    a2b2_cases::{
        UnmapActualIdentity, UnmapActualTarget, UnmapCallback, UnmapCause, UnmapFailureClass,
        UnmapMode, UnmapNode, UnmapPath, UnmapPhase, UnmapRole, UnmapSelector, UnmapSqliteOutcome,
        UnmapTargetScope, UnmapTiming, UnmapTopology,
    },
    ManagedTestShmTargetWitness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObservedSharedUnmapOutcome {
    RequestValidation,
    AdmissionRejected,
    WrapperBefore,
    HeldSharedLock,
    HeldExclusiveLock,
    DetachBefore,
    DetachAfterKnown,
    DetachAfterUncertain,
    CompletionNativeUncertain,
    KeepSuccess,
    DeleteSuccess,
}

impl ObservedSharedUnmapOutcome {
    pub(super) fn selector(self) -> UnmapSelector {
        use ObservedSharedUnmapOutcome as O;
        match self {
            O::RequestValidation => UnmapSelector::SharedDeleteRequestValidation,
            O::AdmissionRejected => UnmapSelector::SharedKeepCallbackAdmission,
            O::WrapperBefore => UnmapSelector::SharedKeepCallbackWrapperBefore,
            O::HeldSharedLock => UnmapSelector::SharedKeepHeldSharedLock,
            O::HeldExclusiveLock => UnmapSelector::SharedKeepHeldExclusiveLock,
            O::DetachBefore => UnmapSelector::SharedKeepDetachBefore,
            O::DetachAfterKnown => UnmapSelector::SharedKeepDetachAfterKnown,
            O::DetachAfterUncertain => UnmapSelector::SharedKeepDetachAfterUncertain,
            O::CompletionNativeUncertain => UnmapSelector::SharedKeepCompletionNativeUncertain,
            O::KeepSuccess => UnmapSelector::SharedKeepSuccess,
            O::DeleteSuccess => UnmapSelector::SharedDeleteSuccess,
        }
    }

    pub(super) fn into_identity(
        self,
        target: ManagedTestShmTargetWitness,
        pre_shared_mask: u8,
        pre_exclusive_mask: u8,
    ) -> UnmapActualIdentity {
        use ObservedSharedUnmapOutcome as O;
        let (mode, variant, phase, timing, class) = match self {
            O::RequestValidation => (
                UnmapMode::Delete,
                0,
                UnmapPhase::RequestValidation,
                UnmapTiming::Validation,
                UnmapFailureClass::ProtocolViolation,
            ),
            O::AdmissionRejected => (
                UnmapMode::Keep,
                0,
                UnmapPhase::CallbackAdmission,
                UnmapTiming::BeforeCall,
                UnmapFailureClass::RegistryRejected,
            ),
            O::WrapperBefore => (
                UnmapMode::Keep,
                1,
                UnmapPhase::ConnectionDetach,
                UnmapTiming::BeforeCall,
                UnmapFailureClass::IoBeforeMutation,
            ),
            O::HeldSharedLock | O::HeldExclusiveLock => (
                UnmapMode::Keep,
                0,
                UnmapPhase::HeldLockGate,
                UnmapTiming::Validation,
                UnmapFailureClass::ProtocolViolation,
            ),
            O::DetachBefore => (
                UnmapMode::Keep,
                0,
                UnmapPhase::ConnectionDetach,
                UnmapTiming::BeforeCall,
                UnmapFailureClass::IoBeforeMutation,
            ),
            O::DetachAfterKnown => (
                UnmapMode::Keep,
                0,
                UnmapPhase::ConnectionDetach,
                UnmapTiming::AfterSuccessKnown,
                UnmapFailureClass::MutatedButKnown,
            ),
            O::DetachAfterUncertain => (
                UnmapMode::Keep,
                0,
                UnmapPhase::ConnectionDetach,
                UnmapTiming::AfterSuccessUncertain,
                UnmapFailureClass::OutcomeUncertainPoisoned,
            ),
            O::CompletionNativeUncertain => (
                UnmapMode::Keep,
                0,
                UnmapPhase::CallbackCompletion,
                UnmapTiming::NativeUncertain,
                UnmapFailureClass::RegistryRejected,
            ),
            O::KeepSuccess => (
                UnmapMode::Keep,
                0,
                UnmapPhase::Success,
                UnmapTiming::Success,
                UnmapFailureClass::None,
            ),
            O::DeleteSuccess => (
                UnmapMode::Delete,
                0,
                UnmapPhase::Success,
                UnmapTiming::Success,
                UnmapFailureClass::None,
            ),
        };
        UnmapActualIdentity {
            path: UnmapPath::Unmap,
            topology: UnmapTopology::SharedNonFinal,
            mode,
            node: UnmapNode::Live,
            variant,
            pre_shared_mask,
            pre_exclusive_mask,
            phase,
            cause: UnmapCause::None,
            timing,
            class,
            target: UnmapActualTarget {
                scope: UnmapTargetScope::RouteMain,
                registration_id: target.registration_id(),
                route_ordinal: target.route_ordinal(),
                runtime_generation: target.runtime_generation(),
                shm_connection_id: target.shm_connection_id(),
                role: UnmapRole::Main,
                callback: UnmapCallback::Shm,
                occurrence: 1,
            },
            sqlite_outcome: if self.is_success() {
                UnmapSqliteOutcome::Ok
            } else {
                UnmapSqliteOutcome::Ioerr
            },
        }
    }

    pub(super) fn is_success(self) -> bool {
        matches!(self, Self::KeepSuccess | Self::DeleteSuccess)
    }

    pub(super) fn callback_began(self) -> bool {
        !matches!(
            self,
            Self::RequestValidation | Self::AdmissionRejected | Self::WrapperBefore
        )
    }

    pub(super) fn completion_succeeded(self) -> bool {
        matches!(
            self,
            Self::HeldSharedLock
                | Self::HeldExclusiveLock
                | Self::DetachBefore
                | Self::KeepSuccess
                | Self::DeleteSuccess
        )
    }

    pub(super) fn action_succeeded(self) -> bool {
        matches!(
            self,
            Self::DetachAfterKnown
                | Self::DetachAfterUncertain
                | Self::CompletionNativeUncertain
                | Self::KeepSuccess
                | Self::DeleteSuccess
        )
    }

    pub(super) fn route_terminal(self) -> bool {
        matches!(
            self,
            Self::AdmissionRejected
                | Self::DetachAfterKnown
                | Self::DetachAfterUncertain
                | Self::CompletionNativeUncertain
        )
    }

    pub(super) fn domain_terminal(self) -> bool {
        matches!(self, Self::DetachAfterKnown | Self::DetachAfterUncertain)
    }
}
