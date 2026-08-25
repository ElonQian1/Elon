use std::collections::BTreeSet;

use super::super::super::{
    map as owner_map,
    model::{
        Boundary, NodeRole, PathOp, SourceNode, SourceNodeId, SourceOwnerId, MAP_EXTEND_OPS,
        MAP_OPS,
    },
    shared,
};
use super::super::{
    model::{MapBoundaryReviewStatus, MapSiteId, MapSourceStep, MapSourceStepId},
    scope::{PENDING_BOUNDARIES, RESOLVED_GRAPH_BOUNDARIES},
};

#[derive(Clone, Copy)]
enum LedgerSelector {
    OwnerSymbol(SourceOwnerId, &'static str),
    Site(MapSiteId),
}

struct ResolvedExpectation {
    node: SourceNodeId,
    owner: SourceOwnerId,
    symbol: &'static str,
    role: NodeRole,
    ops: &'static [PathOp],
    boundary: Boundary,
    selector: LedgerSelector,
    witnesses: &'static [MapSourceStepId],
}

struct PendingExpectation {
    node: SourceNodeId,
    status: MapBoundaryReviewStatus,
    witnesses: &'static [MapSourceStepId],
}

const PENDING_EXPECTATIONS: &[PendingExpectation] = &[
    PendingExpectation {
        node: SourceNodeId::AbiRawGate,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawStateRejectedOrPanicked,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::AbiRawStateAbandon,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RawAbandonEmpty,
            MapSourceStepId::RawAbandonInstalled,
            MapSourceStepId::RawAbandonRejected,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::AbiMapValidation,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::AbiInputRejected,
            MapSourceStepId::AbiNullOutputRejected,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::ManagedDmsInitialization,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::ExactOpenFaultBefore,
            MapSourceStepId::DmsExclusiveContended,
            MapSourceStepId::SharedDmsInitialized,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::ManagedMapCoordinator,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RegionSizeChanged,
            MapSourceStepId::FileSizeNativeFailure,
            MapSourceStepId::ManagedMapped,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::ManagedRegionLoop,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RegionLoopContinues,
            MapSourceStepId::RegionReuseCandidate,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::ManagedInlineRegionCustody,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::MappingCreateFaultAfterKnown,
            MapSourceStepId::ViewMapNativeCleanupFailed,
            MapSourceStepId::ViewMapFaultAfterKnown,
        ],
    },
    PendingExpectation {
        node: SourceNodeId::WalMainColdNodeWitness,
        status: MapBoundaryReviewStatus::CrossLedgerStateWitnessPending,
        witnesses: &[
            MapSourceStepId::RegionSizeBudgetRejected,
            MapSourceStepId::RegionCountBudgetRejected,
            MapSourceStepId::LogicalEndBudgetRejected,
            MapSourceStepId::AllocationGranularityFailure,
        ],
    },
];

const RESOLVED_EXPECTATIONS: &[ResolvedExpectation] = &[
    ResolvedExpectation {
        node: SourceNodeId::ManagedRegionSizeValidation,
        owner: SourceOwnerId::ManagedTypes,
        symbol: "fn validate_region_size",
        role: NodeRole::ManagedValidation,
        ops: MAP_OPS,
        boundary: Boundary::Expanded,
        selector: LedgerSelector::OwnerSymbol(
            SourceOwnerId::ManagedTypes,
            "fn validate_region_size",
        ),
        witnesses: &[MapSourceStepId::RegionSizeBudgetRejected],
    },
    ResolvedExpectation {
        node: SourceNodeId::ManagedLogicalEndValidation,
        owner: SourceOwnerId::ManagedTypes,
        symbol: "fn validate_logical_end",
        role: NodeRole::ManagedValidation,
        ops: MAP_OPS,
        boundary: Boundary::Expanded,
        selector: LedgerSelector::OwnerSymbol(
            SourceOwnerId::ManagedTypes,
            "fn validate_logical_end",
        ),
        witnesses: &[
            MapSourceStepId::RegionCountBudgetRejected,
            MapSourceStepId::LogicalEndOverflowRejected,
            MapSourceStepId::LogicalEndBudgetRejected,
        ],
    },
    ResolvedExpectation {
        node: SourceNodeId::ManagedExistingSizeValidation,
        owner: SourceOwnerId::ManagedTypes,
        symbol: "fn validate_existing_size",
        role: NodeRole::ManagedValidation,
        ops: MAP_OPS,
        boundary: Boundary::Expanded,
        selector: LedgerSelector::OwnerSymbol(
            SourceOwnerId::ManagedTypes,
            "fn validate_existing_size",
        ),
        witnesses: &[MapSourceStepId::ExistingSizeBudgetRejected],
    },
    ResolvedExpectation {
        node: SourceNodeId::ManagedMappedTotalValidation,
        owner: SourceOwnerId::ManagedTypes,
        symbol: "fn validate_mapped_total",
        role: NodeRole::ManagedValidation,
        ops: MAP_OPS,
        boundary: Boundary::Expanded,
        selector: LedgerSelector::OwnerSymbol(
            SourceOwnerId::ManagedTypes,
            "fn validate_mapped_total",
        ),
        witnesses: &[MapSourceStepId::MappingBudgetRejected],
    },
    ResolvedExpectation {
        node: SourceNodeId::ManagedFileSize,
        owner: SourceOwnerId::ManagedNamespaceIo,
        symbol: "fn size",
        role: NodeRole::ManagedOperation,
        ops: MAP_OPS,
        boundary: Boundary::TypedOutcomeSeam,
        selector: LedgerSelector::Site(MapSiteId::FileSize),
        witnesses: &[
            MapSourceStepId::FileSizeFaultBefore,
            MapSourceStepId::FileSizeAfterSelectorRejected,
            MapSourceStepId::FileSizeNativeFailure,
            MapSourceStepId::ObserveNotPresent,
        ],
    },
    ResolvedExpectation {
        node: SourceNodeId::ManagedFileGrow,
        owner: SourceOwnerId::ManagedNamespaceIo,
        symbol: "fn truncate",
        role: NodeRole::ManagedOperation,
        ops: MAP_EXTEND_OPS,
        boundary: Boundary::TypedOutcomeSeam,
        selector: LedgerSelector::Site(MapSiteId::FileGrow),
        witnesses: &[
            MapSourceStepId::FileGrowFaultBefore,
            MapSourceStepId::FileGrowNativeFailure,
            MapSourceStepId::FileGrowFaultAfterKnown,
            MapSourceStepId::FileGrowFaultAfterUncertain,
        ],
    },
];

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_pending_boundaries(steps)?;
    validate_resolved_boundaries(steps)
}

fn graph_nodes() -> impl Iterator<Item = &'static SourceNode> {
    shared::NODES.iter().chain(owner_map::NODES)
}

fn graph_node(id: SourceNodeId) -> Option<&'static SourceNode> {
    graph_nodes().find(|node| node.id == id)
}

fn validate_pending_boundaries(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let graph_pending = graph_nodes()
        .filter(|node| {
            node.boundary == Boundary::PendingExpansion
                && node.ops.iter().any(|op| MAP_OPS.contains(op))
        })
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let ledger_pending = PENDING_BOUNDARIES
        .iter()
        .map(|record| record.node)
        .collect::<BTreeSet<_>>();
    let expected_pending = PENDING_EXPECTATIONS
        .iter()
        .map(|expected| expected.node)
        .collect::<BTreeSet<_>>();
    if graph_pending != expected_pending
        || ledger_pending != expected_pending
        || ledger_pending.len() != PENDING_BOUNDARIES.len()
        || PENDING_BOUNDARIES.len() != PENDING_EXPECTATIONS.len()
    {
        return Err("Map pending ledger is not the frozen graph PendingExpansion set");
    }

    for expected in PENDING_EXPECTATIONS {
        let Some(record) = PENDING_BOUNDARIES
            .iter()
            .find(|record| record.node == expected.node)
        else {
            return Err("frozen Map pending boundary is missing from the ledger");
        };
        let Some(node) = graph_node(expected.node) else {
            return Err("Map pending ledger references a missing graph node");
        };
        let expected_witnesses = expected.witnesses.iter().copied().collect::<BTreeSet<_>>();
        let recorded_witnesses = record.witnesses.iter().copied().collect::<BTreeSet<_>>();
        if record.status != expected.status
            || expected_witnesses != recorded_witnesses
            || expected_witnesses.len() != expected.witnesses.len()
            || recorded_witnesses.len() != record.witnesses.len()
        {
            return Err("Map pending graph boundary status or witness set is not exact");
        }
        for witness in record.witnesses {
            let Some(step) = steps.iter().find(|step| step.id == *witness) else {
                return Err("Map pending graph boundary has a detached ledger witness");
            };
            if step.ops.iter().any(|op| !node.ops.contains(op)) {
                return Err("Map pending witness operation scope escapes its graph node");
            }
        }
    }
    Ok(())
}

fn validate_resolved_boundaries(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let expected_nodes = RESOLVED_EXPECTATIONS
        .iter()
        .map(|expected| expected.node)
        .collect::<BTreeSet<_>>();
    let resolved_nodes = RESOLVED_GRAPH_BOUNDARIES
        .iter()
        .map(|record| record.node)
        .collect::<BTreeSet<_>>();
    if expected_nodes != resolved_nodes
        || resolved_nodes.len() != RESOLVED_GRAPH_BOUNDARIES.len()
        || RESOLVED_GRAPH_BOUNDARIES.len() != RESOLVED_EXPECTATIONS.len()
    {
        return Err("Map resolved graph-boundary ledger set is not exact");
    }

    for expected in RESOLVED_EXPECTATIONS {
        let Some(record) = RESOLVED_GRAPH_BOUNDARIES
            .iter()
            .find(|record| record.node == expected.node)
        else {
            return Err("resolved Map graph boundary is missing its ledger record");
        };
        let Some(node) = graph_node(expected.node) else {
            return Err("resolved Map ledger references a missing graph node");
        };
        if node.owner != expected.owner
            || node.symbol != expected.symbol
            || node.role != expected.role
            || node.ops != expected.ops
            || node.boundary != expected.boundary
            || node.boundary == Boundary::PendingExpansion
        {
            return Err("resolved Map graph boundary changed its cross-ledger shape");
        }

        let expected_witnesses = expected.witnesses.iter().copied().collect::<BTreeSet<_>>();
        let recorded_witnesses = record.witnesses.iter().copied().collect::<BTreeSet<_>>();
        if expected_witnesses != recorded_witnesses
            || expected_witnesses.len() != expected.witnesses.len()
            || recorded_witnesses.len() != record.witnesses.len()
        {
            return Err("resolved Map graph boundary witness set is not exact");
        }
        let selected_witnesses = steps
            .iter()
            .filter(|step| match expected.selector {
                LedgerSelector::OwnerSymbol(owner, symbol) => {
                    step.anchor.owner == owner && step.anchor.symbol == symbol
                }
                LedgerSelector::Site(site) => step.site == site,
            })
            .map(|step| step.id)
            .collect::<BTreeSet<_>>();
        if selected_witnesses != expected_witnesses {
            return Err("resolved Map graph node is not exact-linked to its terminal ledger steps");
        }
        for witness in record.witnesses {
            let Some(step) = steps.iter().find(|step| step.id == *witness) else {
                return Err("resolved Map graph boundary has a detached ledger witness");
            };
            if step.ops.iter().any(|op| !node.ops.contains(op)) {
                return Err("resolved Map witness operation scope escapes its graph node");
            }
        }
    }
    Ok(())
}
