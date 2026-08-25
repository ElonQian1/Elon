use super::super::super::model::{
    EdgeKind, Epoch, Reachability, SourceEdge, SourceEffect, SourceNodeId, ACQUIRE_OPS, ALL_OPS,
    LOCK_OPS, MAP_OPS,
};
use super::require_edge_shape;

pub(super) fn validate(edges: &[SourceEdge]) -> Result<(), &'static str> {
    validate_abandon_owner_order(edges)?;
    validate_fallback_projections(edges)?;
    validate_busy_projection(edges)
}

fn validate_abandon_owner_order(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, kind, reachability) in [
        (
            "raw.reject.abandon",
            SourceNodeId::AbiRawGate,
            SourceNodeId::AbiRawAbandon,
            EdgeKind::Abandon,
            Reachability::Conditional,
        ),
        (
            "raw.abandon.raw-state",
            SourceNodeId::AbiRawAbandon,
            SourceNodeId::AbiRawStateAbandon,
            EdgeKind::Call,
            Reachability::Required,
        ),
        (
            "raw-state.abandon.pinned-drop",
            SourceNodeId::AbiRawStateAbandon,
            SourceNodeId::RegistryPinnedDrop,
            EdgeKind::UnwindRetention,
            Reachability::Conditional,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            kind,
            ALL_OPS,
            Epoch::AbiInput,
            reachability,
            SourceEffect::RetainCustody,
        )?;
    }
    Ok(())
}

fn validate_fallback_projections(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, ops, reachability) in [
        (
            "map.raw-abandon.fallback",
            SourceNodeId::AbiRawStateAbandon,
            SourceNodeId::AbiMapFallbackProjection,
            MAP_OPS,
            Reachability::Required,
        ),
        (
            "map.fallback.unavailable-code",
            SourceNodeId::AbiMapFallbackProjection,
            SourceNodeId::AbiMapUnavailableCode,
            MAP_OPS,
            Reachability::Required,
        ),
        (
            "lock.raw-abandon.fallback",
            SourceNodeId::AbiRawStateAbandon,
            SourceNodeId::AbiLockFallbackProjection,
            LOCK_OPS,
            Reachability::Required,
        ),
        (
            "lock.fallback.unavailable-code",
            SourceNodeId::AbiLockFallbackProjection,
            SourceNodeId::AbiLockUnavailableCode,
            LOCK_OPS,
            Reachability::Required,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            EdgeKind::ResultProjection,
            ops,
            Epoch::AbiInput,
            reachability,
            SourceEffect::None,
        )?;
    }
    Ok(())
}

fn validate_busy_projection(edges: &[SourceEdge]) -> Result<(), &'static str> {
    require_edge_shape(
        edges,
        "lock.registry.busy-projection",
        SourceNodeId::RegistryLockBusyProjection,
        SourceNodeId::AbiLockBusyProjection,
        EdgeKind::ResultProjection,
        ACQUIRE_OPS,
        Epoch::AbiInput,
        Reachability::Required,
        SourceEffect::None,
    )
}
