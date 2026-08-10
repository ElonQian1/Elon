use super::{handle_request, TOOL_NAME};
use crate::{
    node_agent_project_docs_mcp::McpRequest,
    project_feature_registry::{ProjectFeaturePriority, ProjectFeatureStatus},
    project_feature_registry_service::{register_feature, RegisterFeatureRequest},
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

struct FeatureFixture(PathBuf);

impl FeatureFixture {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "elon_project_feature_mcp_{label}_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(path.join("docs")).unwrap();
        let status = crate::git_command_error::git_command()
            .args(["init", "--quiet"])
            .current_dir(&path)
            .status()
            .unwrap();
        assert!(status.success());
        Self(path)
    }

    fn request(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let request: McpRequest = serde_json::from_value(json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":method,
            "params":params
        }))?;
        handle_request(&self.0, &request)
    }

    fn register_feature(&self) {
        fs::write(
            self.0.join("docs/feature.md"),
            "---\nversion_status: current\n---\n# Feature\n",
        )
        .unwrap();
        register_feature(
            &self.0,
            RegisterFeatureRequest {
                id: "mcp-list".to_string(),
                title: "MCP list".to_string(),
                summary: "Verify the bounded feature profile.".to_string(),
                status: ProjectFeatureStatus::Proposed,
                priority: ProjectFeaturePriority::P1,
                requirement_path: "docs/feature.md".to_string(),
                knowledge_node_id: String::new(),
                owner: "codex".to_string(),
                tags: vec!["agent-memory".to_string()],
                task_paths: vec!["src/feature.rs".to_string()],
                dependencies: Vec::new(),
                acceptance_criteria: vec!["The profile remains bounded.".to_string()],
                actor: "codex-test".to_string(),
                reason: "profile contract".to_string(),
                expected_registry_revision: None,
            },
        )
        .unwrap();
    }
}

impl Drop for FeatureFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tool_catalog_stays_compact_until_describe_is_explicit() {
    let fixture = FeatureFixture::new("describe");
    let listed = fixture.request("tools/list", json!({})).unwrap();
    let tools = listed["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], TOOL_NAME);
    let compact_catalog = serde_json::to_string(&listed).unwrap();
    assert!(!compact_catalog.contains("project_features_register"));
    assert!(!compact_catalog.contains("expected_registry_revision"));
    assert!(!fixture.0.join(".elon/project-features.json").exists());

    let described = fixture
        .request(
            "tools/call",
            json!({
                "name":TOOL_NAME,
                "arguments":{"action":"describe","payload":{}}
            }),
        )
        .unwrap();
    assert_eq!(described["isError"], false);
    assert_eq!(
        described["structuredContent"]["schema"],
        "elon.project_feature_workflow_description.v1"
    );
    let detailed_catalog = serde_json::to_string(&described).unwrap();
    assert!(detailed_catalog.contains("project_features_register"));
    assert!(detailed_catalog.contains("expected_registry_revision"));
    assert!(!fixture.0.join(".elon/project-features.json").exists());
}

#[test]
fn list_action_accepts_missing_payload_and_direct_tools_are_rejected() {
    let fixture = FeatureFixture::new("list");
    fixture.register_feature();

    let response = fixture
        .request(
            "tools/call",
            json!({"name":TOOL_NAME,"arguments":{"action":"list"}}),
        )
        .expect("list without payload should use an empty object");
    assert_eq!(response["isError"], false);
    assert_eq!(response["structuredContent"]["total"], 1);
    assert_eq!(
        response["structuredContent"]["features"][0]["id"],
        "mcp-list"
    );
    assert!(response["structuredContent"]["response_budget"].is_object());

    let direct = fixture.request(
        "tools/call",
        json!({"name":"project_features_list","arguments":{}}),
    );
    assert!(direct.is_err());
}
