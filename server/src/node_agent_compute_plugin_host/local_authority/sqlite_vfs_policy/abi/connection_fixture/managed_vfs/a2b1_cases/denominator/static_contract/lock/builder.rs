use std::collections::BTreeSet;

use super::super::{
    initialization::{InitializationExpansion, InitializationFailure, InitializationSuccess},
    model::{
        ContractEdge, ContractGraph, ContractNode, Decision, DecisionStage, ExclusionProof,
        Expected, NodeKind, RootOperation,
    },
    source::SourceWitness,
};

pub(super) struct Builder {
    nodes: Vec<ContractNode>,
    edges: Vec<ContractEdge>,
    node_ids: BTreeSet<String>,
    source_leaves: BTreeSet<String>,
    terminal_count: usize,
}

impl Builder {
    pub(super) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_ids: BTreeSet::new(),
            source_leaves: BTreeSet::new(),
            terminal_count: 0,
        }
    }

    pub(super) fn decision(&mut self, id: impl Into<String>, source: SourceWitness) -> String {
        self.node(id, NodeKind::Decision, source)
    }

    pub(super) fn continuation(
        &mut self,
        id: impl Into<String>,
        owner: &'static str,
        source: SourceWitness,
    ) -> String {
        self.node(
            id,
            NodeKind::Continuation {
                expansion_owner: owner,
            },
            source,
        )
    }

    pub(super) fn terminal(
        &mut self,
        id: impl Into<String>,
        expected: Expected,
        source: SourceWitness,
    ) -> String {
        let id = id.into();
        let leaf_id = id.clone();
        assert!(
            self.source_leaves.insert(leaf_id.clone()),
            "duplicate Lock leaf: {leaf_id}"
        );
        self.terminal_count += 1;
        self.node(id, NodeKind::Terminal { leaf_id, expected }, source)
    }

    #[allow(dead_code)]
    pub(super) fn excluded(
        &mut self,
        id: impl Into<String>,
        proof: ExclusionProof,
        source: SourceWitness,
    ) -> String {
        let id = id.into();
        let leaf_id = id.clone();
        assert!(
            self.source_leaves.insert(leaf_id.clone()),
            "duplicate Lock leaf: {leaf_id}"
        );
        self.node(id, NodeKind::Excluded { leaf_id, proof }, source)
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
    ) -> (Vec<InitializationSuccess>, Vec<InitializationFailure>) {
        let declared_leaves: BTreeSet<_> = expansion.leaf_ids.iter().cloned().collect();
        let mut observed_leaves = BTreeSet::new();
        for node in expansion.nodes {
            match &node.kind {
                NodeKind::Terminal { leaf_id, .. } => {
                    self.terminal_count += 1;
                    observed_leaves.insert(leaf_id.clone());
                    assert!(
                        self.source_leaves.insert(leaf_id.clone()),
                        "duplicate init leaf: {leaf_id}"
                    );
                }
                NodeKind::Excluded { leaf_id, .. } => {
                    observed_leaves.insert(leaf_id.clone());
                    assert!(
                        self.source_leaves.insert(leaf_id.clone()),
                        "duplicate init leaf: {leaf_id}"
                    );
                }
                NodeKind::Decision | NodeKind::Continuation { .. } => {}
            }
            assert!(
                node.witness.is_some(),
                "Lock initialization node lacks source witness: {}",
                node.id
            );
            assert!(
                self.node_ids.insert(node.id.clone()),
                "duplicate Lock node: {}",
                node.id
            );
            self.nodes.push(node);
        }
        assert_eq!(
            declared_leaves, observed_leaves,
            "initialization leaf inventory drift"
        );
        self.edges.extend(expansion.edges);
        (expansion.successes, expansion.failures)
    }

    pub(super) fn finish(self, root: String) -> ContractGraph {
        assert!(self.node_ids.contains(&root), "Lock root is absent");
        ContractGraph {
            name: "Lock",
            root_operation: RootOperation::Lock,
            root,
            nodes: self.nodes,
            edges: self.edges,
            source_leaf_universe: self.source_leaves,
            declared_denominator: self.terminal_count,
            legacy_non_denominator_width: 10,
        }
    }

    fn node(&mut self, id: impl Into<String>, kind: NodeKind, source: SourceWitness) -> String {
        let id = id.into();
        assert!(
            self.node_ids.insert(id.clone()),
            "duplicate Lock node: {id}"
        );
        self.nodes.push(ContractNode {
            id: id.clone(),
            kind,
            witness: Some(source),
        });
        id
    }
}
