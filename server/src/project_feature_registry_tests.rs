use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Barrier},
    thread,
};

use crate::{
    node_agent_project_docs_mcp::McpRequest,
    node_agent_project_feature_mcp,
    project_feature_projection::context_projection,
    project_feature_registry::{ProjectFeaturePriority, ProjectFeatureStatus},
    project_feature_registry_service::{
        check_drift, claim_feature, feature_history, list_features, plan_feature, record_evidence,
        register_feature, transition_feature, RegisterFeatureRequest,
    },
    project_feature_registry_store::{load_registry, save_registry, FeatureEvidenceInput},
    project_feature_registry_update::{
        rebind_requirement, update_feature, RebindRequirementRequest, UpdateFeatureRequest,
    },
};

#[test]
fn feature_mcp_list_accepts_missing_payload_and_returns_standard_result() {
    let workspace = TestWorkspace::new("mcp-list");
    workspace.write("docs/feature.md", &current_requirement("MCP list"));
    register(
        workspace.path(),
        "mcp-list",
        "docs/feature.md",
        vec![],
        None,
    );
    let request: McpRequest = serde_json::from_value(serde_json::json!({
        "jsonrpc":"2.0",
        "id":1,
        "method":"tools/call",
        "params":{
            "name":"project_feature_workflow",
            "arguments":{"action":"list"}
        }
    }))
    .unwrap();
    let response = node_agent_project_feature_mcp::handle_request(workspace.path(), &request)
        .expect("list without payload should use an empty object");
    assert_eq!(response["isError"], false);
    assert!(response["content"]
        .as_array()
        .is_some_and(|items| items.len() == 1));
    assert_eq!(response["structuredContent"]["total"], 1);
    assert_eq!(
        response["structuredContent"]["features"][0]["id"],
        "mcp-list"
    );
    assert!(response["structuredContent"]["response_budget"].is_object());
}

struct TestWorkspace(PathBuf);

impl TestWorkspace {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "elon_project_feature_{label}_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&path).unwrap();
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&path)
            .status()
            .unwrap();
        assert!(status.success());
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn current_requirement(title: &str) -> String {
    format!("---\nversion_status: current\n---\n# {title}\n\n## 验收标准\n\n- 可验证。\n")
}

fn register(
    workspace: &Path,
    id: &str,
    requirement_path: &str,
    dependencies: Vec<String>,
    expected_registry_revision: Option<String>,
) -> serde_json::Value {
    register_feature(
        workspace,
        RegisterFeatureRequest {
            id: id.to_string(),
            title: format!("功能 {id}"),
            summary: format!("实现 {id} 并保持低 token 导航。"),
            status: ProjectFeatureStatus::Proposed,
            priority: ProjectFeaturePriority::P1,
            requirement_path: requirement_path.to_string(),
            knowledge_node_id: String::new(),
            owner: "codex".to_string(),
            tags: vec!["agent-memory".to_string()],
            task_paths: vec!["src/feature.rs".to_string()],
            dependencies,
            acceptance_criteria: vec!["生命周期可验证".to_string()],
            actor: "codex-test".to_string(),
            reason: "integration coverage".to_string(),
            expected_registry_revision,
        },
    )
    .unwrap()
}

fn revision(value: &serde_json::Value) -> String {
    value["registry_revision"].as_str().unwrap().to_string()
}

fn transition(
    workspace: &Path,
    id: &str,
    to: ProjectFeatureStatus,
    claim_id: &str,
    expected_revision: &str,
) -> serde_json::Value {
    transition_feature(
        workspace,
        id,
        to,
        "codex-test",
        "lifecycle test",
        claim_id,
        Some(expected_revision),
    )
    .unwrap()
}

#[test]
fn feature_lifecycle_refreshes_evidence_and_keeps_context_body_free() {
    let workspace = TestWorkspace::new("lifecycle");
    workspace.write("docs/feature.md", &current_requirement("Agent memory"));
    workspace.write("src/feature.rs", "pub fn enabled() -> bool { true }\n");
    workspace.write("tests/feature.rs", "#[test]\nfn feature_works() {}\n");

    let registered = register(
        workspace.path(),
        "agent-memory",
        "docs/feature.md",
        vec![],
        None,
    );
    assert_eq!(registered["source_bodies_stored"], 0);
    let accepted = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Accepted,
        "",
        &revision(&registered),
    );
    let ready = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Ready,
        "",
        &revision(&accepted),
    );

    let projection = context_projection(
        workspace.path(),
        "agent memory feature",
        &["src/feature.rs".to_string()],
    );
    assert_eq!(projection["selected_count"], 1);
    assert_eq!(projection["selected"][0]["id"], "agent-memory");
    assert_eq!(projection["source_bodies_returned"], 0);
    assert!(projection.to_string().len() < 5_000);

    let claimed = claim_feature(
        workspace.path(),
        "agent-memory",
        "codex-test-agent",
        5,
        Some(&revision(&ready)),
    )
    .unwrap();
    let claim_id = claimed["claim"]["claim_id"].as_str().unwrap();
    let in_progress = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::InProgress,
        claim_id,
        &revision(&claimed),
    );
    let recorded = record_evidence(
        workspace.path(),
        "agent-memory",
        claim_id,
        "codex-test",
        vec![
            FeatureEvidenceInput {
                path: "src/feature.rs".to_string(),
                locator: "enabled".to_string(),
                evidence_kind: "source".to_string(),
            },
            FeatureEvidenceInput {
                path: "tests/feature.rs".to_string(),
                locator: "feature_works".to_string(),
                evidence_kind: "test".to_string(),
            },
        ],
        Some(&revision(&in_progress)),
    )
    .unwrap();
    let implemented = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Implemented,
        claim_id,
        &revision(&recorded),
    );

    workspace.write(
        "tests/feature.rs",
        "#[test]\nfn feature_works() { assert!(true); }\n",
    );
    assert!(transition_feature(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Verified,
        "codex-test",
        "stale evidence must fail",
        "",
        Some(&revision(&implemented)),
    )
    .is_err());
    let refreshed = record_evidence(
        workspace.path(),
        "agent-memory",
        "",
        "codex-test",
        vec![FeatureEvidenceInput {
            path: "tests/feature.rs".to_string(),
            locator: "feature_works".to_string(),
            evidence_kind: "test".to_string(),
        }],
        Some(&revision(&implemented)),
    )
    .unwrap();
    assert_eq!(
        refreshed["feature"]["implementation_evidence"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let verified = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Verified,
        "",
        &revision(&refreshed),
    );
    let released = transition(
        workspace.path(),
        "agent-memory",
        ProjectFeatureStatus::Released,
        "",
        &revision(&verified),
    );
    assert_eq!(released["status"], "released");

    let plan = plan_feature(workspace.path(), "agent-memory").unwrap();
    assert_eq!(plan["source_policy"]["requirement_body_returned"], false);
    let history = feature_history(workspace.path(), Some("agent-memory"), 0, 100).unwrap();
    assert!(history["total"].as_u64().unwrap() >= 8);
    assert_eq!(history["source_bodies_returned"], 0);
}

#[test]
fn drift_rebind_revision_dependency_and_expired_claim_fail_closed() {
    let workspace = Arc::new(TestWorkspace::new("guards"));
    workspace.write("docs/dependency.md", &current_requirement("Dependency"));
    workspace.write("docs/child.md", &current_requirement("Child"));

    let dependency = register(
        workspace.path(),
        "dependency",
        "docs/dependency.md",
        vec![],
        None,
    );
    let child = register(
        workspace.path(),
        "child",
        "docs/child.md",
        vec!["dependency".to_string()],
        Some(revision(&dependency)),
    );
    let accepted = transition(
        workspace.path(),
        "child",
        ProjectFeatureStatus::Accepted,
        "",
        &revision(&child),
    );
    assert!(transition_feature(
        workspace.path(),
        "child",
        ProjectFeatureStatus::Ready,
        "codex-test",
        "dependency remains incomplete",
        "",
        Some(&revision(&accepted)),
    )
    .is_err());

    let updated = update_feature(
        workspace.path(),
        UpdateFeatureRequest {
            feature_id: "child".to_string(),
            title: Some("Child updated".to_string()),
            summary: None,
            priority: None,
            knowledge_node_id: None,
            owner: None,
            tags: None,
            task_paths: None,
            dependencies: None,
            acceptance_criteria: None,
            actor: "codex-test".to_string(),
            reason: "metadata update".to_string(),
            expected_registry_revision: Some(revision(&accepted)),
        },
    )
    .unwrap();
    assert!(update_feature(
        workspace.path(),
        UpdateFeatureRequest {
            feature_id: "child".to_string(),
            title: Some("stale writer".to_string()),
            summary: None,
            priority: None,
            knowledge_node_id: None,
            owner: None,
            tags: None,
            task_paths: None,
            dependencies: None,
            acceptance_criteria: None,
            actor: "codex-test".to_string(),
            reason: String::new(),
            expected_registry_revision: Some(revision(&accepted)),
        },
    )
    .is_err());

    workspace.write(
        "docs/child.md",
        &format!("{}\n范围已经改变。\n", current_requirement("Child")),
    );
    let drift = check_drift(workspace.path(), Some("child")).unwrap();
    assert_eq!(drift["drifted"], 1);
    let rebound = rebind_requirement(
        workspace.path(),
        RebindRequirementRequest {
            feature_id: "child".to_string(),
            requirement_path: String::new(),
            actor: "codex-test".to_string(),
            reason: "accept changed scope".to_string(),
            expected_registry_revision: Some(revision(&updated)),
        },
    )
    .unwrap();
    assert_eq!(rebound["feature"]["status"], "proposed");
    assert_eq!(rebound["review_required"], true);

    let list = list_features(workspace.path(), &[], "", 0, 1).unwrap();
    assert_eq!(list["returned"], 1);
    assert_eq!(list["total"], 2);

    let dependency_accepted = transition(
        workspace.path(),
        "dependency",
        ProjectFeatureStatus::Accepted,
        "",
        &revision(&rebound),
    );
    let dependency_ready = transition(
        workspace.path(),
        "dependency",
        ProjectFeatureStatus::Ready,
        "",
        &revision(&dependency_accepted),
    );
    let first_claim = claim_feature(
        workspace.path(),
        "dependency",
        "first-agent",
        5,
        Some(&revision(&dependency_ready)),
    )
    .unwrap();
    let mut loaded = load_registry(workspace.path()).unwrap();
    let claim = loaded
        .registry
        .features
        .iter_mut()
        .find(|feature| feature.id == "dependency")
        .and_then(|feature| feature.claim.as_mut())
        .unwrap();
    claim.claimed_at_ms = 1;
    claim.expires_at_ms = 2;
    let expired = save_registry(
        workspace.path(),
        loaded.registry,
        loaded.revision.as_deref(),
    )
    .unwrap();
    let reclaimed = claim_feature(
        workspace.path(),
        "dependency",
        "second-agent",
        5,
        expired.revision.as_deref(),
    )
    .unwrap();
    assert_ne!(
        reclaimed["claim"]["claim_id"],
        first_claim["claim"]["claim_id"]
    );
    assert_eq!(reclaimed["claim"]["agent_id"], "second-agent");

    let concurrent_revision = revision(&reclaimed);
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for suffix in ["a", "b"] {
        let workspace = Arc::clone(&workspace);
        let barrier = Arc::clone(&barrier);
        let concurrent_revision = concurrent_revision.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            update_feature(
                workspace.path(),
                UpdateFeatureRequest {
                    feature_id: "child".to_string(),
                    title: Some(format!("concurrent-{suffix}")),
                    summary: None,
                    priority: None,
                    knowledge_node_id: None,
                    owner: None,
                    tags: None,
                    task_paths: None,
                    dependencies: None,
                    acceptance_criteria: None,
                    actor: format!("writer-{suffix}"),
                    reason: "concurrency test".to_string(),
                    expected_registry_revision: Some(concurrent_revision),
                },
            )
        }));
    }
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
}
