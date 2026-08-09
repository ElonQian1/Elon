use std::fs;
use std::path::{Path, PathBuf};

use super::debug_integration::{DebugIntegrationCoordinator, DebugMergeCandidateRequest};
use super::debug_integration_contract::DebugArtifactStatus;

struct RepositoryFixture {
    root: PathBuf,
    base: String,
}

impl RepositoryFixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon-debug-integration-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("create repository root");
        git(&root, &["init"]);
        git(
            &root,
            &["config", "user.email", "debug-integration@elon.local"],
        );
        git(
            &root,
            &["config", "user.name", "Elon Debug Integration Test"],
        );
        fs::write(root.join("shared.txt"), "base\n").expect("write base file");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "base"]);
        let base = git_output(&root, &["rev-parse", "HEAD"]);
        Self { root, base }
    }

    fn session(&self, name: &str, file: &str, contents: &str) -> (PathBuf, String) {
        let worktree = self.root.with_file_name(format!(
            "{}-{name}",
            self.root.file_name().expect("repo name").to_string_lossy()
        ));
        git(
            &self.root,
            &[
                "worktree",
                "add",
                "-b",
                &format!("session-{name}"),
                path(&worktree),
                &self.base,
            ],
        );
        fs::write(worktree.join(file), contents).expect("write session change");
        git(&worktree, &["add", "."]);
        git(&worktree, &["commit", "-m", &format!("session {name}")]);
        let commit = git_output(&worktree, &["rev-parse", "HEAD"]);
        (worktree, commit)
    }
}

fn candidate(base: &str, commit: &str, session: &str) -> DebugMergeCandidateRequest {
    DebugMergeCandidateRequest {
        ready: true,
        commit_sha: Some(commit.into()),
        base_sha: Some(base.into()),
        source_session_id: Some(session.into()),
        preview_owner: Some(session.into()),
        ..DebugMergeCandidateRequest::default()
    }
}

fn coordinator(name: &str) -> DebugIntegrationCoordinator {
    DebugIntegrationCoordinator::new(
        std::env::temp_dir().join(format!("elon-debug-slot-{name}-{}", uuid::Uuid::new_v4())),
        "stable-node".into(),
    )
}

fn register(
    coordinator: &DebugIntegrationCoordinator,
    source: &Path,
    request: &DebugMergeCandidateRequest,
) -> super::debug_integration::DebugIntegrationPlan {
    register_with_lkg(coordinator, source, request, None)
}

fn register_with_lkg(
    coordinator: &DebugIntegrationCoordinator,
    source: &Path,
    request: &DebugMergeCandidateRequest,
    lkg_enabled: Option<bool>,
) -> super::debug_integration::DebugIntegrationPlan {
    coordinator
        .register_candidate(
            path(source),
            "project-a",
            "physical-device-a",
            "com.elon.app.uituner_stable-node",
            Some(request),
            "compat-session",
            lkg_enabled,
        )
        .expect("register ready candidate")
}

#[test]
fn three_sessions_merge_committed_candidates_in_audited_order() {
    let repo = RepositoryFixture::new("three-sessions");
    let (one_root, one) = repo.session("one", "one.txt", "one\n");
    let (two_root, two) = repo.session("two", "two.txt", "two\n");
    let (three_root, three) = repo.session("three", "three.txt", "three\n");
    let coordinator = coordinator("three-sessions");

    let first = register(&coordinator, &one_root, &candidate(&repo.base, &one, "one"));
    assert_eq!(first.generation, 1);
    let second = register(&coordinator, &two_root, &candidate(&repo.base, &two, "two"));
    assert_eq!(second.generation, 2);
    let third = register(
        &coordinator,
        &three_root,
        &candidate(&repo.base, &three, "three"),
    );
    assert_eq!(third.generation, 3);

    let integrated = coordinator.materialize(&third).expect("merge all commits");
    assert_eq!(
        fs::read_to_string(integrated.join("one.txt"))
            .unwrap()
            .trim(),
        "one"
    );
    assert_eq!(
        fs::read_to_string(integrated.join("two.txt"))
            .unwrap()
            .trim(),
        "two"
    );
    assert_eq!(
        fs::read_to_string(integrated.join("three.txt"))
            .unwrap()
            .trim(),
        "three"
    );
    let status = coordinator.status(&third.slot_id).unwrap().unwrap();
    assert_eq!(status.status, "MERGED");
    assert_eq!(
        status
            .contributions
            .iter()
            .map(|item| item.commit_sha.as_str())
            .collect::<Vec<_>>(),
        vec![one, two, three]
    );
}

#[test]
fn commits_already_contained_by_an_advanced_base_are_not_cherry_picked_again() {
    let repo = RepositoryFixture::new("advanced-base");
    let (source, first) = repo.session("source", "one.txt", "one\n");
    fs::write(source.join("two.txt"), "two\n").unwrap();
    git(&source, &["add", "."]);
    git(&source, &["commit", "-m", "source two"]);
    let second = git_output(&source, &["rev-parse", "HEAD"]);
    fs::write(source.join("three.txt"), "three\n").unwrap();
    git(&source, &["add", "."]);
    git(&source, &["commit", "-m", "source three"]);
    let third = git_output(&source, &["rev-parse", "HEAD"]);
    let coordinator = coordinator("advanced-base");
    let mut original = candidate(&repo.base, &third, "source");
    original.commits = Some(vec![first.clone(), second.clone(), third.clone()]);
    let first_plan = register(&coordinator, &source, &original);
    assert_eq!(first_plan.contributions, vec![first, second, third.clone()]);

    git(&repo.root, &["merge", "--ff-only", &third]);
    fs::write(repo.root.join("base-main.txt"), "new base\n").unwrap();
    git(&repo.root, &["add", "."]);
    git(&repo.root, &["commit", "-m", "advance base main"]);
    let advanced_base = git_output(&repo.root, &["rev-parse", "HEAD"]);
    let mut already_integrated = candidate(&advanced_base, &third, "source");
    already_integrated.commits = original.commits.clone();

    let normalized = register(&coordinator, &source, &already_integrated);
    assert_eq!(normalized.generation, first_plan.generation + 1);
    assert_eq!(normalized.base_sha, advanced_base);
    assert!(normalized.contributions.is_empty());
    let integrated = coordinator
        .materialize(&normalized)
        .expect("advanced base should materialize without repeat cherry-picks");
    assert_eq!(
        git_output(&integrated, &["rev-parse", "HEAD"]),
        advanced_base
    );
    let status = coordinator.status(&normalized.slot_id).unwrap().unwrap();
    assert!(status.contributions.is_empty());
    assert_eq!(
        status.integration_revision.as_deref(),
        Some(advanced_base.as_str())
    );
    assert_eq!(status.source_revision.as_deref(), Some(third.as_str()));
}

#[test]
fn explicit_empty_commits_clear_the_accumulated_sequence() {
    let repo = RepositoryFixture::new("explicit-empty");
    let (source, commit) = repo.session("source", "one.txt", "one\n");
    let coordinator = coordinator("explicit-empty");
    let first = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "source"),
    );
    assert_eq!(first.contributions, vec![commit.clone()]);

    let empty = DebugMergeCandidateRequest {
        ready: true,
        commit_sha: Some(commit.clone()),
        commits: Some(Vec::new()),
        base_sha: Some(repo.base.clone()),
        source_session_id: Some("source".into()),
        preview_owner: Some("source".into()),
        ..DebugMergeCandidateRequest::default()
    };
    let cleared = register(&coordinator, &source, &empty);
    assert_eq!(cleared.generation, first.generation + 1);
    assert!(cleared.contributions.is_empty());
    let integrated = coordinator.materialize(&cleared).unwrap();
    assert!(!integrated.join("one.txt").exists());
    assert_eq!(git_output(&integrated, &["rev-parse", "HEAD"]), repo.base);
}

#[test]
fn failed_generation_restart_reuses_the_clean_owned_worktree() {
    let repo = RepositoryFixture::new("failed-restart");
    let (source, commit) = repo.session("source", "one.txt", "one\n");
    let coordinator = coordinator("failed-restart");
    let failed = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "source"),
    );
    let integrated = coordinator
        .materialize(&failed)
        .expect("failed generation must first be owned and materialized");
    fs::create_dir_all(integrated.join("android/app/build")).unwrap();
    fs::write(
        integrated.join("android/app/build/partial-output.bin"),
        b"preserve incremental output",
    )
    .unwrap();
    coordinator
        .record_runtime_failure(&failed, "simulated failed operation".into())
        .unwrap();

    let restarted = coordinator.restart_failed_generation(&failed).unwrap();
    assert_eq!(restarted.generation, failed.generation);
    assert_eq!(restarted.worktree, failed.worktree);
    let integrated = coordinator
        .materialize(&restarted)
        .expect("restart must reuse the verified generation worktree");
    assert!(integrated.join("one.txt").exists());
    assert!(integrated
        .join("android/app/build/partial-output.bin")
        .exists());
    let status = coordinator.status(&restarted.slot_id).unwrap().unwrap();
    assert_eq!(status.status, "MERGED");
    assert_eq!(status.desired_generation, restarted.generation);
}

#[test]
fn runtime_reconnect_restores_the_same_installed_generation_without_reinstalling() {
    let repo = RepositoryFixture::new("runtime-reconnect");
    let (source, commit) = repo.session("source", "one.txt", "one\n");
    let coordinator = coordinator("runtime-reconnect");
    let plan = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "source"),
    );
    coordinator.materialize(&plan).unwrap();
    coordinator.mark_building(&plan).unwrap();
    coordinator
        .record_artifact(
            &plan,
            DebugArtifactStatus {
                apk_path: "artifacts/reused.apk".into(),
                sha256: "reused-sha".into(),
                package_name: plan.package_name.clone(),
                version_code: "1".into(),
                version_name: "test".into(),
                app_label: "一龙调试 stable-node".into(),
                signer_sha256: "signer-a".into(),
                generation: plan.generation,
            },
        )
        .unwrap();
    coordinator.record_deployed(&plan).unwrap();
    coordinator
        .record_runtime_failure(&plan, "simulated reconnect interruption".into())
        .unwrap();

    let restarted = coordinator.restart_failed_generation(&plan).unwrap();
    assert_eq!(restarted.generation, plan.generation);
    coordinator.confirm_reused_deployment(&restarted).unwrap();

    let status = coordinator.status(&plan.slot_id).unwrap().unwrap();
    assert_eq!(status.status, "DEPLOYED");
    assert_eq!(status.installed_generation, Some(plan.generation));
    assert!(status.last_error.is_none());
}

#[test]
fn runtime_reconnect_cannot_claim_a_generation_without_install_evidence() {
    let repo = RepositoryFixture::new("reconnect-no-install");
    let (source, commit) = repo.session("source", "one.txt", "one\n");
    let coordinator = coordinator("reconnect-no-install");
    let plan = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "source"),
    );
    coordinator.materialize(&plan).unwrap();

    let error = coordinator
        .confirm_reused_deployment(&plan)
        .expect_err("never-installed generation must fail closed");
    assert!(error.to_string().contains("DEBUG_REUSE_NOT_DEPLOYED"));
    let status = coordinator.status(&plan.slot_id).unwrap().unwrap();
    assert_eq!(status.status, "MERGED");
    assert_eq!(status.installed_generation, None);
}

#[test]
fn unowned_failed_generation_still_allocates_a_fresh_worktree() {
    let repo = RepositoryFixture::new("unowned-failed-restart");
    let (source, commit) = repo.session("source", "one.txt", "one\n");
    let coordinator = coordinator("unowned-failed-restart");
    let failed = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "source"),
    );
    fs::create_dir_all(&failed.worktree).unwrap();
    coordinator
        .record_runtime_failure(&failed, "failed before materialization".into())
        .unwrap();

    let restarted = coordinator.restart_failed_generation(&failed).unwrap();
    assert!(restarted.generation > failed.generation);
    assert_ne!(restarted.worktree, failed.worktree);
    coordinator
        .materialize(&restarted)
        .expect("unowned worktree must never be resumed");
}

#[test]
fn unicode_node_data_root_survives_status_json_and_worktree_materialization() {
    let repo = RepositoryFixture::new("unicode-source-一龙");
    let (source, commit) = repo.session("unicode", "unicode.txt", "一龙\n");
    let integration_root = std::env::temp_dir()
        .join(format!("一龙-node-data-{}", uuid::Uuid::new_v4()))
        .join("ElonNodeData")
        .join("android-debug-integration");
    let coordinator =
        DebugIntegrationCoordinator::new(integration_root.clone(), "stable-node".into());
    let plan = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "unicode"),
    );
    assert!(plan.worktree.to_string_lossy().contains("一龙"));
    assert!(!plan.worktree.to_string_lossy().contains("u4E00u9F99"));

    let status_path = integration_root.join(&plan.slot_id).join("status.json");
    let status: super::debug_integration::DebugIntegrationStatus =
        serde_json::from_slice(&fs::read(&status_path).unwrap()).unwrap();
    let restored =
        super::debug_integration_contract::plan_from_status(&integration_root, &source, &status);
    assert_eq!(restored.worktree, plan.worktree);
    assert!(restored.worktree.to_string_lossy().contains("一龙"));
    let materialized = coordinator
        .materialize(&restored)
        .expect("native Unicode worktree path must remain usable");
    assert_eq!(
        fs::read_to_string(materialized.join("unicode.txt"))
            .unwrap()
            .trim(),
        "一龙"
    );
}

#[test]
fn merge_conflict_fails_closed_and_preserves_last_usable_artifact() {
    let repo = RepositoryFixture::new("conflict");
    let (one_root, one) = repo.session("one", "shared.txt", "from one\n");
    let (two_root, two) = repo.session("two", "shared.txt", "from two\n");
    let coordinator = coordinator("conflict");
    let first = register_with_lkg(
        &coordinator,
        &one_root,
        &candidate(&repo.base, &one, "one"),
        Some(true),
    );
    coordinator.materialize(&first).expect("first merge");
    coordinator.mark_building(&first).unwrap();
    coordinator
        .record_artifact(
            &first,
            DebugArtifactStatus {
                apk_path: "artifacts/usable.apk".into(),
                sha256: "usable-sha".into(),
                package_name: first.package_name.clone(),
                version_code: "1".into(),
                version_name: "test".into(),
                app_label: "一龙调试 stable-node".into(),
                signer_sha256: "signer-a".into(),
                generation: first.generation,
            },
        )
        .unwrap();

    let second = register_with_lkg(
        &coordinator,
        &two_root,
        &candidate(&repo.base, &two, "two"),
        Some(true),
    );
    let error = coordinator
        .materialize(&second)
        .expect_err("conflict must fail");
    assert!(error.to_string().contains("DEBUG_MERGE_CONFLICT"));
    let status = coordinator.status(&second.slot_id).unwrap().unwrap();
    assert_eq!(status.status, "MERGE_CONFLICT");
    assert_eq!(status.conflicts, vec![two]);
    assert_eq!(status.last_usable.unwrap().sha256, "usable-sha");
    assert_eq!(status.installed_generation, None);
}

#[test]
fn newer_generation_fences_queued_and_installing_older_generation() {
    let repo = RepositoryFixture::new("fencing");
    let (one_root, one) = repo.session("one", "one.txt", "one\n");
    let (two_root, two) = repo.session("two", "two.txt", "two\n");
    let coordinator = coordinator("fencing");
    let first = register(&coordinator, &one_root, &candidate(&repo.base, &one, "one"));
    let second = register(&coordinator, &two_root, &candidate(&repo.base, &two, "two"));
    assert_eq!(second.generation, first.generation + 1);

    let merge_error = coordinator
        .materialize(&first)
        .expect_err("old build is stale");
    assert!(merge_error
        .to_string()
        .contains("DEBUG_GENERATION_SUPERSEDED"));
    let install_error = coordinator
        .authorize_install(&first)
        .expect_err("old install is stale");
    assert!(install_error
        .to_string()
        .contains("DEBUG_GENERATION_SUPERSEDED"));
    coordinator
        .materialize(&second)
        .expect("new generation remains valid");
}

#[test]
fn non_ready_and_dirty_candidates_are_rejected_without_a_generation() {
    let repo = RepositoryFixture::new("candidate-gates");
    let (source, commit) = repo.session("one", "one.txt", "one\n");
    let coordinator = coordinator("candidate-gates");
    let mut not_ready = candidate(&repo.base, &commit, "one");
    not_ready.ready = false;
    let error = coordinator
        .register_candidate(
            path(&source),
            "project-a",
            "physical-device-a",
            "com.elon.app.uituner_stable-node",
            Some(&not_ready),
            "compat-session",
            None,
        )
        .expect_err("non-ready candidate must fail");
    assert!(error.to_string().contains("DEBUG_CANDIDATE_NOT_READY"));

    fs::write(source.join("dirty.txt"), "not committed\n").unwrap();
    let error = coordinator
        .register_candidate(
            path(&source),
            "project-a",
            "physical-device-a",
            "com.elon.app.uituner_stable-node",
            Some(&candidate(&repo.base, &commit, "one")),
            "compat-session",
            None,
        )
        .expect_err("dirty candidate must fail");
    assert!(error.to_string().contains("DEBUG_CANDIDATE_DIRTY"));
}

#[test]
fn signer_drift_preserves_the_pinned_artifact_and_fails_closed() {
    let repo = RepositoryFixture::new("signer-drift");
    let (source, commit) = repo.session("one", "one.txt", "one\n");
    let coordinator = coordinator("signer-drift");
    let plan = register_with_lkg(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "one"),
        Some(true),
    );
    coordinator.materialize(&plan).unwrap();
    coordinator.mark_building(&plan).unwrap();
    let artifact = |signer: &str, sha: &str| DebugArtifactStatus {
        apk_path: format!("artifacts/{sha}.apk"),
        sha256: sha.into(),
        package_name: plan.package_name.clone(),
        version_code: "1".into(),
        version_name: "test".into(),
        app_label: "一龙调试 stable-node".into(),
        signer_sha256: signer.into(),
        generation: plan.generation,
    };
    coordinator
        .record_artifact(&plan, artifact("signer-a", "usable"))
        .unwrap();
    let error = coordinator
        .record_artifact(&plan, artifact("signer-b", "rejected"))
        .expect_err("signer drift must fail");
    assert!(error.to_string().contains("DEBUG_APK_SIGNATURE_MISMATCH"));
    let status = coordinator.status(&plan.slot_id).unwrap().unwrap();
    assert!(status.lkg_enabled);
    assert_eq!(status.status, "SIGNATURE_MISMATCH");
    assert_eq!(status.last_usable.unwrap().sha256, "usable");
}

#[test]
fn lkg_is_disabled_by_default_and_does_not_gate_install() {
    let repo = RepositoryFixture::new("lkg-disabled");
    let (source, commit) = repo.session("one", "one.txt", "one\n");
    let coordinator = coordinator("lkg-disabled");
    let plan = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "one"),
    );
    let enabled_plan = register_with_lkg(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "one"),
        Some(true),
    );
    assert_eq!(enabled_plan.generation, plan.generation);
    coordinator.materialize(&plan).unwrap();
    coordinator.mark_building(&plan).unwrap();
    let artifact = |signer: &str, sha: &str| DebugArtifactStatus {
        apk_path: format!("artifacts/{sha}.apk"),
        sha256: sha.into(),
        package_name: plan.package_name.clone(),
        version_code: "1".into(),
        version_name: "test".into(),
        app_label: "一龙调试 stable-node".into(),
        signer_sha256: signer.into(),
        generation: plan.generation,
    };
    coordinator
        .record_artifact(&plan, artifact("signer-a", "first"))
        .unwrap();
    coordinator
        .record_artifact(&plan, artifact("signer-b", "second"))
        .expect("default-disabled LKG must not pin or validate the prior signer");
    coordinator
        .authorize_install(&plan)
        .expect("default-disabled LKG must not block ADB installation");
    let status = coordinator.status(&plan.slot_id).unwrap().unwrap();
    assert!(!status.lkg_enabled);
    assert!(status.last_usable.is_none());
    assert_eq!(status.status, "BUILD_READY");
}

fn git(root: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {} failed", args.join(" "));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn path(value: &Path) -> &str {
    value.to_str().expect("utf-8 test path")
}
