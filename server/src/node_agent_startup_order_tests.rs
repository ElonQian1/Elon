#[test]
fn orphan_sweep_starts_after_recovery_ownership_barrier() {
    let source = include_str!("node_agent_main.rs");
    let sidecar = source
        .find("node_agent_sidecar_recovery::reconcile_surviving_sidecars(runtime.clone()).await")
        .unwrap();
    let update = source
        .find("node_agent_update_reconcile::reconcile_startup(runtime.clone()).await")
        .unwrap();
    let runtime_online = source.find(".mark_runtime_online_if_target(").unwrap();
    let orphan = source
        .find("runtime.reconcile_local_completion_outbox().await")
        .unwrap();
    let periodic = source
        .find("node_agent_local_task_orphan_reconcile::spawn_reconciler(runtime.clone())")
        .unwrap();
    assert!(sidecar < update);
    assert!(update < runtime_online);
    assert!(runtime_online < orphan);
    assert!(orphan < periodic);
}
