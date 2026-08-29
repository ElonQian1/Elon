//! Frozen identity projection for a runtime-validated final-connection Unmap outcome.

use super::super::super::{
    a2b2_cases::{
        UnmapActualIdentity, UnmapActualTarget, UnmapCallback, UnmapCause, UnmapFailureClass,
        UnmapMode, UnmapNode, UnmapPath, UnmapPhase, UnmapRole, UnmapSelector, UnmapSqliteOutcome,
        UnmapTargetScope, UnmapTiming, UnmapTopology,
    },
    ManagedTestShmTargetWitness,
};

pub(super) fn supports(selector: UnmapSelector) -> bool {
    !matches!(
        selector,
        UnmapSelector::SharedDeleteRequestValidation
            | UnmapSelector::SharedKeepCallbackAdmission
            | UnmapSelector::SharedKeepCallbackWrapperBefore
            | UnmapSelector::SharedKeepHeldSharedLock
            | UnmapSelector::SharedKeepHeldExclusiveLock
            | UnmapSelector::SharedKeepDetachBefore
            | UnmapSelector::SharedKeepDetachAfterKnown
            | UnmapSelector::SharedKeepDetachAfterUncertain
            | UnmapSelector::SharedKeepCompletionNativeUncertain
            | UnmapSelector::SharedKeepSuccess
            | UnmapSelector::SharedDeleteSuccess
    )
}

pub(super) fn is_delete(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::FinalDeleteAuthMainIdentityMissing
            | UnmapSelector::FinalDeleteAuthMainOrGenerationMismatch
            | UnmapSelector::FinalDeleteAuthMainNotExclusive
            | UnmapSelector::FinalDeleteAuthLockStateUncertain
            | UnmapSelector::FinalDeleteSiblingBefore
            | UnmapSelector::FinalDeleteSiblingNativeRetryable
            | UnmapSelector::FinalDeleteSiblingNativeUncertain
            | UnmapSelector::FinalDeleteSiblingAfterKnown
            | UnmapSelector::FinalDeleteSiblingAfterUncertain
            | UnmapSelector::FinalDeleteDetachBefore
            | UnmapSelector::FinalDeleteDetachAfterKnown
            | UnmapSelector::FinalDeleteDetachAfterUncertain
            | UnmapSelector::FinalDeleteCompletionNativeUncertain
            | UnmapSelector::FinalDeleteSuccessDeleted
            | UnmapSelector::FinalDeleteSuccessNotFound
    )
}

pub(super) fn node_absent(selector: UnmapSelector) -> bool {
    selector == UnmapSelector::FinalKeepSuccessNodeAbsent
}

pub(super) fn requires_main_exclusive(selector: UnmapSelector) -> bool {
    is_delete(selector) && selector != UnmapSelector::FinalDeleteAuthMainNotExclusive
}

pub(super) fn is_success(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::FinalKeepSuccessLiveNode
            | UnmapSelector::FinalKeepSuccessNodeAbsent
            | UnmapSelector::FinalDeleteSuccessDeleted
            | UnmapSelector::FinalDeleteSuccessNotFound
    )
}

pub(super) fn route_terminal(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::FinalKeepViewUnmapNativeUncertain
            | UnmapSelector::FinalKeepViewUnmapAfterKnown
            | UnmapSelector::FinalKeepViewUnmapAfterUncertain
            | UnmapSelector::FinalKeepMappingCloseBefore
            | UnmapSelector::FinalKeepMappingCloseNativeUncertain
            | UnmapSelector::FinalKeepMappingCloseAfterKnown
            | UnmapSelector::FinalKeepMappingCloseAfterUncertain
            | UnmapSelector::FinalKeepDmsReleaseBefore
            | UnmapSelector::FinalKeepDmsReleaseNativeUncertain
            | UnmapSelector::FinalKeepDmsReleaseAfterKnown
            | UnmapSelector::FinalKeepDmsReleaseAfterUncertain
            | UnmapSelector::FinalKeepFileCloseBefore
            | UnmapSelector::FinalKeepFileCloseNativeRetryable
            | UnmapSelector::FinalKeepFileCloseNativeUncertain
            | UnmapSelector::FinalKeepFileCloseAfterKnown
            | UnmapSelector::FinalKeepFileCloseAfterUncertain
            | UnmapSelector::FinalKeepDetachBefore
            | UnmapSelector::FinalKeepDetachAfterKnown
            | UnmapSelector::FinalKeepDetachAfterUncertain
            | UnmapSelector::FinalKeepCompletionNativeUncertain
            | UnmapSelector::FinalDeleteAuthLockStateUncertain
            | UnmapSelector::FinalDeleteSiblingBefore
            | UnmapSelector::FinalDeleteSiblingNativeRetryable
            | UnmapSelector::FinalDeleteSiblingNativeUncertain
            | UnmapSelector::FinalDeleteSiblingAfterKnown
            | UnmapSelector::FinalDeleteSiblingAfterUncertain
            | UnmapSelector::FinalDeleteDetachBefore
            | UnmapSelector::FinalDeleteDetachAfterKnown
            | UnmapSelector::FinalDeleteDetachAfterUncertain
            | UnmapSelector::FinalDeleteCompletionNativeUncertain
    )
}

pub(super) fn domain_terminal(selector: UnmapSelector) -> bool {
    route_terminal(selector)
        && !matches!(
            selector,
            UnmapSelector::FinalKeepCompletionNativeUncertain
                | UnmapSelector::FinalDeleteCompletionNativeUncertain
        )
}

pub(super) fn completion_succeeds(selector: UnmapSelector) -> bool {
    !route_terminal(selector)
}

pub(super) fn into_identity(
    selector: UnmapSelector,
    target: ManagedTestShmTargetWitness,
    pre_shared_mask: u8,
    pre_exclusive_mask: u8,
) -> UnmapActualIdentity {
    use UnmapFailureClass as C;
    use UnmapPhase as P;
    use UnmapSelector as S;
    use UnmapTiming as T;

    let (phase, timing, class, variant) = match selector {
        S::FinalKeepViewUnmapBefore => (P::ViewUnmap, T::BeforeCall, C::IoBeforeMutation, 0),
        S::FinalKeepViewUnmapNativeUncertain => (
            P::ViewUnmap,
            T::NativeUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepViewUnmapAfterKnown => {
            (P::ViewUnmap, T::AfterSuccessKnown, C::MutatedButKnown, 0)
        }
        S::FinalKeepViewUnmapAfterUncertain => (
            P::ViewUnmap,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepMappingCloseBefore => (P::MappingClose, T::BeforeCall, C::MutatedButKnown, 0),
        S::FinalKeepMappingCloseNativeUncertain => (
            P::MappingClose,
            T::NativeUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepMappingCloseAfterKnown => {
            (P::MappingClose, T::AfterSuccessKnown, C::MutatedButKnown, 0)
        }
        S::FinalKeepMappingCloseAfterUncertain => (
            P::MappingClose,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepDmsReleaseBefore => (P::DmsSharedRelease, T::BeforeCall, C::MutatedButKnown, 0),
        S::FinalKeepDmsReleaseNativeUncertain => (
            P::DmsSharedRelease,
            T::NativeUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepDmsReleaseAfterKnown => (
            P::DmsSharedRelease,
            T::AfterSuccessKnown,
            C::MutatedButKnown,
            0,
        ),
        S::FinalKeepDmsReleaseAfterUncertain => (
            P::DmsSharedRelease,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepFileCloseBefore => (P::ShmFileClose, T::BeforeCall, C::MutatedButKnown, 0),
        S::FinalKeepFileCloseNativeRetryable => (
            P::ShmFileClose,
            T::NativeRetryable,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepFileCloseNativeUncertain => (
            P::ShmFileClose,
            T::NativeUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepFileCloseAfterKnown => {
            (P::ShmFileClose, T::AfterSuccessKnown, C::MutatedButKnown, 0)
        }
        S::FinalKeepFileCloseAfterUncertain => (
            P::ShmFileClose,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepDetachBefore => (P::ConnectionDetach, T::BeforeCall, C::MutatedButKnown, 0),
        S::FinalKeepDetachAfterKnown => (
            P::ConnectionDetach,
            T::AfterSuccessKnown,
            C::MutatedButKnown,
            0,
        ),
        S::FinalKeepDetachAfterUncertain => (
            P::ConnectionDetach,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalKeepCompletionNativeUncertain => (
            P::CallbackCompletion,
            T::NativeUncertain,
            C::RegistryRejected,
            0,
        ),
        S::FinalKeepSuccessLiveNode | S::FinalKeepSuccessNodeAbsent => {
            (P::Success, T::Success, C::None, 0)
        }
        S::FinalDeleteAuthMainIdentityMissing => (
            P::DeleteAuthorization,
            T::Validation,
            C::ProtocolViolation,
            1,
        ),
        S::FinalDeleteAuthMainOrGenerationMismatch => (
            P::DeleteAuthorization,
            T::Validation,
            C::ProtocolViolation,
            2,
        ),
        S::FinalDeleteAuthMainNotExclusive => (
            P::DeleteAuthorization,
            T::Validation,
            C::ProtocolViolation,
            3,
        ),
        S::FinalDeleteAuthLockStateUncertain => (
            P::DeleteAuthorization,
            T::Validation,
            C::OutcomeUncertainPoisoned,
            4,
        ),
        S::FinalDeleteSiblingBefore => {
            (P::ExactSiblingDelete, T::BeforeCall, C::MutatedButKnown, 0)
        }
        S::FinalDeleteSiblingNativeRetryable => (
            P::ExactSiblingDelete,
            T::NativeRetryable,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalDeleteSiblingNativeUncertain => (
            P::ExactSiblingDelete,
            T::NativeUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalDeleteSiblingAfterKnown => (
            P::ExactSiblingDelete,
            T::AfterSuccessKnown,
            C::MutatedButKnown,
            0,
        ),
        S::FinalDeleteSiblingAfterUncertain => (
            P::ExactSiblingDelete,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            0,
        ),
        S::FinalDeleteDetachBefore => (P::ConnectionDetach, T::BeforeCall, C::MutatedButKnown, 1),
        S::FinalDeleteDetachAfterKnown => (
            P::ConnectionDetach,
            T::AfterSuccessKnown,
            C::MutatedButKnown,
            1,
        ),
        S::FinalDeleteDetachAfterUncertain => (
            P::ConnectionDetach,
            T::AfterSuccessUncertain,
            C::OutcomeUncertainPoisoned,
            1,
        ),
        S::FinalDeleteCompletionNativeUncertain => (
            P::CallbackCompletion,
            T::NativeUncertain,
            C::RegistryRejected,
            1,
        ),
        S::FinalDeleteSuccessDeleted => (P::Success, T::Success, C::None, 0),
        S::FinalDeleteSuccessNotFound => (P::Success, T::Success, C::None, 1),
        _ => unreachable!("SharedNonFinal selector cannot become a final outcome"),
    };
    UnmapActualIdentity {
        path: UnmapPath::Unmap,
        topology: UnmapTopology::FinalConnection,
        mode: if is_delete(selector) {
            UnmapMode::Delete
        } else {
            UnmapMode::Keep
        },
        node: if node_absent(selector) {
            UnmapNode::Absent
        } else {
            UnmapNode::Live
        },
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
        sqlite_outcome: if is_success(selector) {
            UnmapSqliteOutcome::Ok
        } else {
            UnmapSqliteOutcome::Ioerr
        },
    }
}
