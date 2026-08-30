use super::super::model::{ContractGraph, NodeKind, RootOperation, SqliteResult};
use super::validate_graph;

pub(super) fn validate_standard_negative_clones(graph: &ContractGraph) -> Result<(), String> {
    validate_graph(graph)?;
    let reject = |label: &str, clone: ContractGraph| {
        validate_graph(&clone)
            .err()
            .map(|_| ())
            .ok_or_else(|| format!("{label} clone was falsely accepted"))
    };
    let mut removed = graph.clone();
    let leaf = removed.source_leaf_universe.iter().next().unwrap().clone();
    removed.source_leaf_universe.remove(&leaf);
    reject("removed leaf", removed)?;
    let mut added = graph.clone();
    added
        .source_leaf_universe
        .insert("invariant.invented.leaf".to_owned());
    reject("added leaf", added)?;
    let mut tampered = graph.clone();
    let expected = tampered
        .nodes
        .iter_mut()
        .find_map(|node| match &mut node.kind {
            NodeKind::Terminal { expected, .. } => Some(expected),
            _ => None,
        })
        .ok_or_else(|| "negative clone source has no terminal".to_owned())?;
    expected.sqlite = match graph.root_operation {
        RootOperation::Map => SqliteResult::LockUnavailable,
        RootOperation::Lock => SqliteResult::MapUnavailable,
    };
    reject("Expected tamper", tampered)?;
    let mut inflated = graph.clone();
    inflated.declared_denominator += inflated.legacy_non_denominator_width;
    reject("legacy-width inflation", inflated)
}
