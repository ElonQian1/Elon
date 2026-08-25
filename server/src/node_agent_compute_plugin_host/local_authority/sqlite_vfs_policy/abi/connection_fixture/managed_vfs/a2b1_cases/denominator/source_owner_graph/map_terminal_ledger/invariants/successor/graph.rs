use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::super::super::super::model::SourceEffect;
use super::super::super::{
    model::{MapExit, MapSourceStep, MapSourceStepId},
    reviewed_trace::{
        RawAbandonOutcome, RawStateCase, RawStateTraceDisposition, ReviewedFrontierIngress,
        ReviewedOpenFrontier, ReviewedOpenFrontierRecord, ReviewedTerminal, ReviewedTraceCondition,
        ReviewedTraceEndpoint, ReviewedTraceRelation, OPEN_FRONTIERS, OPEN_FRONTIER_RECORDS,
        RAW_ABANDON_OUTCOMES, RAW_STATE_OUTCOMES, SUCCESSOR_EDGES, TERMINALS,
    },
};
use super::shared::{edge, edge_key, require_edge, EdgeKey, INSTALLED_RAW_VALUES};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_endpoints()?;
    validate_edges(steps)
}

fn validate_endpoints() -> Result<(), &'static str> {
    let terminals = TERMINALS.iter().copied().collect::<BTreeSet<_>>();
    let expected_terminals = [
        ReviewedTerminal::AbiUnavailableNull,
        ReviewedTerminal::AbiUnavailableNoSlot,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if terminals != expected_terminals || terminals.len() != TERMINALS.len() {
        return Err("Map ABI reviewed terminal endpoint set is not exact");
    }

    let frontiers = OPEN_FRONTIERS.iter().copied().collect::<BTreeSet<_>>();
    let expected_frontiers = [
        ReviewedOpenFrontier::TypedMapOperation,
        ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if frontiers != expected_frontiers
        || frontiers.len() != OPEN_FRONTIERS.len()
        || OPEN_FRONTIER_RECORDS.len() != OPEN_FRONTIERS.len()
    {
        return Err("Map ABI/raw open-frontier endpoint set is not exact");
    }

    let records = OPEN_FRONTIER_RECORDS
        .iter()
        .map(|record| (record.frontier, *record))
        .collect::<BTreeMap<_, _>>();
    if records.len() != OPEN_FRONTIER_RECORDS.len()
        || records.get(&ReviewedOpenFrontier::TypedMapOperation)
            != Some(&ReviewedOpenFrontierRecord {
                frontier: ReviewedOpenFrontier::TypedMapOperation,
                ingress: ReviewedFrontierIngress::ExpectedTypedState,
                known_exit: None,
                custody_unresolved: true,
                route_projection_unresolved: true,
            })
        || records.get(&ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection)
            != Some(&ReviewedOpenFrontierRecord {
                frontier: ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
                ingress: ReviewedFrontierIngress::PrefixRawRejectionAfterAbandon,
                known_exit: Some(MapExit::AbiUnavailableNull),
                custody_unresolved: true,
                route_projection_unresolved: true,
            })
    {
        return Err("Map ABI/raw open-frontier ingress or unresolved projection is not exact");
    }
    Ok(())
}

fn validate_edges(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let actual = SUCCESSOR_EDGES
        .iter()
        .map(edge_key)
        .collect::<BTreeSet<_>>();
    if actual.len() != SUCCESSOR_EDGES.len() || actual.len() != 31 {
        return Err("Map ABI/raw reviewed successor edge set has a duplicate or wrong cardinality");
    }

    for expected in abi_edges() {
        require_edge(&actual, expected)?;
    }
    require_edge(
        &actual,
        edge(
            ReviewedTraceEndpoint::Step(MapSourceStepId::AbiRawDispatch),
            ReviewedTraceEndpoint::OpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
            ReviewedTraceCondition::RawExpectedTypeEntry,
            ReviewedTraceRelation::OpenFrontier,
            SourceEffect::None,
            Some(INSTALLED_RAW_VALUES),
        ),
    )?;

    for record in RAW_STATE_OUTCOMES {
        match record.trace {
            RawStateTraceDisposition::BeyondOpenFrontier(frontier) => {
                validate_beyond_frontier_record(record.case, record.step, frontier)?;
            }
            RawStateTraceDisposition::PrefixSuccessor(successor) => {
                require_edge(
                    &actual,
                    edge(
                        ReviewedTraceEndpoint::Step(MapSourceStepId::AbiRawDispatch),
                        ReviewedTraceEndpoint::Step(record.step),
                        ReviewedTraceCondition::RawState(record.case),
                        ReviewedTraceRelation::ConditionalBranch,
                        SourceEffect::None,
                        Some(record.slots),
                    ),
                )?;
                require_edge(
                    &actual,
                    edge(
                        ReviewedTraceEndpoint::Step(record.step),
                        successor,
                        ReviewedTraceCondition::RawState(record.case),
                        ReviewedTraceRelation::Abandon,
                        SourceEffect::None,
                        Some(record.slots),
                    ),
                )?;
            }
        }
    }

    for record in RAW_ABANDON_OUTCOMES {
        let relation = match record.outcome {
            RawAbandonOutcome::InstalledDropCompleted
            | RawAbandonOutcome::InstalledDropUnwindCaught => ReviewedTraceRelation::Cleanup,
            _ => ReviewedTraceRelation::ResultProjection,
        };
        require_edge(
            &actual,
            edge(
                ReviewedTraceEndpoint::Step(record.step),
                record.prefix_successor,
                ReviewedTraceCondition::RawAbandon(record.outcome),
                relation,
                record.effect,
                Some(record.slots),
            ),
        )?;
    }
    require_edge(
        &actual,
        edge(
            ReviewedTraceEndpoint::Step(MapSourceStepId::RawFallbackProjection),
            ReviewedTraceEndpoint::OpenFrontier(
                ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
            ),
            ReviewedTraceCondition::Unconditional,
            ReviewedTraceRelation::OpenFrontier,
            SourceEffect::None,
            None,
        ),
    )?;

    validate_materialized_sinks(steps)?;
    validate_acyclic_reachable_graph()
}

fn validate_beyond_frontier_record(
    case: RawStateCase,
    step: MapSourceStepId,
    frontier: ReviewedOpenFrontier,
) -> Result<(), &'static str> {
    let exact_post_frontier_source = matches!(
        (case, step, frontier),
        (
            RawStateCase::AcceptedAfterTypedOperation,
            MapSourceStepId::RawStateAccepted,
            ReviewedOpenFrontier::TypedMapOperation,
        ) | (
            RawStateCase::CaughtUnwindFromTypedOperation,
            MapSourceStepId::RawStateCaughtPanic,
            ReviewedOpenFrontier::TypedMapOperation,
        )
    );
    if !exact_post_frontier_source {
        return Err("Map raw post-operation source is not honestly beyond the typed frontier");
    }
    if SUCCESSOR_EDGES.iter().any(|edge| {
        edge.from == ReviewedTraceEndpoint::Step(step)
            || edge.to == ReviewedTraceEndpoint::Step(step)
    }) {
        return Err("Map raw post-operation marker was inverted into the prefix graph");
    }
    Ok(())
}

fn validate_materialized_sinks(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let step_ids = steps.iter().map(|step| step.id).collect::<BTreeSet<_>>();
    for edge in SUCCESSOR_EDGES {
        for endpoint in [edge.from, edge.to] {
            if let ReviewedTraceEndpoint::Step(id) = endpoint {
                if id == MapSourceStepId::Count || !step_ids.contains(&id) {
                    return Err("Map reviewed successor edge references a detached source step");
                }
            }
        }
        if matches!(
            edge.from,
            ReviewedTraceEndpoint::Terminal(_) | ReviewedTraceEndpoint::OpenFrontier(_)
        ) {
            return Err("Map reviewed terminal or open frontier has an outgoing edge");
        }
    }
    Ok(())
}

fn abi_edges() -> [EdgeKey; 7] {
    use MapSourceStepId::{AbiInputRejected, AbiNullFirst, AbiNullOutputRejected, AbiRawDispatch};
    [
        edge(
            ReviewedTraceEndpoint::Step(AbiNullFirst),
            ReviewedTraceEndpoint::Step(AbiInputRejected),
            ReviewedTraceCondition::AbiInvalidOutputWritable,
            ReviewedTraceRelation::ConditionalBranch,
            SourceEffect::OutputNull,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiNullFirst),
            ReviewedTraceEndpoint::Step(AbiInputRejected),
            ReviewedTraceCondition::AbiInvalidOutputAbsent,
            ReviewedTraceRelation::ConditionalBranch,
            SourceEffect::None,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiNullFirst),
            ReviewedTraceEndpoint::Step(AbiRawDispatch),
            ReviewedTraceCondition::AbiValidOutputWritable,
            ReviewedTraceRelation::Continuation,
            SourceEffect::OutputNull,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiNullFirst),
            ReviewedTraceEndpoint::Step(AbiNullOutputRejected),
            ReviewedTraceCondition::AbiValidOutputAbsent,
            ReviewedTraceRelation::ConditionalBranch,
            SourceEffect::None,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiInputRejected),
            ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNull),
            ReviewedTraceCondition::AbiInvalidOutputWritable,
            ReviewedTraceRelation::ResultProjection,
            SourceEffect::None,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiInputRejected),
            ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
            ReviewedTraceCondition::AbiInvalidOutputAbsent,
            ReviewedTraceRelation::ResultProjection,
            SourceEffect::None,
            None,
        ),
        edge(
            ReviewedTraceEndpoint::Step(AbiNullOutputRejected),
            ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
            ReviewedTraceCondition::AbiValidOutputAbsent,
            ReviewedTraceRelation::ResultProjection,
            SourceEffect::None,
            None,
        ),
    ]
}

fn validate_acyclic_reachable_graph() -> Result<(), &'static str> {
    let mut endpoints = BTreeSet::new();
    let mut incoming = BTreeMap::<ReviewedTraceEndpoint, usize>::new();
    let mut outgoing = BTreeMap::<ReviewedTraceEndpoint, Vec<ReviewedTraceEndpoint>>::new();
    for edge in SUCCESSOR_EDGES {
        endpoints.insert(edge.from);
        endpoints.insert(edge.to);
        incoming.entry(edge.from).or_default();
        *incoming.entry(edge.to).or_default() += 1;
        outgoing.entry(edge.from).or_default().push(edge.to);
    }
    let roots = endpoints
        .iter()
        .copied()
        .filter(|endpoint| incoming.get(endpoint).copied().unwrap_or(0) == 0)
        .collect::<BTreeSet<_>>();
    let root = ReviewedTraceEndpoint::Step(MapSourceStepId::AbiNullFirst);
    if roots != [root].into_iter().collect() {
        return Err("Map ABI/raw reviewed successor graph does not have the exact ABI root");
    }

    let mut queue = VecDeque::from([root]);
    let mut reachable = BTreeSet::new();
    while let Some(endpoint) = queue.pop_front() {
        if !reachable.insert(endpoint) {
            continue;
        }
        if let Some(successors) = outgoing.get(&endpoint) {
            queue.extend(successors.iter().copied());
        }
    }
    if reachable != endpoints {
        return Err("Map ABI/raw reviewed successor graph has an unreachable endpoint");
    }

    let mut indegree = incoming;
    let mut ready = indegree
        .iter()
        .filter_map(|(endpoint, count)| (*count == 0).then_some(*endpoint))
        .collect::<VecDeque<_>>();
    let mut visited = 0usize;
    while let Some(endpoint) = ready.pop_front() {
        visited += 1;
        if let Some(successors) = outgoing.get(&endpoint) {
            for successor in successors {
                let Some(count) = indegree.get_mut(successor) else {
                    return Err("Map ABI/raw reviewed graph lost an endpoint indegree");
                };
                *count -= 1;
                if *count == 0 {
                    ready.push_back(*successor);
                }
            }
        }
    }
    if visited != endpoints.len() {
        return Err("Map ABI/raw reviewed successor graph contains a cycle");
    }
    Ok(())
}
