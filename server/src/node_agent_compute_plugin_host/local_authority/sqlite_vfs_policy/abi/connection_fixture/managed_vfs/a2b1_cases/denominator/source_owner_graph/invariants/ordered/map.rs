use std::collections::BTreeSet;

use super::super::super::model::{
    Boundary, EdgeKind, Epoch, NodeRole, PathOp, Reachability, SourceEdge, SourceEffect,
    SourceNode, SourceNodeId, SourceOwnerId, MAP_EXTEND_OPS, MAP_OPS,
};
use super::require_edge_shape;

pub(super) fn validate(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    validate_nodes(nodes)?;
    validate_edges(edges)?;
    validate_incident_edge_sets(edges)?;
    validate_file_effect_scopes(edges)
}

fn validate_nodes(nodes: &[SourceNode]) -> Result<(), &'static str> {
    for (id, owner, symbol, role, ops, boundary) in [
        (
            SourceNodeId::ManagedRegionSizeValidation,
            SourceOwnerId::ManagedTypes,
            "fn validate_region_size",
            NodeRole::ManagedValidation,
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::ManagedLogicalEndValidation,
            SourceOwnerId::ManagedTypes,
            "fn validate_logical_end",
            NodeRole::ManagedValidation,
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::ManagedExistingSizeValidation,
            SourceOwnerId::ManagedTypes,
            "fn validate_existing_size",
            NodeRole::ManagedValidation,
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::ManagedMappedTotalValidation,
            SourceOwnerId::ManagedTypes,
            "fn validate_mapped_total",
            NodeRole::ManagedValidation,
            MAP_OPS,
            Boundary::Expanded,
        ),
        (
            SourceNodeId::ManagedFileSize,
            SourceOwnerId::ManagedNamespaceIo,
            "fn size",
            NodeRole::ManagedOperation,
            MAP_OPS,
            Boundary::TypedOutcomeSeam,
        ),
        (
            SourceNodeId::ManagedFileGrow,
            SourceOwnerId::ManagedNamespaceIo,
            "fn truncate",
            NodeRole::ManagedOperation,
            MAP_EXTEND_OPS,
            Boundary::TypedOutcomeSeam,
        ),
    ] {
        let Some(node) = nodes.iter().find(|node| node.id == id) else {
            return Err("resolved Map source node is missing");
        };
        if node.owner != owner
            || node.symbol != symbol
            || node.role != role
            || node.ops != ops
            || node.epoch != Epoch::WalMainSteady
            || node.boundary != boundary
            || node.state_witness.is_some()
        {
            return Err("resolved Map source node changed its reviewed shape");
        }
    }
    Ok(())
}

fn validate_edges(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, kind, ops, reachability, effect) in [
        (
            "map.coordinator.region-size",
            SourceNodeId::ManagedMapCoordinator,
            SourceNodeId::ManagedRegionSizeValidation,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Required,
            SourceEffect::None,
        ),
        (
            "map.region-size.logical-end",
            SourceNodeId::ManagedRegionSizeValidation,
            SourceNodeId::ManagedLogicalEndValidation,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.logical-end.granularity",
            SourceNodeId::ManagedLogicalEndValidation,
            SourceNodeId::WindowsAllocationGranularity,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-size-fault.operation",
            SourceNodeId::ManagedFaultBegin,
            SourceNodeId::ManagedFileSize,
            EdgeKind::Continuation,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-size.fault-finish",
            SourceNodeId::ManagedFileSize,
            SourceNodeId::ManagedFaultFinish,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-size.existing-size",
            SourceNodeId::ManagedFileSize,
            SourceNodeId::ManagedExistingSizeValidation,
            EdgeKind::Continuation,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.existing-size.region-loop",
            SourceNodeId::ManagedExistingSizeValidation,
            SourceNodeId::ManagedRegionLoop,
            EdgeKind::Continuation,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.existing-size.file-grow-fault",
            SourceNodeId::ManagedExistingSizeValidation,
            SourceNodeId::ManagedFaultBegin,
            EdgeKind::ConditionalCall,
            MAP_EXTEND_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-grow-fault.operation",
            SourceNodeId::ManagedFaultBegin,
            SourceNodeId::ManagedFileGrow,
            EdgeKind::Continuation,
            MAP_EXTEND_OPS,
            Reachability::Conditional,
            SourceEffect::PlatformMutation,
        ),
        (
            "map.file-grow.fault-finish",
            SourceNodeId::ManagedFileGrow,
            SourceNodeId::ManagedFaultFinish,
            EdgeKind::Call,
            MAP_EXTEND_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-grow.region-loop",
            SourceNodeId::ManagedFileGrow,
            SourceNodeId::ManagedRegionLoop,
            EdgeKind::Continuation,
            MAP_EXTEND_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.file-grow.poison",
            SourceNodeId::ManagedFileGrow,
            SourceNodeId::ManagedPoison,
            EdgeKind::MutationBeforeContinuation,
            MAP_EXTEND_OPS,
            Reachability::Conditional,
            SourceEffect::Poison,
        ),
        (
            "map.loop.mapped-total",
            SourceNodeId::ManagedRegionLoop,
            SourceNodeId::ManagedMappedTotalValidation,
            EdgeKind::Call,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
        (
            "map.mapped-total.mapping-fault",
            SourceNodeId::ManagedMappedTotalValidation,
            SourceNodeId::ManagedFaultBegin,
            EdgeKind::ConditionalCall,
            MAP_OPS,
            Reachability::Conditional,
            SourceEffect::None,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            kind,
            ops,
            Epoch::WalMainSteady,
            reachability,
            effect,
        )?;
    }
    Ok(())
}

fn validate_incident_edge_sets(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (node, expected_ids) in [
        (
            SourceNodeId::ManagedRegionSizeValidation,
            &["map.coordinator.region-size", "map.region-size.logical-end"][..],
        ),
        (
            SourceNodeId::ManagedLogicalEndValidation,
            &["map.region-size.logical-end", "map.logical-end.granularity"][..],
        ),
        (
            SourceNodeId::ManagedExistingSizeValidation,
            &[
                "map.file-size.existing-size",
                "map.existing-size.region-loop",
                "map.existing-size.file-grow-fault",
            ][..],
        ),
        (
            SourceNodeId::ManagedMappedTotalValidation,
            &["map.loop.mapped-total", "map.mapped-total.mapping-fault"][..],
        ),
        (
            SourceNodeId::ManagedFileSize,
            &[
                "map.file-size-fault.operation",
                "map.file-size.fault-finish",
                "map.file-size.existing-size",
            ][..],
        ),
        (
            SourceNodeId::ManagedFileGrow,
            &[
                "map.file-grow-fault.operation",
                "map.file-grow.fault-finish",
                "map.file-grow.region-loop",
                "map.file-grow.poison",
            ][..],
        ),
    ] {
        let actual = edges
            .iter()
            .filter(|edge| edge.from == node || edge.to == node)
            .map(|edge| edge.id)
            .collect::<BTreeSet<_>>();
        let expected = expected_ids.iter().copied().collect::<BTreeSet<_>>();
        if actual != expected || actual.len() != expected_ids.len() {
            return Err("resolved Map node incident edge set changed without review");
        }
    }
    Ok(())
}

fn validate_file_effect_scopes(edges: &[SourceEdge]) -> Result<(), &'static str> {
    if edges.iter().any(|edge| {
        (edge.from == SourceNodeId::ManagedFileGrow || edge.to == SourceNodeId::ManagedFileGrow)
            && edge.ops != MAP_EXTEND_OPS
    }) {
        return Err("FileGrow escaped its Extend-only operation scope");
    }
    if edges.iter().any(|edge| {
        (edge.from == SourceNodeId::ManagedFileSize || edge.to == SourceNodeId::ManagedFileSize)
            && (edge.ops != MAP_OPS || edge.effect == SourceEffect::PlatformMutation)
    }) {
        return Err("read-only FileSize acquired mutation or lost an operation");
    }
    let mutation_ids = edges
        .iter()
        .filter(|edge| {
            (edge.from == SourceNodeId::ManagedFileGrow || edge.to == SourceNodeId::ManagedFileGrow)
                && edge.effect == SourceEffect::PlatformMutation
        })
        .map(|edge| edge.id)
        .collect::<BTreeSet<_>>();
    if mutation_ids != ["map.file-grow-fault.operation"].into_iter().collect() {
        return Err("FileGrow platform mutation is not isolated to the truncate call");
    }
    Ok(())
}
