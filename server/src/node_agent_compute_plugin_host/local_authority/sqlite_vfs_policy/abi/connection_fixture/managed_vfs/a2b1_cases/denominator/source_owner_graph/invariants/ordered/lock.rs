use super::super::super::model::{
    EdgeKind, Epoch, Reachability, SourceEdge, SourceEffect, SourceNodeId, ACQUIRE_OPS,
    LOCK_EXCLUSIVE_OPS, LOCK_OPS, LOCK_SHARED_OPS, SHARED_LOCK_OPS, UNLOCK_EXCLUSIVE_OPS,
    UNLOCK_SHARED_OPS,
};
use super::require_edge_shape;

pub(super) fn validate(edges: &[SourceEdge]) -> Result<(), &'static str> {
    require_edge_shape(
        edges,
        "lock.coordinator.local-gates",
        SourceNodeId::ManagedLockCoordinator,
        SourceNodeId::ManagedLockLocalGate,
        EdgeKind::Call,
        LOCK_OPS,
        Epoch::WalMainSteady,
        Reachability::Conditional,
        SourceEffect::None,
    )?;
    require_edge_shape(
        edges,
        "lock.acquire.ensure-node",
        SourceNodeId::ManagedLockAcquire,
        SourceNodeId::ManagedEnsureNode,
        EdgeKind::ConditionalCall,
        ACQUIRE_OPS,
        Epoch::ColdNodeAcquirePrefix,
        Reachability::Required,
        SourceEffect::None,
    )?;
    validate_exclusive_range_mutation(edges)?;
    validate_after_success_fault(edges)?;
    validate_completion(edges)
}

fn validate_exclusive_range_mutation(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, ops) in [
        ("lock.acquire.exclusive-range", LOCK_EXCLUSIVE_OPS),
        ("lock.release.exclusive-range", UNLOCK_EXCLUSIVE_OPS),
    ] {
        require_edge_shape(
            edges,
            id,
            SourceNodeId::ManagedLockPlatformExclusiveMasks,
            SourceNodeId::ManagedLockExclusiveRanges,
            EdgeKind::MutationBeforeContinuation,
            ops,
            Epoch::WalMainSteady,
            Reachability::Required,
            SourceEffect::LocalMaskMutation,
        )?;
    }
    Ok(())
}

fn validate_after_success_fault(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, ops) in [
        (
            "lock.acquire.shared-after-fault",
            SourceNodeId::ManagedLockPlatformSharedMasks,
            LOCK_SHARED_OPS,
        ),
        (
            "lock.acquire.exclusive-after-fault",
            SourceNodeId::ManagedLockExclusiveRanges,
            LOCK_EXCLUSIVE_OPS,
        ),
        (
            "lock.release.shared-after-fault",
            SourceNodeId::ManagedLockPlatformSharedMasks,
            UNLOCK_SHARED_OPS,
        ),
        (
            "lock.release.exclusive-after-fault",
            SourceNodeId::ManagedLockExclusiveRanges,
            UNLOCK_EXCLUSIVE_OPS,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            SourceNodeId::ManagedFaultFinish,
            EdgeKind::Call,
            ops,
            Epoch::WalMainSteady,
            Reachability::Required,
            SourceEffect::None,
        )?;
    }
    Ok(())
}

fn validate_completion(edges: &[SourceEdge]) -> Result<(), &'static str> {
    require_edge_shape(
        edges,
        "lock.success.complete",
        SourceNodeId::ManagedFaultFinish,
        SourceNodeId::RegistryCallbackComplete,
        EdgeKind::CallbackCompletion,
        LOCK_OPS,
        Epoch::WalMainSteady,
        Reachability::Required,
        SourceEffect::CallbackLease,
    )?;
    require_edge_shape(
        edges,
        "lock.local-shared.complete",
        SourceNodeId::ManagedLockLocalSharedMasks,
        SourceNodeId::RegistryCallbackComplete,
        EdgeKind::CallbackCompletion,
        SHARED_LOCK_OPS,
        Epoch::WalMainSteady,
        Reachability::Required,
        SourceEffect::CallbackLease,
    )
}
