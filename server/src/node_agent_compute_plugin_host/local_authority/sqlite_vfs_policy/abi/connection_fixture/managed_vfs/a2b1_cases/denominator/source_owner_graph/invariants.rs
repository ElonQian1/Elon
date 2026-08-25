mod ordered;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sha2::{Digest, Sha256};

use super::{
    lock, map,
    model::{EdgeKind, PathOp, SourceEdge, SourceNode, SourceNodeId, ALL_OPS},
    owners::{self, OWNERS, SOURCE_BASELINE_COMMIT},
    shared,
};

pub(super) fn validate() -> Result<(), &'static str> {
    validate_owner_snapshots()?;
    let nodes = all_nodes();
    let edges = all_edges();
    validate_nodes(&nodes)?;
    validate_edges(&nodes, &edges)?;
    validate_roots_and_reachability(&nodes, &edges)?;
    ordered::validate(&nodes, &edges)?;
    Ok(())
}

fn all_nodes() -> Vec<SourceNode> {
    shared::NODES
        .iter()
        .chain(map::NODES)
        .chain(lock::NODES)
        .copied()
        .collect()
}

fn all_edges() -> Vec<SourceEdge> {
    shared::EDGES
        .iter()
        .chain(map::ROUTE_EDGES)
        .chain(map::OPERATION_EDGES)
        .chain(lock::PREFIX_EDGES)
        .chain(lock::MANAGED_EDGES)
        .copied()
        .collect()
}

fn validate_owner_snapshots() -> Result<(), &'static str> {
    if !lower_hex(SOURCE_BASELINE_COMMIT, 40) {
        return Err("source owner graph baseline commit is not lowercase SHA-1");
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for owner in OWNERS {
        if !ids.insert(owner.id) || !paths.insert(owner.path) {
            return Err("duplicate source owner id or path");
        }
        if !lower_hex(owner.blob_oid, 40) || !lower_hex(owner.normalized_sha256, 64) {
            return Err("source owner snapshot digest has the wrong shape");
        }
        if owner.symbols.is_empty() {
            return Err("source owner snapshot has no symbol sentinels");
        }
        let source = owners::source_content(owner.id);
        let mut symbols = BTreeSet::new();
        for symbol in owner.symbols {
            if symbol.is_empty() || !symbols.insert(*symbol) || !source.contains(symbol) {
                return Err("source owner symbol sentinel is missing or duplicated");
            }
        }
        let normalized = source.replace("\r\n", "\n");
        let digest = Sha256::digest(normalized.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if digest != owner.normalized_sha256 {
            return Err("source owner bytes changed after graph review");
        }
        let header = format!("blob {}\0", normalized.len());
        let mut git_blob = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
        git_blob.update(header.as_bytes());
        git_blob.update(normalized.as_bytes());
        let git_oid = git_blob
            .finish()
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if git_oid != owner.blob_oid {
            return Err("source owner bytes do not match the reviewed Git blob OID");
        }
    }
    Ok(())
}

fn validate_nodes(nodes: &[SourceNode]) -> Result<(), &'static str> {
    let owners = OWNERS.iter().map(|owner| owner.id).collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut referenced_owners = BTreeSet::new();
    for node in nodes {
        if !ids.insert(node.id) || !owners.contains(&node.owner) {
            return Err("duplicate source node or detached source owner");
        }
        if node.symbol.is_empty() || !owners::source_content(node.owner).contains(node.symbol) {
            return Err("source node symbol is not present in its reviewed owner");
        }
        validate_ops(node.ops)?;
        referenced_owners.insert(node.owner);
        let _reviewed_shape = (node.role, node.epoch, node.boundary, node.state_witness);
    }
    if referenced_owners != owners {
        return Err("source owner snapshot is not represented by a graph node");
    }
    Ok(())
}

fn validate_edges(nodes: &[SourceNode], edges: &[SourceEdge]) -> Result<(), &'static str> {
    let node_map = nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();
    let mut ids = BTreeSet::new();
    let mut shapes = BTreeSet::new();
    for edge in edges {
        let Some(from) = node_map.get(&edge.from) else {
            return Err("source edge has a missing from endpoint");
        };
        let Some(to) = node_map.get(&edge.to) else {
            return Err("source edge has a missing to endpoint");
        };
        if edge.id.is_empty()
            || !ids.insert(edge.id)
            || !shapes.insert((edge.from, edge.to, edge.kind, edge.ops))
        {
            return Err("source edge id or typed shape is duplicated");
        }
        validate_ops(edge.ops)?;
        if edge
            .ops
            .iter()
            .any(|op| !from.ops.contains(op) || !to.ops.contains(op))
        {
            return Err("source edge operation scope escapes an endpoint");
        }
        let _reviewed_shape = (edge.epoch, edge.reachability, edge.effect);
    }
    Ok(())
}

fn validate_roots_and_reachability(
    nodes: &[SourceNode],
    edges: &[SourceEdge],
) -> Result<(), &'static str> {
    let mut incoming = nodes
        .iter()
        .map(|node| (node.id, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<SourceNodeId, Vec<SourceNodeId>>::new();
    for edge in edges {
        *incoming.get_mut(&edge.to).expect("validated endpoint") += 1;
        outgoing.entry(edge.from).or_default().push(edge.to);
    }
    let roots = incoming
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect::<BTreeSet<_>>();
    let expected = [SourceNodeId::AbiMapSlot, SourceNodeId::AbiLockSlot]
        .into_iter()
        .collect::<BTreeSet<_>>();
    if roots != expected {
        return Err("source owner graph roots are not exactly ABI xShmMap/xShmLock slots");
    }
    let mut reached = roots.clone();
    let mut queue = roots.into_iter().collect::<VecDeque<_>>();
    while let Some(node) = queue.pop_front() {
        for next in outgoing.get(&node).into_iter().flatten() {
            if reached.insert(*next) {
                queue.push_back(*next);
            }
        }
    }
    let all = nodes.iter().map(|node| node.id).collect::<BTreeSet<_>>();
    if reached != all {
        return Err("source owner graph contains a node unreachable from both ABI roots");
    }
    for op in ALL_OPS {
        validate_operation_reachability(*op, nodes, edges)?;
    }
    Ok(())
}

fn validate_operation_reachability(
    op: PathOp,
    nodes: &[SourceNode],
    edges: &[SourceEdge],
) -> Result<(), &'static str> {
    let root = match op {
        PathOp::MapObserve | PathOp::MapExtend => SourceNodeId::AbiMapSlot,
        PathOp::LockShared
        | PathOp::LockExclusive
        | PathOp::UnlockShared
        | PathOp::UnlockExclusive => SourceNodeId::AbiLockSlot,
    };
    let expected = nodes
        .iter()
        .filter(|node| node.ops.contains(&op))
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let mut reached = [root].into_iter().collect::<BTreeSet<_>>();
    let mut queue = VecDeque::from([root]);
    while let Some(node) = queue.pop_front() {
        for edge in edges.iter().filter(|edge| edge.ops.contains(&op)) {
            let next = if edge.from == node {
                Some(edge.to)
            } else if edge.kind == EdgeKind::StatePrerequisite && edge.to == node {
                Some(edge.from)
            } else {
                None
            };
            if let Some(next) = next.filter(|next| expected.contains(next)) {
                if reached.insert(next) {
                    queue.push_back(next);
                }
            }
        }
    }
    if reached != expected {
        Err("source owner graph contains an operation-scoped node unreachable from its ABI root")
    } else {
        Ok(())
    }
}

fn validate_ops(ops: &[PathOp]) -> Result<(), &'static str> {
    if ops.is_empty() || ops.iter().copied().collect::<BTreeSet<_>>().len() != ops.len() {
        Err("source owner graph operation scope is empty or duplicated")
    } else {
        Ok(())
    }
}

fn lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
