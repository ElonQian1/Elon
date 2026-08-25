use super::super::super::model::{
    Boundary, EdgeKind, Epoch, NodeRole, PathOp, Reachability, SourceEdge, SourceEffect,
    SourceNode, SourceNodeId, StateWitness,
};

pub(super) fn validate(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    validate_cleanup_set(edges)?;
    validate_unlock_does_not_initialize(edges)?;
    validate_cold_witness(nodes, edges)?;
    validate_callback_contexts(edges)?;
    validate_projections_and_boundaries(nodes, edges)
}

fn validate_cleanup_set(edges: &[SourceEdge]) -> Result<(), &'static str> {
    let cleanup_rewrite_ids = [
        "init.close.rewrite",
        "init.dms.close-rewrite",
        "init.dms.unlock-rewrite",
        "map.cleanup.retain",
        "map.native-cleanup.inline-retain",
    ];
    if edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::CleanupRewrite)
        .any(|edge| !cleanup_rewrite_ids.contains(&edge.id))
        || edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::CleanupRewrite)
            .count()
            != cleanup_rewrite_ids.len()
    {
        return Err("cleanup rewrite edge set changed without structural review");
    }
    Ok(())
}

fn validate_unlock_does_not_initialize(edges: &[SourceEdge]) -> Result<(), &'static str> {
    if edges.iter().any(|edge| {
        edge.ops
            .iter()
            .any(|op| matches!(op, PathOp::UnlockShared | PathOp::UnlockExclusive))
            && matches!(
                edge.to,
                SourceNodeId::ManagedEnsureNode
                    | SourceNodeId::ManagedOpenNode
                    | SourceNodeId::ManagedOpenShm
                    | SourceNodeId::ManagedOpenExact
                    | SourceNodeId::ManagedConsumeOpenFailure
                    | SourceNodeId::ManagedRetainFailureHandleCustody
                    | SourceNodeId::ManagedOpenCleanup
                    | SourceNodeId::ManagedPinnedClose
                    | SourceNodeId::ManagedDmsInitialization
                    | SourceNodeId::WindowsByteLock
            )
    }) {
        return Err("unlock source path incorrectly enters node initialization");
    }
    Ok(())
}

fn validate_cold_witness(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    let cold_witness_edge_ids = [
        "map.coordinator.pre-ensure-return.cold-witness",
        "map.promotion.cold-witness",
        "lock.cold.prior-map",
    ];
    let cold_witness_edges = edges
        .iter()
        .filter(|edge| {
            edge.from == SourceNodeId::WalMainColdNodeWitness
                || edge.to == SourceNodeId::WalMainColdNodeWitness
        })
        .collect::<Vec<_>>();
    if cold_witness_edges.len() != cold_witness_edge_ids.len()
        || cold_witness_edges
            .iter()
            .any(|edge| !cold_witness_edge_ids.contains(&edge.id))
    {
        return Err("cold Lock state witness does not have exactly two inputs and one consumer");
    }
    if nodes.iter().any(|node| {
        let expected = (node.id == SourceNodeId::WalMainColdNodeWitness)
            .then_some(StateWitness::WalMainPromotedNodeAbsentAfterEarlyMapReturn);
        node.state_witness != expected
    }) {
        return Err(
            "cold Lock prerequisite is not isolated as the typed WalMain/node-absent witness",
        );
    }
    Ok(())
}

fn validate_callback_contexts(edges: &[SourceEdge]) -> Result<(), &'static str> {
    let promotion_nodes = [
        SourceNodeId::PromotionCallbackBegin,
        SourceNodeId::PromotionProcessBegin,
        SourceNodeId::PromotionOwnerBegin,
        SourceNodeId::PromotionStateBegin,
        SourceNodeId::PromotionCallbackComplete,
        SourceNodeId::PromotionProcessComplete,
        SourceNodeId::PromotionOwnerFinish,
        SourceNodeId::PromotionStateFinish,
    ];
    let operation_callback_nodes = [
        SourceNodeId::RegistryCallbackBegin,
        SourceNodeId::RegistryProcessBegin,
        SourceNodeId::RegistryOwnerBegin,
        SourceNodeId::RegistryStateBegin,
        SourceNodeId::RegistryCallbackComplete,
        SourceNodeId::RegistryProcessComplete,
        SourceNodeId::RegistryOwnerFinish,
        SourceNodeId::RegistryStateFinish,
    ];
    if edges.iter().any(|edge| {
        (promotion_nodes.contains(&edge.from) && operation_callback_nodes.contains(&edge.to))
            || (operation_callback_nodes.contains(&edge.from) && promotion_nodes.contains(&edge.to))
    }) {
        return Err("promotion and with_shm callback leases share a source edge");
    }
    Ok(())
}

fn validate_projections_and_boundaries(
    nodes: &[SourceNode],
    edges: &[SourceEdge],
) -> Result<(), &'static str> {
    let pointer_edges = edges
        .iter()
        .filter(|edge| edge.effect == SourceEffect::OutputPointer)
        .collect::<Vec<_>>();
    if pointer_edges.len() != 1
        || pointer_edges[0].from != SourceNodeId::RegistryMapProjection
        || pointer_edges[0].to != SourceNodeId::AbiMapProjection
    {
        return Err("map pointer output is not limited to the audited Mapped projection");
    }
    if !nodes
        .iter()
        .filter(|node| node.role == NodeRole::Entry)
        .all(|node| {
            matches!(
                node.id,
                SourceNodeId::AbiMapSlot
                    | SourceNodeId::AbiMapEntry
                    | SourceNodeId::AbiLockSlot
                    | SourceNodeId::AbiLockEntry
            )
        })
    {
        return Err("non-ABI source node was marked as an entry");
    }
    for node in nodes
        .iter()
        .filter(|node| node.role == NodeRole::PlatformSeam)
    {
        if node.boundary != Boundary::TypedOutcomeSeam {
            return Err("platform source node expanded unreviewed OS outcome branches");
        }
    }
    if edges.iter().any(|edge| {
        edge.reachability == Reachability::Required
            && edge.kind == EdgeKind::TerminalReturn
            && edge.epoch == Epoch::ColdNodeAcquirePrefix
    }) {
        return Err("conditional cold-prefix terminal was marked universally required");
    }
    Ok(())
}
