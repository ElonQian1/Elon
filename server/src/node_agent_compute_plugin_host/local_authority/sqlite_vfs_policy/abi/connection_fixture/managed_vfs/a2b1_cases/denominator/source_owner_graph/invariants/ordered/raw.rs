use std::collections::BTreeSet;

use super::super::super::model::{
    Boundary, EdgeKind, Epoch, NodeRole, Reachability, SourceEdge, SourceEffect, SourceNode,
    SourceNodeId, SourceOwnerId, ACQUIRE_OPS, LOCK_OPS, MAP_OPS,
};
use super::require_edge_shape;

pub(super) fn validate(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    validate_operation_scoped_raw_nodes(nodes)?;
    validate_map_fallback_nodes(nodes)?;
    validate_map_fallback_incident_edge_sets(edges)?;
    validate_abandon_owner_order(edges)?;
    validate_fallback_projections(edges)?;
    validate_busy_projection(edges)
}

fn validate_operation_scoped_raw_nodes(nodes: &[SourceNode]) -> Result<(), &'static str> {
    for (id, symbol, ops, boundary) in [
        (
            SourceNodeId::AbiMapRawGate,
            "unsafe fn with_installed_state",
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::AbiLockRawGate,
            "unsafe fn with_installed_state",
            LOCK_OPS,
            Boundary::PendingExpansion,
        ),
        (
            SourceNodeId::AbiMapRawStateAbandon,
            "unsafe fn abandon_installed_state",
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::AbiLockRawStateAbandon,
            "unsafe fn abandon_installed_state",
            LOCK_OPS,
            Boundary::PendingExpansion,
        ),
    ] {
        let Some(node) = nodes.iter().find(|node| node.id == id) else {
            return Err("operation-scoped raw-state graph node is missing");
        };
        if node.owner != SourceOwnerId::AbiRawState
            || node.symbol != symbol
            || node.role != NodeRole::RawStateGate
            || node.ops != ops
            || node.epoch != Epoch::AbiInput
            || node.boundary != boundary
            || node.state_witness.is_some()
        {
            return Err("operation-scoped raw-state graph node changed its reviewed shape");
        }
    }
    Ok(())
}

fn validate_map_fallback_nodes(nodes: &[SourceNode]) -> Result<(), &'static str> {
    for (id, owner, symbol) in [
        (
            SourceNodeId::AbiMapFallbackProjection,
            SourceOwnerId::AbiFileState,
            "Ok(Err(_)) | Err(_) =>",
        ),
        (
            SourceNodeId::AbiMapUnavailableCode,
            SourceOwnerId::AbiResultCodes,
            "SHM_MAP_UNAVAILABLE",
        ),
    ] {
        let Some(node) = nodes.iter().find(|node| node.id == id) else {
            return Err("Map raw fallback projection node is missing");
        };
        if node.owner != owner
            || node.symbol != symbol
            || node.role != NodeRole::AbiProjection
            || node.ops != MAP_OPS
            || node.epoch != Epoch::AbiInput
            || node.boundary != Boundary::Expanded
            || node.state_witness.is_some()
        {
            return Err("Map raw fallback projection node changed its reviewed shape");
        }
    }
    Ok(())
}

fn validate_map_fallback_incident_edge_sets(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (node, expected_ids) in [
        (
            SourceNodeId::AbiMapFallbackProjection,
            &["map.raw-abandon.fallback", "map.fallback.unavailable-code"][..],
        ),
        (
            SourceNodeId::AbiMapUnavailableCode,
            &["map.fallback.unavailable-code"][..],
        ),
    ] {
        let actual = edges
            .iter()
            .filter(|edge| edge.from == node || edge.to == node)
            .map(|edge| edge.id)
            .collect::<BTreeSet<_>>();
        let expected = expected_ids.iter().copied().collect::<BTreeSet<_>>();
        if actual != expected || actual.len() != expected_ids.len() {
            return Err("Map raw fallback projection incident edge set changed without review");
        }
    }
    Ok(())
}

fn validate_abandon_owner_order(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, kind, ops, reachability, effect) in [
        (
            "map.raw-run.gate",
            SourceNodeId::AbiRawRun,
            SourceNodeId::AbiMapRawGate,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Required,
            SourceEffect::None,
        ),
        (
            "lock.raw-run.gate",
            SourceNodeId::AbiRawRun,
            SourceNodeId::AbiLockRawGate,
            EdgeKind::Call,
            LOCK_OPS,
            Reachability::Required,
            SourceEffect::None,
        ),
        (
            "map.raw.failure-abandon",
            SourceNodeId::AbiMapRawGate,
            SourceNodeId::AbiRawAbandon,
            EdgeKind::Abandon,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "lock.raw.failure-abandon",
            SourceNodeId::AbiLockRawGate,
            SourceNodeId::AbiRawAbandon,
            EdgeKind::Abandon,
            LOCK_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.raw-abandon.raw-state",
            SourceNodeId::AbiRawAbandon,
            SourceNodeId::AbiMapRawStateAbandon,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Required,
            SourceEffect::None,
        ),
        (
            "lock.raw-abandon.raw-state",
            SourceNodeId::AbiRawAbandon,
            SourceNodeId::AbiLockRawStateAbandon,
            EdgeKind::Call,
            LOCK_OPS,
            Reachability::Required,
            SourceEffect::None,
        ),
        (
            "map.raw-state-abandon.pinned-drop",
            SourceNodeId::AbiMapRawStateAbandon,
            SourceNodeId::RegistryPinnedDrop,
            EdgeKind::UnwindRetention,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::RetainCustody,
        ),
        (
            "lock.raw-state-abandon.pinned-drop",
            SourceNodeId::AbiLockRawStateAbandon,
            SourceNodeId::RegistryPinnedDrop,
            EdgeKind::UnwindRetention,
            LOCK_OPS,
            Reachability::Conditional,
            SourceEffect::RetainCustody,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            kind,
            ops,
            Epoch::AbiInput,
            reachability,
            effect,
        )?;
    }
    Ok(())
}

fn validate_fallback_projections(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, ops, reachability) in [
        (
            "map.raw-abandon.fallback",
            SourceNodeId::AbiMapRawStateAbandon,
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
            SourceNodeId::AbiLockRawStateAbandon,
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
