use serde_json::{json, Value};
use std::sync::Arc;
use std::{fs, path::PathBuf};

use crate::node_agent_project_docs_mcp::{
    authorize_session, descriptor_for_project, descriptor_for_vault, handle_request,
    test_transport_routes, McpRequest,
};

fn workspace() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_project_docs_mcp_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("AGENTS.md"), "# Agent entry\n").unwrap();
    assert!(crate::git_command_error::git_command()
        .current_dir(&root)
        .args(["init", "-q"])
        .status()
        .unwrap()
        .success());
    assert!(crate::git_command_error::git_command()
        .current_dir(&root)
        .args(["add", "AGENTS.md"])
        .status()
        .unwrap()
        .success());
    assert!(crate::git_command_error::git_command()
        .current_dir(&root)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            "initial"
        ])
        .status()
        .unwrap()
        .success());
    root
}

fn request(value: Value) -> McpRequest {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn descriptor_binds_short_lived_session_to_git_workspace() {
    let root = workspace();
    let descriptor = descriptor_for_project(root.to_str().unwrap(), 7799).unwrap();
    assert_eq!(descriptor["transport"], "streamable-http");
    let config_path = PathBuf::from(descriptor["configPath"].as_str().unwrap());
    let config: Value = serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
    let url = reqwest::Url::parse(
        config["mcpServers"]["yilong_project_docs"]["url"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let session_id = url
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_string();
    let token = url
        .query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned())
        .unwrap();
    assert_eq!(
        authorize_session(&session_id, &token).unwrap(),
        root.canonicalize().unwrap()
    );
    assert!(authorize_session(&session_id, "wrong").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn mcp_lists_and_directly_calls_compact_document_tools() {
    let root = workspace();
    let listed = handle_request(
        &root,
        request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})),
    )
    .await
    .unwrap();
    let names = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "project_docs_analyze",
            "project_docs_get_issues",
            "project_docs_get_map",
            "project_docs_get_node",
            "project_docs_review_map",
            "project_docs_plan_context",
            "project_docs_get_status",
            "project_docs_read",
            "project_docs_get_suggestions",
            "project_docs_save_suggestions",
            "project_docs_apply_suggestions",
            "project_docs_apply_file_operations",
            "project_docs_update_issue",
            "project_docs_get_health_history",
            "project_docs_get_history",
            "project_docs_get_version_diff",
            "project_docs_restore_version"
        ]
    );
    let save_schema = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "project_docs_save_suggestions")
        .unwrap();
    let suggestion_properties =
        &save_schema["inputSchema"]["properties"]["suggestions"]["properties"];
    assert!(suggestion_properties.get("proposed_profile").is_some());
    assert!(suggestion_properties.get("proposed_home").is_some());
    assert!(suggestion_properties.get("document_metadata").is_some());
    assert!(suggestion_properties.get("governance_facets").is_some());
    assert!(suggestion_properties
        .get("proposed_knowledge_graph")
        .is_some());
    assert!(
        suggestion_properties["proposed_sections"]["items"]["properties"]
            .get("parent_id")
            .is_some()
    );
    let analyzed = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"project_docs_analyze","arguments":{"limit":20}}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        analyzed["result"]["structuredContent"]["budget"]["classification_model_tokens"],
        0
    );
    assert!(
        analyzed["result"]["structuredContent"]["knowledge_architecture"]
            .get("score")
            .is_some()
    );
    assert_eq!(
        analyzed["result"]["structuredContent"]["document_health"]["source"],
        "server"
    );
    assert!(
        analyzed["result"]["structuredContent"]["document_health"]["identity"]["manifest_revision"]
            .is_null()
    );
    let issues = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":22,"method":"tools/call",
            "params":{"name":"project_docs_get_issues","arguments":{"limit":20}}
        })),
    )
    .await
    .unwrap();
    assert!(
        issues["result"]["structuredContent"]["returned"]
            .as_u64()
            .unwrap()
            > 0
    );
    let fingerprint = issues["result"]["structuredContent"]["issues"][0]["fingerprint"]
        .as_str()
        .unwrap();
    let assigned = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":221,"method":"tools/call",
            "params":{"name":"project_docs_update_issue","arguments":{
                "fingerprint":fingerprint,"status":"assigned","owner":"docs-team","due_at":"2026-08-01"
            }}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        assigned["result"]["structuredContent"]["workflow"]["status"],
        "assigned"
    );
    let trend = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":222,"method":"tools/call",
            "params":{"name":"project_docs_get_health_history","arguments":{"limit":10}}
        })),
    )
    .await
    .unwrap();
    assert!(!trend["result"]["structuredContent"]["trend"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(analyzed.get("error").is_none());
    let map = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":23,"method":"tools/call",
            "params":{"name":"project_docs_get_map","arguments":{"view":"capabilities","depth":1,"max_nodes":20}}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        map["result"]["structuredContent"]["budget"]["markdown_bodies_read"],
        0
    );
    assert!(map["result"]["structuredContent"]["identity"]["canonical_workspace"].is_string());
    assert!(map["result"]["structuredContent"]["identity"]["knowledge_map_revision"].is_string());
    let root_node_id = map["result"]["structuredContent"]["root_id"]
        .as_str()
        .unwrap();
    let node = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":24,"method":"tools/call",
            "params":{"name":"project_docs_get_node","arguments":{"node_id":root_node_id}}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        node["result"]["structuredContent"]["budget"]["markdown_bodies_read"],
        0
    );
    let planned = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":25,"method":"tools/call",
            "params":{"name":"project_docs_plan_context","arguments":{"query":"Agent","max_tokens":500}}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        planned["result"]["structuredContent"]["budget"]["markdown_bodies_read"],
        0
    );
    let catalog_revision = analyzed["result"]["structuredContent"]["catalog_revision"]
        .as_str()
        .unwrap();
    let source_revision = analyzed["result"]["structuredContent"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|document| document["path"] == "AGENTS.md")
        .unwrap()["content_hash"]
        .as_str()
        .unwrap();
    let saved = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{
                "name":"project_docs_save_suggestions",
                "arguments":{
                    "expected_catalog_revision":catalog_revision,
                    "suggestions":{
                        "version":1,"status":"ready","summary":"共享入口保持必须文档。",
                        "proposed_sections":[],
                        "assignments":[{"path":"AGENTS.md","section_id":"required","reason":"共享路由入口"}],
                        "conflicts":[],"move_suggestions":[],
                        "file_operations":[{
                            "id":"rename-agent-entry","kind":"rename","source_path":"AGENTS.md",
                            "target_path":"AI_AGENT.md","source_revision":source_revision,
                            "reason":"使用可识别的共享入口名称","status":"proposed"
                        }],
                        "documents_read":0,"estimated_tokens_used":0
                    }
                }
            }
        })),
    )
    .await
    .unwrap();
    let suggestions_revision = saved["result"]["structuredContent"]["suggestions_revision"]
        .as_str()
        .unwrap();
    let applied = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{
                "name":"project_docs_apply_suggestions",
                "arguments":{
                    "expected_catalog_revision":catalog_revision,
                    "expected_suggestions_revision":suggestions_revision
                }
            }
        })),
    )
    .await
    .unwrap();
    assert_eq!(applied["result"]["structuredContent"]["status"], "applied");
    assert_eq!(
        applied["result"]["structuredContent"]["manifest"]["governance_overrides"]["AGENTS.md"],
        "required"
    );
    assert_eq!(
        applied["result"]["structuredContent"]["markdown_changed"],
        false
    );
    assert_eq!(
        applied["result"]["structuredContent"]["auto_authorized"],
        true
    );
    let applied_suggestions_revision = applied["result"]["structuredContent"]
        ["suggestions_revision"]
        .as_str()
        .unwrap();
    let git_baseline_commit = applied["result"]["structuredContent"]["git_baseline_commit"]
        .as_str()
        .unwrap();
    let files_applied = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{
                "name":"project_docs_apply_file_operations",
                "arguments":{
                    "operation_ids":["rename-agent-entry"],
                    "expected_catalog_revision":catalog_revision,
                    "expected_manifest_revision":applied["result"]["structuredContent"]["manifest_revision"],
                    "expected_suggestions_revision":applied_suggestions_revision
                    ,"git_baseline_commit":git_baseline_commit
                }
            }
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        files_applied["result"]["structuredContent"]["status"],
        "file_operations_applied"
    );
    assert_eq!(
        files_applied["result"]["structuredContent"]["auto_authorized"],
        true
    );
    assert_eq!(
        files_applied["result"]["structuredContent"]["git_document_transaction_complete"],
        true
    );
    assert!(
        files_applied["result"]["structuredContent"]["git_result_commit"]
            .as_str()
            .is_some()
    );
    assert!(root.join("AI_AGENT.md").is_file());
    assert!(!root.join("AGENTS.md").exists());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn managed_vault_descriptor_exposes_version_tools_without_a_user_git_workspace() {
    let vault_id = format!("mcp-test-{}", uuid::Uuid::new_v4().simple());
    let descriptor = descriptor_for_vault(&vault_id, 7799).unwrap();
    assert_eq!(descriptor["managedVaultId"], vault_id);
    let workspace = PathBuf::from(descriptor["projectRoot"].as_str().unwrap());
    assert!(workspace.join(".git").is_dir());
    let history = handle_request(
        &workspace,
        request(json!({
            "jsonrpc":"2.0","id":40,"method":"tools/call",
            "params":{"name":"project_docs_get_history","arguments":{}}
        })),
    )
    .await
    .unwrap();
    assert!(!history["result"]["structuredContent"]["versions"]
        .as_array()
        .unwrap()
        .is_empty());
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn descriptor_creation_is_safe_under_parallel_cleanup() {
    let roots = (0..32).map(|_| workspace()).collect::<Vec<_>>();
    let roots = Arc::new(roots);
    let workers = (0..32)
        .map(|index| {
            let roots = Arc::clone(&roots);
            std::thread::spawn(move || {
                let root = &roots[index];
                let descriptor = descriptor_for_project(root.to_str().unwrap(), 7799).unwrap();
                let url = reqwest::Url::parse(descriptor["url"].as_str().unwrap()).unwrap();
                let session_id = url
                    .path_segments()
                    .unwrap()
                    .next_back()
                    .unwrap()
                    .to_string();
                let token = url
                    .query_pairs()
                    .find(|(key, _)| key == "token")
                    .map(|(_, value)| value.into_owned())
                    .unwrap();
                authorize_session(&session_id, &token).unwrap()
            })
        })
        .collect::<Vec<_>>();
    for (index, worker) in workers.into_iter().enumerate() {
        assert_eq!(worker.join().unwrap(), roots[index].canonicalize().unwrap());
    }
    for root in roots.iter() {
        fs::remove_dir_all(root).unwrap();
    }
}

#[tokio::test]
async fn streamable_http_transport_accepts_direct_mcp_calls() {
    let root = workspace();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, test_transport_routes())
            .await
            .unwrap();
    });
    let descriptor = descriptor_for_project(root.to_str().unwrap(), port).unwrap();
    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(descriptor["url"].as_str().unwrap())
        .json(&json!({"jsonrpc":"2.0","id":9,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["result"]["tools"][0]["name"], "project_docs_analyze");
    server.abort();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn descriptor_rejects_non_git_directories() {
    let root = std::env::temp_dir().join(format!(
        "elon_project_docs_mcp_non_git_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    assert!(descriptor_for_project(root.to_str().unwrap(), 7799).is_err());
    fs::remove_dir_all(root).unwrap();
}
