use super::super::live_backend::fresh_runtime_source_candidate;
use super::fixtures::{candidate, cleanup, run};
use crate::node_agent_android_live::protocol::{LiveSessionView, LiveSourceProofView};

fn session(source_revision: &str) -> LiveSessionView {
    LiveSessionView {
        id: "live_test".into(),
        device_id: "device-1".into(),
        package_name: "com.example.test".into(),
        project_root: None,
        device_port: 38917,
        created_at: "2026-07-17T00:00:00Z".into(),
        connected: true,
        runtime_build_id: Some("build-1".into()),
        runtime_version: Some("1.0.0".into()),
        tree_revision: 1,
        node_count: 1,
        history_count: 0,
        redo_count: 0,
        source_proof: Some(LiveSourceProofView {
            source_revision: source_revision.into(),
            runtime_build_id: Some("build-1".into()),
            source_parity_loss: 0.0,
            verified_at: "2026-07-17T00:00:00Z".into(),
        }),
        last_seen_at: None,
        last_error: None,
    }
}

#[test]
fn reuses_matching_patch_free_fresh_runtime_proof() {
    let (root, mut run) = run(false);
    run.best = Some(candidate("best", 0.01));
    let candidate = fresh_runtime_source_candidate(&run, &session("source-1"), Some("source-1"))
        .expect("matching fresh runtime proof should avoid a redundant install");
    assert!(candidate.source_parity_verified);
    assert_eq!(candidate.source_parity_loss, Some(0.0));
    assert_eq!(candidate.source_revision.as_deref(), Some("source-1"));
    cleanup(root);
}

#[test]
fn rejects_stale_source_revision() {
    let (root, mut run) = run(false);
    run.best = Some(candidate("best", 0.01));
    assert!(
        fresh_runtime_source_candidate(&run, &session("source-old"), Some("source-1")).is_none()
    );
    cleanup(root);
}

#[test]
fn rejects_runtime_with_live_operations_or_patch_history() {
    let (root, mut run) = run(false);
    let mut best = candidate("best", 0.01);
    best.operations
        .push(serde_json::json!({"property":"width"}));
    run.best = Some(best);
    assert!(fresh_runtime_source_candidate(&run, &session("source-1"), Some("source-1")).is_none());

    run.best = Some(candidate("best", 0.01));
    let mut patched = session("source-1");
    patched.history_count = 1;
    assert!(fresh_runtime_source_candidate(&run, &patched, Some("source-1")).is_none());
    cleanup(root);
}
