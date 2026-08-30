use std::collections::BTreeSet;

use super::{
    super::model::{ContractGraph, Decision, NodeKind},
    ValidatedGraph,
};

pub(super) fn validate_node_degrees(
    graph: &ContractGraph,
    outgoing: &[Vec<(usize, Decision)>],
) -> Result<(), String> {
    for (index, node) in graph.nodes.iter().enumerate() {
        match &node.kind {
            NodeKind::Decision => {
                if outgoing[index].is_empty() {
                    return Err(format!(
                        "{} decision {:?} has no branch",
                        graph.name, node.id
                    ));
                }
                let mut decisions = BTreeSet::new();
                if outgoing[index]
                    .iter()
                    .any(|(_, decision)| !decisions.insert(decision))
                {
                    return Err(format!(
                        "{} decision {:?} repeats an outgoing branch",
                        graph.name, node.id
                    ));
                }
            }
            NodeKind::Continuation { .. } if outgoing[index].len() != 1 => {
                return Err(format!(
                    "{} continuation {:?} must have exactly one expansion, found {}",
                    graph.name,
                    node.id,
                    outgoing[index].len()
                ));
            }
            NodeKind::Terminal { .. } | NodeKind::Excluded { .. }
                if !outgoing[index].is_empty() =>
            {
                return Err(format!(
                    "{} final node {:?} has outgoing edges",
                    graph.name, node.id
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn visit_acyclic(
    graph: &ContractGraph,
    outgoing: &[Vec<(usize, Decision)>],
    node: usize,
    colors: &mut [u8],
) -> Result<(), String> {
    match colors[node] {
        1 => {
            return Err(format!(
                "cycle at {}::{:?}",
                graph.name, graph.nodes[node].id
            ));
        }
        2 => return Ok(()),
        _ => {}
    }
    colors[node] = 1;
    for (next, _) in &outgoing[node] {
        visit_acyclic(graph, outgoing, *next, colors)?;
    }
    colors[node] = 2;
    Ok(())
}

pub(super) fn collect_final_leaves(
    graph: &ContractGraph,
    outgoing: &[Vec<(usize, Decision)>],
    index: usize,
    validated: &mut ValidatedGraph,
) -> Result<(), String> {
    let node = &graph.nodes[index];
    match &node.kind {
        NodeKind::Terminal { leaf_id, .. } => {
            if !validated.included.insert(leaf_id.clone()) {
                return Err(format!(
                    "{} terminal leaf {leaf_id:?} has two paths",
                    graph.name
                ));
            }
        }
        NodeKind::Excluded { leaf_id, .. } => {
            if !validated.excluded.insert(leaf_id.clone()) {
                return Err(format!(
                    "{} excluded leaf {leaf_id:?} has two paths",
                    graph.name
                ));
            }
        }
        NodeKind::Decision | NodeKind::Continuation { .. } => {
            for (next, _) in &outgoing[index] {
                collect_final_leaves(graph, outgoing, *next, validated)?;
            }
        }
    }
    Ok(())
}
