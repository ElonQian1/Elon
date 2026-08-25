mod callback;
mod fault;
mod lock;
mod map;
mod prefix;
mod raw;
mod structural;

use super::super::model::{
    EdgeKind, Epoch, PathOp, Reachability, SourceEdge, SourceEffect, SourceNode, SourceNodeId,
    ALL_OPS, INITIALIZING_OPS, MAP_OPS,
};

pub(super) fn validate(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    validate_entry_and_outer_wrappers(edges)?;
    callback::validate(edges)?;
    fault::validate(edges)?;
    lock::validate(edges)?;
    map::validate(nodes, edges)?;
    prefix::validate(edges)?;
    validate_region_loop(edges)?;
    validate_failure_cleanup(edges)?;
    raw::validate(nodes, edges)?;
    structural::validate(nodes, edges)
}

fn validate_entry_and_outer_wrappers(edges: &[SourceEdge]) -> Result<(), &'static str> {
    require_edge_shape(
        edges,
        "map.entry.null-output",
        SourceNodeId::AbiMapEntry,
        SourceNodeId::AbiMapNullOutput,
        EdgeKind::Call,
        MAP_OPS,
        Epoch::AbiInput,
        Reachability::Required,
        SourceEffect::OutputNull,
    )?;
    if edges
        .iter()
        .filter(|edge| edge.effect == SourceEffect::OutputNull)
        .count()
        != 1
    {
        return Err("map output-null effect is not isolated to the ABI entry edge");
    }
    for (from, to, kind) in [
        (
            SourceNodeId::FileStateMap,
            SourceNodeId::FixtureMapFault,
            EdgeKind::Call,
        ),
        (
            SourceNodeId::FixtureMapFault,
            SourceNodeId::FixtureFaultController,
            EdgeKind::Call,
        ),
        (
            SourceNodeId::FixtureFaultController,
            SourceNodeId::RouteMapPreparation,
            EdgeKind::ConditionalCall,
        ),
        (
            SourceNodeId::FileStateLock,
            SourceNodeId::FixtureLockFault,
            EdgeKind::Call,
        ),
        (
            SourceNodeId::FixtureLockFault,
            SourceNodeId::FixtureFaultController,
            EdgeKind::Call,
        ),
        (
            SourceNodeId::FixtureFaultController,
            SourceNodeId::RouteLockDelegate,
            EdgeKind::ConditionalCall,
        ),
    ] {
        require_edge(edges, from, to, kind)?;
    }
    Ok(())
}

fn validate_region_loop(edges: &[SourceEdge]) -> Result<(), &'static str> {
    require_edge_shape(
        edges,
        "map.inline-custody.next-region",
        SourceNodeId::ManagedInlineRegionCustody,
        SourceNodeId::ManagedRegionLoop,
        EdgeKind::LoopBack,
        MAP_OPS,
        Epoch::WalMainSteady,
        Reachability::Conditional,
        SourceEffect::None,
    )?;
    require_edge_shape(
        edges,
        "map.loop.select",
        SourceNodeId::ManagedRegionLoop,
        SourceNodeId::ManagedRegionSelect,
        EdgeKind::Continuation,
        MAP_OPS,
        Epoch::WalMainSteady,
        Reachability::Conditional,
        SourceEffect::None,
    )?;
    require_edge_shape(
        edges,
        "map.select.complete",
        SourceNodeId::ManagedRegionSelect,
        SourceNodeId::RegistryCallbackComplete,
        EdgeKind::CallbackCompletion,
        MAP_OPS,
        Epoch::WalMainSteady,
        Reachability::Required,
        SourceEffect::CallbackLease,
    )
}

fn validate_failure_cleanup(edges: &[SourceEdge]) -> Result<(), &'static str> {
    for (id, from, to, kind, reachability, effect) in [
        (
            "init.open.failure-custody",
            SourceNodeId::ManagedOpenExact,
            SourceNodeId::ManagedConsumeOpenFailure,
            EdgeKind::TerminalReturn,
            Reachability::Conditional,
            SourceEffect::RetainCustody,
        ),
        (
            "init.failure.retain-custody",
            SourceNodeId::ManagedConsumeOpenFailure,
            SourceNodeId::ManagedRetainFailureHandleCustody,
            EdgeKind::Call,
            Reachability::Required,
            SourceEffect::RetainCustody,
        ),
        (
            "init.failure-custody.close",
            SourceNodeId::ManagedRetainFailureHandleCustody,
            SourceNodeId::ManagedPinnedClose,
            EdgeKind::ConditionalCall,
            Reachability::Conditional,
            SourceEffect::Cleanup,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            kind,
            INITIALIZING_OPS,
            Epoch::ColdNodeAcquirePrefix,
            reachability,
            effect,
        )?;
    }
    for (id, from, to, kind, effect) in [
        (
            "map.view-before.uncertain-retain",
            SourceNodeId::ManagedFaultBegin,
            SourceNodeId::ManagedMappingCustodyRetain,
            EdgeKind::UnwindRetention,
            SourceEffect::RetainCustody,
        ),
        (
            "map.view-before.cleanup",
            SourceNodeId::ManagedFaultBegin,
            SourceNodeId::ManagedMappingCleanup,
            EdgeKind::ConditionalCall,
            SourceEffect::Cleanup,
        ),
        (
            "map.view.native-cleanup",
            SourceNodeId::WindowsMapView,
            SourceNodeId::ManagedNativeMappingCleanup,
            EdgeKind::ConditionalCall,
            SourceEffect::Cleanup,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            kind,
            MAP_OPS,
            Epoch::WalMainSteady,
            Reachability::Conditional,
            effect,
        )?;
    }
    for (id, from, to, ops, epoch, reachability, effect) in [
        (
            "init.close.rewrite",
            SourceNodeId::ManagedOpenCleanup,
            SourceNodeId::ManagedPinnedClose,
            INITIALIZING_OPS,
            Epoch::ColdNodeAcquirePrefix,
            Reachability::Required,
            SourceEffect::RetainCustody,
        ),
        (
            "init.dms.close-rewrite",
            SourceNodeId::ManagedDmsInitialization,
            SourceNodeId::ManagedOpenCleanup,
            INITIALIZING_OPS,
            Epoch::ColdNodeAcquirePrefix,
            Reachability::Conditional,
            SourceEffect::Cleanup,
        ),
        (
            "init.dms.unlock-rewrite",
            SourceNodeId::WindowsByteUnlock,
            SourceNodeId::ManagedPoison,
            INITIALIZING_OPS,
            Epoch::ColdNodeAcquirePrefix,
            Reachability::Conditional,
            SourceEffect::Poison,
        ),
        (
            "map.cleanup.retain",
            SourceNodeId::ManagedMappingCleanup,
            SourceNodeId::ManagedMappingCustodyRetain,
            MAP_OPS,
            Epoch::WalMainSteady,
            Reachability::Conditional,
            SourceEffect::RetainCustody,
        ),
        (
            "map.native-cleanup.inline-retain",
            SourceNodeId::ManagedNativeMappingCleanup,
            SourceNodeId::ManagedInlineRegionCustody,
            MAP_OPS,
            Epoch::WalMainSteady,
            Reachability::Conditional,
            SourceEffect::RetainCustody,
        ),
    ] {
        require_edge_shape(
            edges,
            id,
            from,
            to,
            EdgeKind::CleanupRewrite,
            ops,
            epoch,
            reachability,
            effect,
        )?;
    }
    Ok(())
}

fn require_edge(
    edges: &[SourceEdge],
    from: SourceNodeId,
    to: SourceNodeId,
    kind: EdgeKind,
) -> Result<(), &'static str> {
    if edges
        .iter()
        .any(|edge| edge.from == from && edge.to == to && edge.kind == kind)
    {
        Ok(())
    } else {
        Err("required ordered source owner edge is missing")
    }
}

#[allow(clippy::too_many_arguments)]
fn require_edge_shape(
    edges: &[SourceEdge],
    id: &str,
    from: SourceNodeId,
    to: SourceNodeId,
    kind: EdgeKind,
    ops: &[PathOp],
    epoch: Epoch,
    reachability: Reachability,
    effect: SourceEffect,
) -> Result<(), &'static str> {
    if edges.iter().any(|edge| {
        edge.id == id
            && edge.from == from
            && edge.to == to
            && edge.kind == kind
            && edge.ops == ops
            && edge.epoch == epoch
            && edge.reachability == reachability
            && edge.effect == effect
    }) {
        Ok(())
    } else {
        Err("required typed source owner edge changed")
    }
}
