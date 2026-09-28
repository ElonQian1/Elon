use super::*;
use serde_json::json;

struct Repo(PathBuf);
impl Repo {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-registry-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--quiet"]).unwrap();
        git(
            &root,
            &[
                "remote",
                "add",
                "origin",
                "https://example.invalid/team/quant.git",
            ],
        )
        .unwrap();
        std::fs::write(root.join("source.txt"), "source\n").unwrap();
        git(&root, &["add", "source.txt"]).unwrap();
        git(
            &root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        )
        .unwrap();
        Self(root)
    }
    fn payload(&self) -> Value {
        json!({"projects":[{"id":"project-1","node_id":"node-1","workspace_path":self.0,
            "repo_url":"git@example.invalid:team/quant.git","status":"active"}]})
    }
    fn identity(&self) -> RegisteredProjectIdentity {
        select(
            &self.0,
            "node-1",
            "owner-1",
            "https://registry.invalid",
            &self.payload(),
        )
        .unwrap()
    }
}
impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn accepts_only_registered_checkout_or_real_linked_worktree() {
    let repo = Repo::new();
    let identity = repo.identity();
    let linked = repo.0.join("linked");
    git(
        &repo.0,
        &["worktree", "add", "--detach", linked.to_str().unwrap()],
    )
    .unwrap();
    let actual = select(
        &linked,
        "node-1",
        "owner-1",
        "https://registry.invalid",
        &repo.payload(),
    )
    .unwrap();
    assert_eq!(actual, identity);
    assert!(actual
        .scope_id()
        .unwrap()
        .starts_with("registered-project-workspace:"));
    let clone = Repo::new();
    assert!(select(
        &clone.0,
        "node-1",
        "owner-1",
        "https://registry.invalid",
        &repo.payload()
    )
    .is_err());
    assert!(select(
        &repo.0.join(".git"),
        "node-1",
        "owner-1",
        "https://registry.invalid",
        &repo.payload()
    )
    .is_err());
}

#[test]
fn rejects_wrong_node_deleted_unbound_duplicate_and_changed_remote() {
    let repo = Repo::new();
    for (field, value) in [
        ("node_id", json!("other-node")),
        ("status", json!("deleted")),
        ("workspace_path", Value::Null),
        ("repo_url", json!("https://evil.invalid/repo")),
        ("id", json!("../../fake")),
    ] {
        let mut payload = repo.payload();
        payload["projects"][0][field] = value;
        assert!(
            select(
                &repo.0,
                "node-1",
                "owner-1",
                "https://registry.invalid",
                &payload
            )
            .is_err(),
            "{field}"
        );
    }
    let mut payload = repo.payload();
    let project = payload["projects"][0].clone();
    payload["projects"].as_array_mut().unwrap().push(project);
    assert!(select(
        &repo.0,
        "node-1",
        "owner-1",
        "https://registry.invalid",
        &payload
    )
    .unwrap_err()
    .to_string()
    .contains("AMBIGUOUS_ROOT"));
    assert!(select(
        &repo.0,
        "node-1",
        "owner-1",
        "https://registry.invalid",
        &json!({"projects":[]})
    )
    .is_err());
    let original = repo.identity();
    let mut changed = original.clone();
    changed.owner_id = "another-owner".into();
    assert_ne!(original.scope_id().unwrap(), changed.scope_id().unwrap());
}

fn view() -> Value {
    json!({"connected":true,"historyCount":0,"redoCount":0,"nodeCount":23,"runtimeBuildId":"build-3",
        "sourceProof":{"runtimeBuildId":"build-3","originWorkspaceRevision":"source-1","generation":3,"sourceParityLoss":0.0}})
}

#[test]
fn verified_runtime_can_restore_only_same_registered_identity_and_build() {
    let repo = Repo::new();
    let identity = repo.identity();
    let first = bind_runtime(identity.clone(), "source-1", &view(), None).unwrap();
    let mut restored = view();
    restored["sourceProof"] = Value::Null;
    assert_eq!(
        bind_runtime(identity.clone(), "source-1", &restored, Some(&first)).unwrap(),
        first
    );
    assert!(bind_runtime(identity.clone(), "source-1", &restored, None).is_err());
    restored["runtimeBuildId"] = json!("other-build");
    assert!(bind_runtime(identity.clone(), "source-1", &restored, Some(&first)).is_err());
    let mut drift = identity.clone();
    drift.project_id = "other-project".into();
    restored["runtimeBuildId"] = json!("build-3");
    assert!(bind_runtime(drift, "source-1", &restored, Some(&first)).is_err());
}

#[test]
fn rejects_unproven_source_patch_history_and_invalid_generation() {
    let repo = Repo::new();
    for pointer in [
        "/connected",
        "/historyCount",
        "/redoCount",
        "/nodeCount",
        "/runtimeBuildId",
        "/sourceProof/runtimeBuildId",
        "/sourceProof/originWorkspaceRevision",
        "/sourceProof/generation",
        "/sourceProof/sourceParityLoss",
    ] {
        let mut changed = view();
        *changed.pointer_mut(pointer).unwrap() = match pointer {
            "/connected" => json!(false),
            "/historyCount" | "/redoCount" => json!(1),
            "/nodeCount" | "/sourceProof/generation" => json!(0),
            "/sourceProof/sourceParityLoss" => json!(-1),
            _ => json!("wrong"),
        };
        assert!(
            bind_runtime(repo.identity(), "source-1", &changed, None).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn remote_normalization_preserves_repository_and_host_identity() {
    assert_eq!(
        normalize_remote("git@example.invalid:team/repo.git").unwrap(),
        normalize_remote("https://example.invalid/team/repo.git").unwrap()
    );
    assert_ne!(
        normalize_remote("https://a.invalid/team/repo").unwrap(),
        normalize_remote("https://b.invalid/team/repo").unwrap()
    );
    for value in [
        "../repo",
        "https://user:secret@example.invalid/repo",
        "https://example.invalid/repo?token=secret",
    ] {
        assert!(normalize_remote(value).is_err());
    }
}
