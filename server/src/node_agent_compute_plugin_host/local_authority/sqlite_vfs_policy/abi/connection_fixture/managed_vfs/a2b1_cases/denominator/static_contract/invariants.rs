mod expected;
mod traversal;

#[cfg(test)]
mod negative;

use super::{
    model::{
        ContractGraph, CustodyState, DecisionStage, NodeKind, RootOperation, TerminalDisposition,
    },
    source::{self, ProductionOwner, SourceWitness},
};
use expected::validate_node_payload;
use std::collections::{BTreeMap, BTreeSet};
use traversal::{collect_final_leaves, validate_node_degrees, visit_acyclic};
const MAP_LEGACY_WIDTH: usize = 18;
const LOCK_LEGACY_WIDTH: usize = 10;
#[derive(Debug)]
pub(super) struct ValidatedGraph {
    pub(super) included: BTreeSet<String>,
    pub(super) excluded: BTreeSet<String>,
}
pub(super) fn unfinished_graph(name: &'static str) -> ContractGraph {
    let (root_operation, legacy_non_denominator_width) = match name {
        "Map" => (RootOperation::Map, MAP_LEGACY_WIDTH),
        "Lock" => (RootOperation::Lock, LOCK_LEGACY_WIDTH),
        _ => (RootOperation::Map, 0),
    };
    ContractGraph {
        name,
        root_operation,
        root: format!("{name}.unfinished"),
        nodes: Vec::new(),
        edges: Vec::new(),
        source_leaf_universe: BTreeSet::new(),
        declared_denominator: 0,
        legacy_non_denominator_width,
    }
}
pub(super) fn validate_graph(graph: &ContractGraph) -> Result<usize, String> {
    Ok(validate_graph_core(graph)?.included.len())
}

pub(super) fn validate_cross_contract(
    map: &ContractGraph,
    lock: &ContractGraph,
) -> Result<(), String> {
    if map.root_operation != RootOperation::Map || lock.root_operation != RootOperation::Lock {
        return Err("cross-contract validation requires Map followed by Lock".to_owned());
    }
    if map.root == lock.root {
        return Err(format!("Map and Lock share root id {:?}", map.root));
    }
    if !map
        .source_leaf_universe
        .is_disjoint(&lock.source_leaf_universe)
    {
        return Err("Map and Lock source leaf universes overlap".to_owned());
    }
    for root in [&map.root, &lock.root] {
        if map.source_leaf_universe.contains(root) || lock.source_leaf_universe.contains(root) {
            return Err(format!("contract root is also a source leaf: {root:?}"));
        }
    }
    Ok(())
}
fn validate_graph_core(graph: &ContractGraph) -> Result<ValidatedGraph, String> {
    validate_header(graph)?;
    let mut node_indices = BTreeMap::new();
    let mut validated_witnesses = BTreeSet::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.id.trim().is_empty() {
            return Err(format!("{} contains an empty node id", graph.name));
        }
        if node_indices.insert(node.id.clone(), index).is_some() {
            return Err(format!("{} repeats node id {:?}", graph.name, node.id));
        }
        let witness = node
            .witness
            .ok_or_else(|| format!("{} node {:?} has no production anchor", graph.name, node.id))?;
        if validated_witnesses.insert(witness) {
            source::validate_witness(witness)
                .map_err(|error| format!("{} node {:?}: {error}", graph.name, node.id))?;
        }
        validate_node_payload(graph, node)?;
    }
    validate_raw_defensive_inventory(graph, &node_indices)?;
    let root = node_indices
        .get(&graph.root)
        .copied()
        .ok_or_else(|| format!("{} root {:?} is not a node", graph.name, graph.root))?;
    let mut outgoing = vec![Vec::new(); graph.nodes.len()];
    let mut indegree = vec![0_usize; graph.nodes.len()];
    let mut edge_keys = BTreeSet::new();
    for edge in &graph.edges {
        if edge.from.trim().is_empty()
            || edge.to.trim().is_empty()
            || edge.decision.branch.trim().is_empty()
        {
            return Err(format!("{} contains an empty edge identity", graph.name));
        }
        let from = node_indices
            .get(&edge.from)
            .copied()
            .ok_or_else(|| format!("{} edge starts at unknown node {:?}", graph.name, edge.from))?;
        let to = node_indices
            .get(&edge.to)
            .copied()
            .ok_or_else(|| format!("{} edge ends at unknown node {:?}", graph.name, edge.to))?;
        if !edge_keys.insert((from, to, edge.decision.clone())) {
            return Err(format!(
                "{} repeats edge {:?} -> {:?} / {:?}",
                graph.name, edge.from, edge.to, edge.decision
            ));
        }
        outgoing[from].push((to, edge.decision.clone()));
        indegree[to] += 1;
    }
    let roots = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<Vec<_>>();
    if roots.len() != 1 || roots[0] != root {
        let ids = roots
            .into_iter()
            .map(|index| graph.nodes[index].id.as_str())
            .collect::<Vec<_>>();
        return Err(format!(
            "{} must have exactly its declared root; found {ids:?}",
            graph.name
        ));
    }
    validate_node_degrees(graph, &outgoing)?;
    let mut colors = vec![0_u8; graph.nodes.len()];
    visit_acyclic(graph, &outgoing, root, &mut colors)?;
    let unreachable = colors
        .iter()
        .enumerate()
        .filter_map(|(index, color)| (*color != 2).then_some(graph.nodes[index].id.as_str()))
        .collect::<Vec<_>>();
    if !unreachable.is_empty() {
        return Err(format!(
            "{} contains unreachable nodes: {unreachable:?}",
            graph.name
        ));
    }
    let mut validated = ValidatedGraph {
        included: BTreeSet::new(),
        excluded: BTreeSet::new(),
    };
    collect_final_leaves(graph, &outgoing, root, &mut validated)?;
    if !validated.included.is_disjoint(&validated.excluded) {
        return Err(format!(
            "{} marks a source leaf both included and excluded",
            graph.name
        ));
    }
    let actual_universe = validated
        .included
        .union(&validated.excluded)
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_universe != graph.source_leaf_universe {
        let missing = graph
            .source_leaf_universe
            .difference(&actual_universe)
            .collect::<Vec<_>>();
        let undeclared = actual_universe
            .difference(&graph.source_leaf_universe)
            .collect::<Vec<_>>();
        return Err(format!(
            "{} source leaf partition mismatch; missing={missing:?}, undeclared={undeclared:?}",
            graph.name
        ));
    }
    if validated.included.is_empty() {
        return Err(format!("{} has no included terminal root path", graph.name));
    }
    if validated.included.len() != graph.declared_denominator {
        return Err(format!(
            "{} declared denominator {} but DFS found {} terminal root paths",
            graph.name,
            graph.declared_denominator,
            validated.included.len()
        ));
    }
    Ok(validated)
}

fn validate_raw_defensive_inventory(
    graph: &ContractGraph,
    node_indices: &BTreeMap<String, usize>,
) -> Result<(), String> {
    let prefix = match graph.root_operation {
        RootOperation::Map => "map.raw",
        RootOperation::Lock => "lock.raw",
    };
    for suffix in [
        "type-mismatch.payload-present.fallback",
        "type-mismatch.payload-present.abandon-without-unwind",
        "type-mismatch.payload-present.abandon-installed-state",
        "type-mismatch.payload-present.raw-slots-cleared",
        "type-mismatch.payload-present.envelope-drop",
        "type-mismatch.payload-present.payload-drop-dispatch",
        "type-mismatch.payload-present.typed-payload-drop",
        "type-mismatch.payload-present.drop-outcome",
        "expected-type.payload-missing.fallback",
        "expected-type.payload-missing.abandon-without-unwind",
        "expected-type.payload-missing.abandon-installed-state",
        "expected-type.payload-missing.raw-slots-cleared",
        "expected-type.payload-missing.envelope-drop",
        "handle-bound-file.producer",
        "handle-bound-file.domain",
        "handle-bound-file.file-mut",
        "handle-bound-file-missing.adapter-projection",
        "handle-bound-file-missing.run-code-return",
        "handle-bound-file-present",
    ] {
        let id = format!("{prefix}.{suffix}");
        if !node_indices.contains_key(&id) {
            return Err(format!("{} raw defensive chain lacks {id}", graph.name));
        }
    }
    for (from, to, stage, branch) in [
        (
            "handle-bound-file.domain",
            "handle-bound-file.file-mut",
            DecisionStage::Adapter,
            "callback_invokes_file_mut",
        ),
        (
            "handle-bound-file.file-mut",
            "handle-bound-file-missing.adapter-projection",
            DecisionStage::Adapter,
            "file_missing_returns_err",
        ),
        (
            "handle-bound-file-missing.adapter-projection",
            "handle-bound-file-missing.run-code-return",
            DecisionStage::AbiProjection,
            "adapter_error_projected_to_fallback_code",
        ),
        (
            "handle-bound-file-missing.run-code-return",
            "terminal.handle-bound-file-missing",
            DecisionStage::AbiProjection,
            "run_code_returns_fallback_code_without_abandon",
        ),
        (
            "handle-bound-file.file-mut",
            "handle-bound-file-present",
            DecisionStage::Adapter,
            "handle_bound_file_present",
        ),
    ] {
        let from = format!("{prefix}.{from}");
        let to = format!("{prefix}.{to}");
        if !graph.edges.iter().any(|edge| {
            edge.from == from
                && edge.to == to
                && edge.decision.stage == stage
                && edge.decision.branch == branch
        }) {
            return Err(format!(
                "{} raw defensive ordered edge drift: {from} -> {to} / {branch}",
                graph.name
            ));
        }
    }
    let (io_symbol, error_projection, present_call) = match graph.root_operation {
        RootOperation::Map => (
            "unsafe extern \"C\" fn map",
            "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
            "|state| match state.shm_map(region, region_size, extend)",
        ),
        RootOperation::Lock => (
            "unsafe extern \"C\" fn lock",
            "Err(()) => result_codes::SHM_LOCK_UNAVAILABLE",
            "|state| match state.shm_lock(offset, count, action)",
        ),
    };
    for (suffix, expected) in [
        (
            "handle-bound-file.file-mut",
            SourceWitness {
                owner: ProductionOwner::AbiFileState,
                symbol: "fn file_mut",
                needle: "self.file.as_deref_mut().ok_or(())",
                occurrence: 1,
            },
        ),
        (
            "handle-bound-file-missing.adapter-projection",
            SourceWitness {
                owner: ProductionOwner::AbiIoShm,
                symbol: io_symbol,
                needle: error_projection,
                occurrence: 1,
            },
        ),
        (
            "handle-bound-file-missing.run-code-return",
            SourceWitness {
                owner: ProductionOwner::AbiFileState,
                symbol: "unsafe fn run_code",
                needle: "Ok(Ok(code)) => code",
                occurrence: 1,
            },
        ),
        (
            "handle-bound-file-present",
            SourceWitness {
                owner: ProductionOwner::AbiIoShm,
                symbol: io_symbol,
                needle: present_call,
                occurrence: 1,
            },
        ),
    ] {
        let id = format!("{prefix}.{suffix}");
        let actual = node_indices
            .get(&id)
            .and_then(|index| graph.nodes.get(*index))
            .and_then(|node| node.witness);
        if actual != Some(expected) {
            return Err(format!("{} raw defensive witness drift: {id}", graph.name));
        }
    }
    for (suffix, disposition, payload, file) in [
        (
            "terminal.type-mismatch.payload-missing.drop-completed",
            TerminalDisposition::Abandoned,
            CustodyState::Cleared,
            CustodyState::NotReached,
        ),
        (
            "terminal.type-mismatch.payload-present.drop-completed",
            TerminalDisposition::Abandoned,
            CustodyState::Released,
            CustodyState::NotReached,
        ),
        (
            "terminal.type-mismatch.payload-present.drop-unwind-caught",
            TerminalDisposition::Quarantined,
            CustodyState::Quarantined,
            CustodyState::NotReached,
        ),
        (
            "terminal.expected-type.payload-missing.drop-completed",
            TerminalDisposition::Abandoned,
            CustodyState::Cleared,
            CustodyState::NotReached,
        ),
        (
            "terminal.handle-bound-file-missing",
            TerminalDisposition::Returned,
            CustodyState::Retained,
            CustodyState::Cleared,
        ),
    ] {
        let id = format!("{prefix}.{suffix}");
        let node = node_indices
            .get(&id)
            .and_then(|index| graph.nodes.get(*index))
            .ok_or_else(|| format!("{} raw defensive inventory lacks {id}", graph.name))?;
        let NodeKind::Terminal { expected, .. } = &node.kind else {
            return Err(format!(
                "{} raw defensive leaf is not terminal: {id}",
                graph.name
            ));
        };
        if (
            expected.disposition,
            expected.raw_slots,
            expected.payload,
            expected.file,
        ) != (disposition, CustodyState::Cleared, payload, file)
            && suffix != "terminal.handle-bound-file-missing"
        {
            return Err(format!("{} raw defensive Expected drift: {id}", graph.name));
        }
        if suffix == "terminal.handle-bound-file-missing"
            && (
                expected.disposition,
                expected.raw_slots,
                expected.payload,
                expected.file,
            ) != (disposition, CustodyState::Unchanged, payload, file)
        {
            return Err(format!("{} raw defensive Expected drift: {id}", graph.name));
        }
    }
    Ok(())
}
fn validate_header(graph: &ContractGraph) -> Result<(), String> {
    let expected = match graph.root_operation {
        RootOperation::Map => ("Map", MAP_LEGACY_WIDTH),
        RootOperation::Lock => ("Lock", LOCK_LEGACY_WIDTH),
    };
    if (graph.name, graph.legacy_non_denominator_width) != expected {
        return Err(format!(
            "invalid contract identity/legacy width: {:?}/{:?}",
            graph.name, graph.root_operation
        ));
    }
    if graph.root.trim().is_empty()
        || graph.nodes.is_empty()
        || graph.edges.is_empty()
        || graph.source_leaf_universe.is_empty()
    {
        return Err(format!("{} contract graph is not materialized", graph.name));
    }
    if graph
        .source_leaf_universe
        .iter()
        .any(|leaf| leaf.trim().is_empty())
    {
        return Err(format!(
            "{} source leaf universe contains an empty id",
            graph.name
        ));
    }
    if graph.source_leaf_universe.contains(&graph.root) {
        return Err(format!("{} root id is also a source leaf", graph.name));
    }
    Ok(())
}
#[cfg(test)]
pub(super) fn validate_standard_negative_clones(graph: &ContractGraph) -> Result<(), String> {
    negative::validate_standard_negative_clones(graph)
}
