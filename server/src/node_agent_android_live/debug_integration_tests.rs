use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    coordinator
        .register_candidate(
            path(source),
            "project-a",
            "physical-device-a",
            "com.elon.app.uituner_stable-node",
            Some(request),
            "compat-session",
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
fn merge_conflict_fails_closed_and_preserves_last_usable_artifact() {
    let repo = RepositoryFixture::new("conflict");
    let (one_root, one) = repo.session("one", "shared.txt", "from one\n");
    let (two_root, two) = repo.session("two", "shared.txt", "from two\n");
    let coordinator = coordinator("conflict");
    let first = register(&coordinator, &one_root, &candidate(&repo.base, &one, "one"));
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

    let second = register(&coordinator, &two_root, &candidate(&repo.base, &two, "two"));
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
        )
        .expect_err("dirty candidate must fail");
    assert!(error.to_string().contains("DEBUG_CANDIDATE_DIRTY"));
}

#[test]
fn signer_drift_preserves_the_pinned_artifact_and_fails_closed() {
    let repo = RepositoryFixture::new("signer-drift");
    let (source, commit) = repo.session("one", "one.txt", "one\n");
    let coordinator = coordinator("signer-drift");
    let plan = register(
        &coordinator,
        &source,
        &candidate(&repo.base, &commit, "one"),
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
    assert_eq!(status.status, "SIGNATURE_MISMATCH");
    assert_eq!(status.last_usable.unwrap().sha256, "usable");
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
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
    let output = Command::new("git")
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
