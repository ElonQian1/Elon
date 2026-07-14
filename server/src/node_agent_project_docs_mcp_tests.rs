use serde_json::{json, Value};
use std::sync::Arc;
use std::{fs, path::PathBuf};

use crate::node_agent_project_docs_mcp::{
    authorize_session, descriptor_for_project, handle_request, test_transport_routes, McpRequest,
};

fn workspace() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_project_docs_mcp_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join("AGENTS.md"), "# Agent entry\n").unwrap();
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
            "project_docs_get_status",
            "project_docs_read",
            "project_docs_get_suggestions",
            "project_docs_save_suggestions",
            "project_docs_apply_suggestions"
        ]
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
    assert!(analyzed.get("error").is_none());
    let catalog_revision = analyzed["result"]["structuredContent"]["catalog_revision"]
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
                        "conflicts":[],"move_suggestions":[],"documents_read":0,"estimated_tokens_used":0
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
                    "reviewed":true,
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
        applied["result"]["structuredContent"]["markdown_changed"],
        false
    );
    fs::remove_dir_all(root).unwrap();
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
