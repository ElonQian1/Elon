use std::collections::BTreeSet;

use super::super::{
    initialization::{InitializationExpansion, InitializationFailure, InitializationSuccess},
    model::{
        ContractEdge, ContractGraph, ContractNode, Decision, DecisionStage, ExclusionProof,
        Expected, NodeKind, RootOperation,
    },
    source::SourceWitness,
};

pub(super) struct MapGraphBuilder {
    nodes: Vec<ContractNode>,
    node_ids: BTreeSet<String>,
    edges: Vec<ContractEdge>,
    leaves: BTreeSet<String>,
    terminal_cases: usize,
}

impl MapGraphBuilder {
    pub(super) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            node_ids: BTreeSet::new(),
            edges: Vec::new(),
            leaves: BTreeSet::new(),
            terminal_cases: 0,
        }
    }

    pub(super) fn decision(&mut self, id: &str, source: SourceWitness) -> String {
        self.push_node(id, NodeKind::Decision, source)
    }

    pub(super) fn terminal(
        &mut self,
        id: &str,
        expected: Expected,
        source: SourceWitness,
    ) -> String {
        let leaf_id = id.to_owned();
        assert!(
            self.leaves.insert(leaf_id.clone()),
            "duplicate Map leaf {id}"
        );
        self.terminal_cases += 1;
        self.push_node(id, NodeKind::Terminal { leaf_id, expected }, source)
    }

    pub(super) fn excluded(
        &mut self,
        id: &str,
        proof: ExclusionProof,
        source: SourceWitness,
    ) -> String {
        let leaf_id = id.to_owned();
        assert!(
            self.leaves.insert(leaf_id.clone()),
            "duplicate Map leaf {id}"
        );
        self.push_node(id, NodeKind::Excluded { leaf_id, proof }, source)
    }

    pub(super) fn edge(
        &mut self,
        from: &str,
        to: &str,
        stage: DecisionStage,
        branch: impl Into<String>,
    ) {
        self.edges.push(ContractEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            decision: Decision::new(stage, branch),
        });
    }

    pub(super) fn merge_initialization(
        &mut self,
        expansion: InitializationExpansion,
    ) -> (
        String,
        Vec<InitializationSuccess>,
        Vec<InitializationFailure>,
    ) {
        for leaf in expansion.leaf_ids {
            assert!(
                self.leaves.insert(leaf),
                "duplicate shared initialization leaf"
            );
        }
        self.terminal_cases += expansion
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::Terminal { .. }))
            .count();
        for node in &expansion.nodes {
            assert!(
                self.node_ids.insert(node.id.clone()),
                "duplicate shared initialization node {}",
                node.id
            );
        }
        self.nodes.extend(expansion.nodes);
        self.edges.extend(expansion.edges);
        (expansion.entry, expansion.successes, expansion.failures)
    }

    pub(super) fn finish(self, root: &str) -> ContractGraph {
        ContractGraph {
            name: "Map",
            root_operation: RootOperation::Map,
            root: root.to_owned(),
            nodes: self.nodes,
            edges: self.edges,
            source_leaf_universe: self.leaves,
            declared_denominator: self.terminal_cases,
            legacy_non_denominator_width: 18,
        }
    }

    fn push_node(&mut self, id: &str, kind: NodeKind, source: SourceWitness) -> String {
        let id = id.to_owned();
        assert!(self.node_ids.insert(id.clone()), "duplicate Map node {id}");
        self.nodes.push(ContractNode {
            id: id.clone(),
            kind,
            witness: Some(source),
        });
        id
    }
}
