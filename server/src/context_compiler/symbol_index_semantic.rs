use std::collections::{BTreeMap, BTreeSet};

use super::symbol_index::{SymbolEdge, SymbolIndex};

#[allow(dead_code)]
impl SymbolIndex {
    pub(crate) fn definitions_for(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.outgoing_edges_of(symbol_id)
            .into_iter()
            .filter(|edge| edge.kind == "definition")
            .collect()
    }

    pub(crate) fn implementations_for(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.outgoing_edges_of(symbol_id)
            .into_iter()
            .chain(self.incoming_edges_of(symbol_id))
            .filter(|edge| is_implementation_edge(edge))
            .collect()
    }

    pub(crate) fn callers_of(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.incoming_edges_of(symbol_id)
            .into_iter()
            .filter(|edge| is_call_edge(edge))
            .collect()
    }

    pub(crate) fn callees_of(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.outgoing_edges_of(symbol_id)
            .into_iter()
            .filter(|edge| is_call_edge(edge))
            .collect()
    }

    pub(crate) fn semantic_edges_for(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        let mut indexes = BTreeSet::new();
        if let Some(incoming) = self.incoming_edges.get(symbol_id) {
            indexes.extend(incoming.iter().copied());
        }
        if let Some(outgoing) = self.outgoing_edges.get(symbol_id) {
            indexes.extend(outgoing.iter().copied());
        }
        indexes
            .into_iter()
            .filter_map(|index| self.edges.get(index))
            .filter(|edge| is_precise_semantic_edge(edge) || is_normalized_semantic_edge(edge))
            .collect()
    }

    pub(crate) fn edge_kind_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for edge in &self.edges {
            *counts.entry(edge.kind.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub(crate) fn edge_source_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for edge in &self.edges {
            *counts.entry(edge.source.to_string()).or_insert(0) += 1;
        }
        counts
    }

    pub(crate) fn precise_semantic_edge_count(&self) -> usize {
        self.edges
            .iter()
            .filter(|edge| is_precise_semantic_edge(edge))
            .count()
    }

    fn incoming_edges_of(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.incoming_edges
            .get(symbol_id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .collect()
    }

    fn outgoing_edges_of(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.outgoing_edges
            .get(symbol_id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .collect()
    }
}

pub(super) fn is_reference_edge(edge: &SymbolEdge) -> bool {
    matches!(edge.kind.as_str(), "reference" | "references" | "def_ref")
}

#[allow(dead_code)]
fn is_implementation_edge(edge: &SymbolEdge) -> bool {
    matches!(edge.kind.as_str(), "implementation" | "implements")
}

#[allow(dead_code)]
fn is_call_edge(edge: &SymbolEdge) -> bool {
    matches!(
        edge.kind.as_str(),
        "calls" | "incoming_call" | "outgoing_call"
    )
}

fn is_precise_semantic_edge(edge: &SymbolEdge) -> bool {
    edge.source == "rust_analyzer_lsp"
        && matches!(
            edge.kind.as_str(),
            "definition" | "reference" | "implementation" | "incoming_call" | "outgoing_call"
        )
}

#[allow(dead_code)]
fn is_normalized_semantic_edge(edge: &SymbolEdge) -> bool {
    matches!(
        edge.kind.as_str(),
        "calls" | "implements" | "references" | "type_uses" | "test_covers"
    )
}
