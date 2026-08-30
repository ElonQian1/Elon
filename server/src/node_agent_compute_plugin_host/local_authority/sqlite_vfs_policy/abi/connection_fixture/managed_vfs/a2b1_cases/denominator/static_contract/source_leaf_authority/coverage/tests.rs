use super::{lock, map, validate_graph_ledger_coverage};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::{
    lock as contract_lock, map as contract_map, model as graph,
};

#[test]
fn independent_ledgers_gate_real_profile_ordinal_and_range_coverage() {
    let mut map_graph = contract_map::graph();
    validate_graph_ledger_coverage(&map_graph).expect("real Map graph covers independent ledger");

    let map_prefix = map::graph_prefix(&super::super::MAP_LOOP_PROFILES[0]);
    let map_leaf = format!("{map_prefix}.ordinal-001.target.projection.terminal.success");
    assert!(map_graph.source_leaf_universe.remove(&map_leaf));
    assert!(validate_graph_ledger_coverage(&map_graph).is_err());
    map_graph.source_leaf_universe.insert(map_leaf);

    let boundary = format!("{map_prefix}.excluded.target-after-ordinal-256");
    let mut unauthorized_boundary = map_graph
        .nodes
        .iter()
        .find(|node| node.id == boundary)
        .expect("reviewed profile has an exact terminal boundary")
        .clone();
    unauthorized_boundary.id = format!("{map_prefix}.excluded.target-after-ordinal-001");
    map_graph.nodes.push(unauthorized_boundary);
    assert!(validate_graph_ledger_coverage(&map_graph).is_err());
    map_graph.nodes.pop();

    let grow = super::super::MAP_LOOP_PROFILES
        .iter()
        .find(|profile| profile.file_grow_count == 1)
        .expect("authority has a grow profile");
    let grow_leaf = format!(
        "{}.ordinal-001.target.projection.terminal.success",
        map::graph_prefix(grow)
    );
    let grow_node = map_graph
        .nodes
        .iter_mut()
        .find(|node| node.id == grow_leaf)
        .expect("grow success leaf exists");
    let graph::NodeKind::Terminal { expected, .. } = &mut grow_node.kind else {
        panic!("grow success leaf is terminal")
    };
    expected.counts.file_grow = 0;
    assert!(validate_graph_ledger_coverage(&map_graph).is_err());
    drop(map_graph);

    let mut lock_graph = contract_lock::graph();
    validate_graph_ledger_coverage(&lock_graph)
        .expect("real Lock graph covers independent range ledger");
    let range = &super::super::LOCK_RANGES[0];
    let valid = format!("{}.valid", lock::graph_prefix(range));
    let position = lock_graph
        .nodes
        .iter()
        .position(|node| node.id == valid)
        .expect("range continuation exists");
    let removed = lock_graph.nodes.remove(position);
    assert!(validate_graph_ledger_coverage(&lock_graph).is_err());
    lock_graph.nodes.insert(position, removed);

    let unauthorized = lock_graph
        .nodes
        .iter()
        .find(|node| node.id == valid)
        .expect("restored range continuation")
        .clone();
    let mut unauthorized = unauthorized;
    unauthorized.id = "lock.request.lock-shared.first0.count1.mask03.valid".to_owned();
    lock_graph.nodes.push(unauthorized);
    assert!(validate_graph_ledger_coverage(&lock_graph).is_err());
}
