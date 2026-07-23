use super::super::live_backend::fresh_runtime_source_candidate;
use super::super::model::{FitCandidate, FitRunDocument};
use super::fixtures::{candidate, cleanup, run};
use crate::node_agent_android_live::debug_integration::DebugIntegrationStatus;
use crate::node_agent_android_live::protocol::{LiveSessionView, LiveSourceProofView};

fn session(generation_revision: &str, origin_workspace_revision: &str) -> LiveSessionView {
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
            generation: 9,
            integration_revision: "integration-9".into(),
            source_revision: "git-source-1".into(),
            generation_revision: generation_revision.into(),
            origin_workspace_revision: origin_workspace_revision.into(),
            runtime_build_id: Some("build-1".into()),
            source_parity_loss: 0.0,
            verified_at: "2026-07-17T00:00:00Z".into(),
        }),
        last_seen_at: None,
        last_error: None,
    }
}

fn integration(package_name: &str) -> DebugIntegrationStatus {
    DebugIntegrationStatus {
        schema: "elon.android_debug_integration.v1".into(),
        slot_id: "slot-1".into(),
        node_fingerprint: "node-1".into(),
        project_id: "project-1".into(),
        device_identity: "device-1".into(),
        package_name: package_name.into(),
        repository_identity: "repo-1".into(),
        base_sha: "base-1".into(),
        source_revision: Some("git-source-1".into()),
        integration_revision: Some("integration-9".into()),
        desired_generation: 9,
        installed_generation: Some(9),
        status: "DEPLOYED".into(),
        lkg_enabled: false,
        integration_worktree: Some("generation-root".into()),
        contributions: Vec::new(),
        conflicts: Vec::new(),
        legacy_packages: Vec::new(),
        preview_owner: None,
        last_error: None,
        last_usable: None,
        updated_at: "2026-07-17T00:00:00Z".into(),
    }
}

fn verified_candidate(
    run: &FitRunDocument,
    session: &LiveSessionView,
    integration: &DebugIntegrationStatus,
) -> Option<FitCandidate> {
    fresh_runtime_source_candidate(
        run,
        session,
        Some("source-1"),
        Some("git-source-1"),
        Some(integration),
        Some("generation-9"),
        Some("integration-9"),
    )
}

#[test]
fn isolated_emulator_package_accepts_matching_patch_free_fresh_runtime_proof() {
    let (root, mut run) = run(false);
    run.best = Some(candidate("best", 0.01));
    let mut live = session("generation-9", "source-1");
    live.package_name = "com.example.test.uitest".into();
    run.package_name = live.package_name.clone();
    let integration = integration(&live.package_name);
    let candidate = verified_candidate(&run, &live, &integration)
        .expect("matching origin proof should avoid a redundant install");
    assert!(candidate.source_parity_verified);
    assert_eq!(candidate.source_parity_loss, Some(0.0));
    assert_eq!(candidate.source_revision.as_deref(), Some("source-1"));
    cleanup(root);
}

#[test]
fn rejects_stale_source_revision() {
    let (root, mut run) = run(false);
    run.best = Some(candidate("best", 0.01));
    assert!(fresh_runtime_source_candidate(
        &run,
        &session("generation-9", "source-old"),
        Some("source-1"),
        Some("git-source-1"),
        Some(&integration("com.example.test")),
        Some("generation-9"),
        Some("integration-9"),
    )
    .is_none());
    cleanup(root);
}

#[test]
fn rejects_runtime_with_live_operations_or_patch_history() {
    let (root, mut run) = run(false);
    let mut best = candidate("best", 0.01);
    best.operations
        .push(serde_json::json!({"property":"width"}));
    run.best = Some(best);
    assert!(fresh_runtime_source_candidate(
        &run,
        &session("generation-9", "source-1"),
        Some("source-1"),
        Some("git-source-1"),
        Some(&integration("com.example.test")),
        Some("generation-9"),
        Some("integration-9"),
    )
    .is_none());

    run.best = Some(candidate("best", 0.01));
    let mut patched = session("generation-9", "source-1");
    patched.history_count = 1;
    assert!(fresh_runtime_source_candidate(
        &run,
        &patched,
        Some("source-1"),
        Some("git-source-1"),
        Some(&integration("com.example.test")),
        Some("generation-9"),
        Some("integration-9"),
    )
    .is_none());
    cleanup(root);
}

#[test]
fn rejects_generation_integration_git_runtime_and_package_identity_drift() {
    let (root, mut run) = run(false);
    run.best = Some(candidate("best", 0.01));
    let live = session("generation-9", "source-1");
    let mut status = integration(&live.package_name);
    for drift in ["generation", "integration", "git", "package", "runtime"] {
        let mut drifted = status.clone();
        let mut drifted_live = live.clone();
        let generation_revision = "generation-9";
        let mut integration_revision = "integration-9";
        let mut origin_git_revision = "git-source-1";
        match drift {
            "generation" => drifted.installed_generation = Some(8),
            "integration" => integration_revision = "integration-8",
            "git" => origin_git_revision = "git-source-2",
            "package" => drifted.package_name = "com.example.other".into(),
            "runtime" => drifted_live.runtime_build_id = Some("build-2".into()),
            _ => unreachable!(),
        }
        assert!(
            fresh_runtime_source_candidate(
                &run,
                &drifted_live,
                Some("source-1"),
                Some(origin_git_revision),
                Some(&drifted),
                Some(generation_revision),
                Some(integration_revision),
            )
            .is_none(),
            "{drift} drift must fail closed"
        );
    }
    status.status = "BUILD_READY".into();
    assert!(verified_candidate(&run, &live, &status).is_none());
    cleanup(root);
}

#[test]
fn source_proof_serializes_generation_and_origin_revisions_unambiguously() {
    let value = serde_json::to_value(
        session("generation-worktree-revision", "origin-workspace-revision")
            .source_proof
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["generationRevision"], "generation-worktree-revision");
    assert_eq!(value["generation"], 9);
    assert_eq!(value["integrationRevision"], "integration-9");
    assert_eq!(value["sourceRevision"], "git-source-1");
    assert_eq!(
        value["originWorkspaceRevision"],
        "origin-workspace-revision"
    );
}
