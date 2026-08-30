//! Projection from a sealed real-boundary observation into the wire identity.

use super::{
    super::{
        a2b2_cases::{
            JointCloseActualIdentity, JointCloseActualTarget, JointCloseCallback, JointCloseMode,
            JointCloseNode, JointClosePath, JointCloseRole, JointCloseSelector as S,
            JointCloseTargetScope, JointCloseTopology,
        },
        ManagedTestShmTargetWitness,
    },
    boundary::SealedJointCloseBoundary,
};

pub(super) fn identity(
    boundary: SealedJointCloseBoundary,
    target: ManagedTestShmTargetWitness,
) -> JointCloseActualIdentity {
    JointCloseActualIdentity {
        path: JointClosePath::JointClose,
        topology: JointCloseTopology::FinalConnection,
        mode: JointCloseMode::Keep,
        node: JointCloseNode::Live,
        variant: boundary.variant(),
        pre_shared_mask: 0,
        pre_exclusive_mask: 0,
        main_lock_prestate: boundary.main_lock_prestate(),
        main_lock_offset_class: boundary.main_lock_offset_class(),
        phase: boundary.phase(),
        cause: boundary.cause(),
        timing: boundary.timing(),
        class: boundary.class(),
        target: JointCloseActualTarget {
            scope: JointCloseTargetScope::RouteMain,
            registration_id: target.registration_id(),
            route_ordinal: target.route_ordinal(),
            runtime_generation: target.runtime_generation(),
            shm_connection_id: target.shm_connection_id(),
            role: JointCloseRole::Main,
            callback: JointCloseCallback::Close,
            occurrence: 1,
        },
        sqlite_outcome: boundary.sqlite_outcome(),
    }
}

pub(super) fn is_shm(selector: S) -> bool {
    matches!(
        selector,
        S::ShmViewUnmapBefore
            | S::ShmViewUnmapNativeUncertain
            | S::ShmViewUnmapAfterKnown
            | S::ShmViewUnmapAfterUncertain
            | S::ShmMappingCloseBefore
            | S::ShmMappingCloseNativeUncertain
            | S::ShmMappingCloseAfterKnown
            | S::ShmMappingCloseAfterUncertain
            | S::ShmDmsReleaseBefore
            | S::ShmDmsReleaseNativeUncertain
            | S::ShmDmsReleaseAfterKnown
            | S::ShmDmsReleaseAfterUncertain
            | S::ShmFileCloseBefore
            | S::ShmFileCloseNativeRetryable
            | S::ShmFileCloseNativeUncertain
            | S::ShmFileCloseAfterKnown
            | S::ShmFileCloseAfterUncertain
            | S::ShmDetachBefore
            | S::ShmDetachAfterKnown
            | S::ShmDetachAfterUncertain
    )
}

pub(super) fn is_main(selector: S) -> bool {
    matches!(
        selector,
        S::MainLockReleaseBefore
            | S::MainLockReleaseNativeUncertainShared
            | S::MainLockReleaseNativeUncertainReserved
            | S::MainLockReleaseAfterKnown
            | S::MainFileCloseBefore
            | S::MainFileCloseNativeRetryable
            | S::MainFileCloseNativeUncertain
            | S::MainFileCloseAfterKnown
    )
}

pub(super) fn is_registry(selector: S) -> bool {
    matches!(
        selector,
        S::RegistryWalMainCloseBefore
            | S::RegistryWalMainCloseNativeUncertain
            | S::RegistryWalMainCloseAfterKnown
    )
}

pub(super) fn observes_physical_actions(selector: S) -> bool {
    is_shm(selector) || is_main(selector) || selector == S::PhysicalSuccess || is_registry(selector)
}
