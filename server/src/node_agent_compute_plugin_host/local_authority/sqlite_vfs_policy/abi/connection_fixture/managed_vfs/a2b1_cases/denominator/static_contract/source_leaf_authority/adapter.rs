//! Iterative `ContractGraph` projection into the neutral source-leaf schema.

mod mapping;

use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::super::model as graph;
use super::super::terminal_descriptor::TypedTerminalDescriptorV1;
use super::{
    accumulator::ManifestAccumulatorV1,
    coverage::validate_graph_ledger_coverage,
    leaf_seal::{leaf_seal_tsv_sha256, FrozenLeafSealVerifierV1, LeafSealV1},
    manifest::validate_derived_manifest_against_frozen,
    model::{
        CaseKeyV1, LeafIdentityV1, LeafOutcomeV1, LeafRecordV1, ManifestContextV1, RootManifestV1,
        RootOperationV1,
    },
    observer::StreamedLeafV1,
    source_scope::validate_source_witness,
};

/// The initial graph identity keeps the exact source-leaf id and uses only a root-level family.
/// A source-led family table may instead be supplied through `stream_graph_manifest_with_identity`.
pub(crate) fn exact_graph_leaf_identity(
    root: RootOperationV1,
    leaf_id: &str,
) -> Result<LeafIdentityV1, String> {
    if leaf_id.is_empty() {
        return Err("graph leaf identity is empty".to_owned());
    }
    Ok(LeafIdentityV1 {
        root,
        leaf_id: leaf_id.to_owned(),
        family_id: format!("{}.static-contract-source-leaf-v1", root.canonical_name()),
        coordinates: Vec::new(),
    })
}

/// Streams every terminal and Excluded root path through the manifest accumulator.
///
/// The callback receives a fixed-size seal after the record has been validated and accumulated.
/// Only the active DFS path is held; completed decision/source paths are dropped immediately.
pub(crate) fn stream_graph_manifest<F>(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    mut emit_seal: F,
) -> Result<RootManifestV1, String>
where
    F: FnMut(&LeafSealV1) -> Result<(), String>,
{
    stream_graph_manifest_with_records(graph, context, |leaf| emit_seal(leaf.seal()))
}

pub(crate) fn stream_graph_manifest_with_identity<I, F>(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    mut identity_for: I,
    mut emit_seal: F,
) -> Result<RootManifestV1, String>
where
    I: FnMut(RootOperationV1, &str) -> Result<LeafIdentityV1, String>,
    F: FnMut(&LeafSealV1) -> Result<(), String>,
{
    stream_graph_manifest_with_identity_and_records(graph, context, identity_for, |leaf| {
        emit_seal(leaf.seal())
    })
}

/// Streams the exact static records together with their co-produced typed terminal descriptors.
///
/// This observer is intentionally layered on the same accumulator as the frozen static manifest.
/// Excluded leaves never carry a descriptor and can therefore never enter the dynamic quotient.
pub(crate) fn stream_graph_manifest_with_records<F>(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    emit_leaf: F,
) -> Result<RootManifestV1, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    stream_graph_manifest_with_identity_and_records(
        graph,
        context,
        exact_graph_leaf_identity,
        emit_leaf,
    )
}

pub(crate) fn stream_graph_manifest_with_identity_and_records<I, F>(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    mut identity_for: I,
    mut emit_leaf: F,
) -> Result<RootManifestV1, String>
where
    I: FnMut(RootOperationV1, &str) -> Result<LeafIdentityV1, String>,
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    let root_operation = mapping::root(graph.root_operation);
    if context.root != root_operation {
        return Err("graph root and manifest context root disagree".to_owned());
    }
    validate_graph_ledger_coverage(graph)?;
    let topology = Topology::new(graph)?;
    let mut accumulator = ManifestAccumulatorV1::new(context)?;
    let mut paths = ActivePaths::default();
    let mut seen_leaf_ids = HashSet::new();
    let mut seen_nodes = BTreeSet::new();
    let mut active = vec![false; graph.nodes.len()];
    let mut validated_witnesses = BTreeSet::new();
    let mut stack = vec![Frame::root(topology.root)];

    while let Some(frame_index) = stack.len().checked_sub(1) {
        if !stack[frame_index].entered {
            let node_index = stack[frame_index].node;
            if active[node_index] {
                return Err(format!(
                    "source-leaf adapter found a cycle at {:?}",
                    graph.nodes[node_index].id
                ));
            }
            active[node_index] = true;
            seen_nodes.insert(node_index);
            let witness = graph.nodes[node_index].witness.ok_or_else(|| {
                format!(
                    "source-leaf adapter found an unanchored node: {:?}",
                    graph.nodes[node_index].id
                )
            })?;
            let witness = mapping::witness(witness);
            if validated_witnesses.insert(witness.clone()) {
                validate_source_witness(&witness)?;
            }
            paths.sources.push(witness);
            stack[frame_index].entered = true;

            match &graph.nodes[node_index].kind {
                graph::NodeKind::Terminal {
                    leaf_id,
                    expected,
                    descriptor,
                } => {
                    topology.require_final(node_index, graph)?;
                    let identity = identity_for(root_operation, leaf_id)?;
                    validate_identity(root_operation, leaf_id, &identity)?;
                    let record = paths.record(
                        identity,
                        LeafOutcomeV1::Terminal(mapping::expected(*expected)),
                    );
                    emit_record(
                        &mut accumulator,
                        &mut emit_leaf,
                        &mut seen_leaf_ids,
                        leaf_id,
                        &record,
                        Some(descriptor),
                    )?;
                    pop_frame(&mut stack, &mut paths, &mut active);
                    continue;
                }
                graph::NodeKind::Excluded { leaf_id, proof } => {
                    topology.require_final(node_index, graph)?;
                    let identity = identity_for(root_operation, leaf_id)?;
                    validate_identity(root_operation, leaf_id, &identity)?;
                    let record =
                        paths.record(identity, LeafOutcomeV1::Excluded(mapping::exclusion(proof)));
                    emit_record(
                        &mut accumulator,
                        &mut emit_leaf,
                        &mut seen_leaf_ids,
                        leaf_id,
                        &record,
                        None,
                    )?;
                    pop_frame(&mut stack, &mut paths, &mut active);
                    continue;
                }
                graph::NodeKind::Decision => topology.require_decision(node_index, graph)?,
                graph::NodeKind::Continuation { expansion_owner } => {
                    if expansion_owner.is_empty() {
                        return Err("source-leaf adapter found an unnamed continuation".to_owned());
                    }
                    topology.require_continuation(node_index, graph)?;
                }
            }
        }

        let node_index = stack[frame_index].node;
        if stack[frame_index].next_child == topology.outgoing[node_index].len() {
            pop_frame(&mut stack, &mut paths, &mut active);
            continue;
        }
        let edge_index = stack[frame_index].next_child;
        stack[frame_index].next_child += 1;
        let (next, decision) = &topology.outgoing[node_index][edge_index];
        let restore_decisions = paths.decisions.len();
        let restore_sources = paths.sources.len();
        paths.decisions.push(mapping::decision(decision));
        stack.push(Frame {
            node: *next,
            next_child: 0,
            restore_decisions,
            restore_sources,
            entered: false,
        });
    }

    if seen_nodes.len() != graph.nodes.len() {
        return Err(format!(
            "source-leaf adapter found {} unreachable graph nodes",
            graph.nodes.len() - seen_nodes.len()
        ));
    }
    if seen_leaf_ids.len() != graph.source_leaf_universe.len()
        || graph
            .source_leaf_universe
            .iter()
            .any(|leaf_id| !seen_leaf_ids.contains(leaf_id))
    {
        let mut missing = graph
            .source_leaf_universe
            .iter()
            .filter(|leaf_id| !seen_leaf_ids.contains(*leaf_id))
            .cloned()
            .collect::<Vec<_>>();
        missing.sort();
        missing.truncate(8);
        let mut extra = seen_leaf_ids
            .iter()
            .filter(|leaf_id| !graph.source_leaf_universe.contains(*leaf_id))
            .cloned()
            .collect::<Vec<_>>();
        extra.sort();
        extra.truncate(8);
        return Err(format!(
            "source-leaf adapter partition differs from graph universe; missing={missing:?}, extra={extra:?}"
        ));
    }
    let manifest = accumulator.finish()?;
    if manifest.included_count != graph.declared_denominator as u64 {
        return Err(format!(
            "streamed included count {} differs from declared denominator {}",
            manifest.included_count, graph.declared_denominator
        ));
    }
    Ok(manifest)
}

/// One-call runtime gate for a checked-in TSV + 256-shard manifest pair.
pub(crate) fn validate_graph_against_frozen(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    frozen_leaf_seal_tsv: &str,
    frozen_manifest: &RootManifestV1,
) -> Result<u64, String> {
    validate_graph_against_frozen_with_records(
        graph,
        context,
        frozen_leaf_seal_tsv,
        frozen_manifest,
        |_| Ok(()),
    )
}

/// Frozen-static ingress for dynamic quotient construction.
///
/// The compact seal is checked before the observer can consume the corresponding full record.
/// The final root-manifest equality remains a required postcondition of the whole traversal.
pub(crate) fn validate_graph_against_frozen_with_records<F>(
    graph: &graph::ContractGraph,
    context: ManifestContextV1,
    frozen_leaf_seal_tsv: &str,
    frozen_manifest: &RootManifestV1,
    mut observe_leaf: F,
) -> Result<u64, String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    let ledger_sha256 = leaf_seal_tsv_sha256(frozen_leaf_seal_tsv)?;
    if context.ledger_sha256 != ledger_sha256 {
        return Err(format!(
            "manifest context ledger SHA-256 does not bind the canonical leaf TSV; declared={}, actual={}",
            context.ledger_sha256.to_lower_hex(),
            ledger_sha256.to_lower_hex()
        ));
    }
    let mut verifier =
        FrozenLeafSealVerifierV1::from_tsv(frozen_leaf_seal_tsv, context.ledger_sha256)?;
    let actual = stream_graph_manifest_with_records(graph, context, |leaf| {
        verifier.observe(leaf.seal())?;
        observe_leaf(leaf)
    })?;
    let observed = verifier.finish()?;
    if observed != actual.included_count + actual.excluded_count {
        return Err("frozen leaf verifier count differs from streamed manifest".to_owned());
    }
    validate_derived_manifest_against_frozen(&actual, frozen_manifest)
        .map_err(|error| format!("frozen root manifest validation failed: {error:?}"))?;
    Ok(actual.included_count)
}

fn validate_identity(
    root: RootOperationV1,
    leaf_id: &str,
    identity: &LeafIdentityV1,
) -> Result<(), String> {
    if identity.root != root || identity.leaf_id != leaf_id {
        return Err(format!(
            "identity mapper changed graph leaf identity {leaf_id:?}: {identity:?}"
        ));
    }
    Ok(())
}

fn emit_record<F>(
    accumulator: &mut ManifestAccumulatorV1,
    emit_leaf: &mut F,
    seen_leaf_ids: &mut HashSet<String>,
    leaf_id: &str,
    record: &LeafRecordV1,
    descriptor: Option<&TypedTerminalDescriptorV1>,
) -> Result<(), String>
where
    F: FnMut(StreamedLeafV1<'_>) -> Result<(), String>,
{
    if !seen_leaf_ids.insert(leaf_id.to_owned()) {
        return Err(format!("graph source leaf has two root paths: {leaf_id:?}"));
    }
    let seal = accumulator.push(record)?;
    match descriptor {
        Some(descriptor) => emit_leaf(StreamedLeafV1::Terminal {
            record,
            descriptor,
            seal: &seal,
        }),
        None => emit_leaf(StreamedLeafV1::Excluded {
            record,
            seal: &seal,
        }),
    }
}

#[derive(Default)]
struct ActivePaths {
    decisions: Vec<super::model::DecisionV1>,
    sources: Vec<super::model::SourceWitnessV1>,
}

impl ActivePaths {
    fn record(&self, identity: LeafIdentityV1, outcome: LeafOutcomeV1) -> LeafRecordV1 {
        LeafRecordV1 {
            key: CaseKeyV1 {
                identity,
                decisions: self.decisions.clone(),
            },
            source_branch: self.sources.clone(),
            outcome,
        }
    }
}

struct Frame {
    node: usize,
    next_child: usize,
    restore_decisions: usize,
    restore_sources: usize,
    entered: bool,
}

impl Frame {
    const fn root(node: usize) -> Self {
        Self {
            node,
            next_child: 0,
            restore_decisions: 0,
            restore_sources: 0,
            entered: false,
        }
    }
}

fn pop_frame(stack: &mut Vec<Frame>, paths: &mut ActivePaths, active: &mut [bool]) {
    let frame = stack
        .pop()
        .expect("pop_frame is called only for a live frame");
    active[frame.node] = false;
    paths.decisions.truncate(frame.restore_decisions);
    paths.sources.truncate(frame.restore_sources);
}

struct Topology<'graph> {
    root: usize,
    outgoing: Vec<Vec<(usize, &'graph graph::Decision)>>,
}

impl<'graph> Topology<'graph> {
    fn new(graph: &'graph graph::ContractGraph) -> Result<Self, String> {
        let mut indices = BTreeMap::new();
        for (index, node) in graph.nodes.iter().enumerate() {
            if node.id.is_empty() || indices.insert(node.id.as_str(), index).is_some() {
                return Err(format!("graph repeats or empties node id {:?}", node.id));
            }
        }
        let root = indices
            .get(graph.root.as_str())
            .copied()
            .ok_or_else(|| format!("graph root is absent: {:?}", graph.root))?;
        let mut outgoing = vec![Vec::new(); graph.nodes.len()];
        for edge in &graph.edges {
            if edge.decision.branch.is_empty() {
                return Err("graph edge has an empty decision branch".to_owned());
            }
            let from = indices
                .get(edge.from.as_str())
                .copied()
                .ok_or_else(|| format!("graph edge starts at an unknown node: {:?}", edge.from))?;
            let to = indices
                .get(edge.to.as_str())
                .copied()
                .ok_or_else(|| format!("graph edge ends at an unknown node: {:?}", edge.to))?;
            outgoing[from].push((to, &edge.decision));
        }
        for edges in &mut outgoing {
            edges.sort_by(|(left_node, left), (right_node, right)| {
                left.cmp(right)
                    .then_with(|| graph.nodes[*left_node].id.cmp(&graph.nodes[*right_node].id))
            });
            if edges.windows(2).any(|pair| pair[0].1 == pair[1].1) {
                return Err("graph node repeats an outgoing decision".to_owned());
            }
        }
        Ok(Self { root, outgoing })
    }

    fn require_final(&self, node: usize, graph: &graph::ContractGraph) -> Result<(), String> {
        if self.outgoing[node].is_empty() {
            Ok(())
        } else {
            Err(format!(
                "final node has outgoing edges: {:?}",
                graph.nodes[node].id
            ))
        }
    }

    fn require_decision(&self, node: usize, graph: &graph::ContractGraph) -> Result<(), String> {
        if self.outgoing[node].is_empty() {
            Err(format!(
                "decision node has no branch: {:?}",
                graph.nodes[node].id
            ))
        } else {
            Ok(())
        }
    }

    fn require_continuation(
        &self,
        node: usize,
        graph: &graph::ContractGraph,
    ) -> Result<(), String> {
        if self.outgoing[node].len() == 1 {
            Ok(())
        } else {
            Err(format!(
                "continuation must have one expansion: {:?}",
                graph.nodes[node].id
            ))
        }
    }
}
