//! Independent ledger-to-graph coverage gate.
//!
//! Frozen leaf seals prove an exact graph inventory, while this module proves that the separately
//! reviewed Map profile/ordinal and Lock range ledgers actually index that inventory.  It never
//! calls graph range/profile constructors and never derives authority rows from graph contents.

mod lock;
mod map;

use std::collections::{BTreeMap, BTreeSet};

use super::super::model as graph;

pub(super) fn validate_graph_ledger_coverage(
    contract: &graph::ContractGraph,
) -> Result<(), String> {
    let index = GraphIndex::new(contract)?;
    match contract.root_operation {
        graph::RootOperation::Map => map::validate(contract, &index),
        graph::RootOperation::Lock => lock::validate(contract, &index),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequiredKind {
    Decision,
    Continuation,
    Terminal,
    Excluded,
}

pub(super) struct GraphIndex<'graph> {
    nodes: BTreeMap<&'graph str, &'graph graph::ContractNode>,
    edges: BTreeSet<(&'graph str, &'graph str, graph::DecisionStage, &'graph str)>,
}

impl<'graph> GraphIndex<'graph> {
    fn new(contract: &'graph graph::ContractGraph) -> Result<Self, String> {
        let mut nodes = BTreeMap::new();
        for node in &contract.nodes {
            if node.id.is_empty() || nodes.insert(node.id.as_str(), node).is_some() {
                return Err(format!(
                    "ledger coverage found a repeated or empty graph node id: {:?}",
                    node.id
                ));
            }
        }
        let edges = contract
            .edges
            .iter()
            .map(|edge| {
                (
                    edge.from.as_str(),
                    edge.to.as_str(),
                    edge.decision.stage,
                    edge.decision.branch.as_str(),
                )
            })
            .collect();
        Ok(Self { nodes, edges })
    }

    pub(super) fn has_node(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    pub(super) fn require_node(
        &self,
        id: &str,
        required: RequiredKind,
    ) -> Result<&'graph graph::ContractNode, String> {
        let node = self
            .nodes
            .get(id)
            .copied()
            .ok_or_else(|| format!("ledger coverage is missing graph node {id:?}"))?;
        let actual = match node.kind {
            graph::NodeKind::Decision => RequiredKind::Decision,
            graph::NodeKind::Continuation { .. } => RequiredKind::Continuation,
            graph::NodeKind::Terminal { .. } => RequiredKind::Terminal,
            graph::NodeKind::Excluded { .. } => RequiredKind::Excluded,
        };
        if actual != required {
            return Err(format!(
                "ledger coverage node {id:?} has kind {actual:?}, expected {required:?}"
            ));
        }
        Ok(node)
    }

    pub(super) fn require_leaf(
        &self,
        contract: &graph::ContractGraph,
        id: &str,
        required: RequiredKind,
    ) -> Result<&'graph graph::ContractNode, String> {
        if !matches!(required, RequiredKind::Terminal | RequiredKind::Excluded) {
            return Err("ledger coverage leaf check requires a final node kind".to_owned());
        }
        if !contract.source_leaf_universe.contains(id) {
            return Err(format!(
                "ledger coverage source-leaf universe is missing {id:?}"
            ));
        }
        self.require_node(id, required)
    }

    pub(super) fn require_edge(
        &self,
        from: &str,
        to: &str,
        stage: graph::DecisionStage,
        branch: &str,
    ) -> Result<(), String> {
        if self.edges.contains(&(from, to, stage, branch)) {
            Ok(())
        } else {
            Err(format!(
                "ledger coverage is missing edge {from:?} -> {to:?} at {stage:?}/{branch:?}"
            ))
        }
    }
}

#[cfg(test)]
mod tests;
