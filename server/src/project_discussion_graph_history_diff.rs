//! Bounded semantic diffs shared by discussion history and lifecycle queries.

use serde_json::{json, Value};
use std::collections::HashMap;

use crate::project_discussion_graph_model::{
    DiscussionEdge, DiscussionGraph, DiscussionNode, DiscussionSource,
};

const MAX_CHANGE_ITEMS: usize = 200;

pub(super) fn semantic_diff(base: &DiscussionGraph, target: &DiscussionGraph) -> Value {
    let base_nodes = by_id(&base.nodes, |item| &item.id);
    let target_nodes = by_id(&target.nodes, |item| &item.id);
    let base_edges = by_id(&base.edges, |item| &item.id);
    let target_edges = by_id(&target.edges, |item| &item.id);
    let base_sources = by_id(&base.sources, |item| &item.id);
    let target_sources = by_id(&target.sources, |item| &item.id);
    let node_added = added(&base_nodes, &target_nodes, node_receipt);
    let node_removed = removed(&base_nodes, &target_nodes, node_receipt);
    let node_changed = changed(&base_nodes, &target_nodes, changed_node);
    let edge_added = added(&base_edges, &target_edges, edge_receipt);
    let edge_removed = removed(&base_edges, &target_edges, edge_receipt);
    let edge_changed = changed(&base_edges, &target_edges, changed_edge);
    let source_added = added(&base_sources, &target_sources, source_receipt);
    let source_removed = removed(&base_sources, &target_sources, source_receipt);
    let source_changed = changed(&base_sources, &target_sources, changed_source);
    let total = node_added.len()
        + node_removed.len()
        + node_changed.len()
        + edge_added.len()
        + edge_removed.len()
        + edge_changed.len()
        + source_added.len()
        + source_removed.len()
        + source_changed.len();
    json!({
        "counts": {
            "nodes_added": node_added.len(),
            "nodes_removed": node_removed.len(),
            "nodes_changed": node_changed.len(),
            "edges_added": edge_added.len(),
            "edges_removed": edge_removed.len(),
            "edges_changed": edge_changed.len(),
            "sources_added": source_added.len(),
            "sources_removed": source_removed.len(),
            "sources_changed": source_changed.len(),
            "total_changes": total,
        },
        "nodes": {
            "added": bounded(node_added),
            "removed": bounded(node_removed),
            "changed": bounded(node_changed),
        },
        "edges": {
            "added": bounded(edge_added),
            "removed": bounded(edge_removed),
            "changed": bounded(edge_changed),
        },
        "sources": {
            "added": bounded(source_added),
            "removed": bounded(source_removed),
            "changed": bounded(source_changed),
        },
        "truncated": total > MAX_CHANGE_ITEMS,
    })
}

pub(super) fn node_changed_fields(
    before: &DiscussionNode,
    after: &DiscussionNode,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    macro_rules! changed {
        ($field:ident) => {
            if before.$field != after.$field {
                fields.push(stringify!($field));
            }
        };
    }
    changed!(root_id);
    changed!(parent_id);
    changed!(kind);
    changed!(title);
    changed!(summary);
    changed!(status);
    changed!(authority);
    changed!(section_id);
    changed!(order);
    changed!(color);
    changed!(source_refs);
    changed!(conversation_refs);
    changed!(document_paths);
    changed!(feature_node_ids);
    changed!(tags);
    fields
}

pub(super) fn incident_edges(
    graph: &DiscussionGraph,
    node_id: &str,
) -> HashMap<String, DiscussionEdge> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.source == node_id || edge.target == node_id)
        .map(|edge| (edge.id.clone(), edge.clone()))
        .collect()
}

fn changed_node(before: &DiscussionNode, after: &DiscussionNode) -> Value {
    json!({
        "id": after.id,
        "title": after.title,
        "changed_fields": node_changed_fields(before, after),
        "from_status": before.status,
        "to_status": after.status,
        "from_parent_id": before.parent_id,
        "to_parent_id": after.parent_id,
    })
}

fn changed_edge(before: &DiscussionEdge, after: &DiscussionEdge) -> Value {
    let mut fields = Vec::new();
    if before.source != after.source {
        fields.push("source");
    }
    if before.target != after.target {
        fields.push("target");
    }
    if before.relation != after.relation {
        fields.push("relation");
    }
    if before.label != after.label {
        fields.push("label");
    }
    json!({
        "id": after.id,
        "changed_fields": fields,
        "from_relation": before.relation,
        "to_relation": after.relation,
    })
}

fn changed_source(before: &DiscussionSource, after: &DiscussionSource) -> Value {
    let mut fields = Vec::new();
    if before.title != after.title {
        fields.push("title");
    }
    if before.kind != after.kind {
        fields.push("kind");
    }
    if before.reference != after.reference {
        fields.push("reference");
    }
    if before.imported_at != after.imported_at {
        fields.push("imported_at");
    }
    json!({"id":after.id,"title":after.title,"changed_fields":fields})
}

fn node_receipt(node: &DiscussionNode) -> Value {
    json!({
        "id": node.id,
        "title": node.title,
        "kind": node.kind,
        "status": node.status,
        "parent_id": node.parent_id,
    })
}

fn edge_receipt(edge: &DiscussionEdge) -> Value {
    json!({
        "id": edge.id,
        "source": edge.source,
        "target": edge.target,
        "relation": edge.relation,
        "label": edge.label,
    })
}

fn source_receipt(source: &DiscussionSource) -> Value {
    json!({
        "id": source.id,
        "title": source.title,
        "kind": source.kind,
        "reference": source.reference,
    })
}

fn by_id<'a, T>(
    items: &'a [T],
    id: impl Fn(&'a T) -> &'a String,
) -> HashMap<&'a str, &'a T> {
    items.iter().map(|item| (id(item).as_str(), item)).collect()
}

fn added<T>(
    base: &HashMap<&str, &T>,
    target: &HashMap<&str, &T>,
    receipt: impl Fn(&T) -> Value,
) -> Vec<Value> {
    let mut values = target
        .iter()
        .filter(|(id, _)| !base.contains_key(*id))
        .map(|(_, value)| receipt(value))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    values
}

fn removed<T>(
    base: &HashMap<&str, &T>,
    target: &HashMap<&str, &T>,
    receipt: impl Fn(&T) -> Value,
) -> Vec<Value> {
    added(target, base, receipt)
}

fn changed<T: PartialEq>(
    base: &HashMap<&str, &T>,
    target: &HashMap<&str, &T>,
    receipt: impl Fn(&T, &T) -> Value,
) -> Vec<Value> {
    let mut values = target
        .iter()
        .filter_map(|(id, after)| {
            let before = base.get(id)?;
            (*before != *after).then(|| receipt(before, after))
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
    values
}

fn bounded(mut values: Vec<Value>) -> Vec<Value> {
    values.truncate(MAX_CHANGE_ITEMS);
    values
}
