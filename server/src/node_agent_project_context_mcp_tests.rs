use super::{definition, handle_request, handles, PROFILE, TOOL_NAME};
use crate::{
    node_agent_project_docs_mcp::{
        descriptor_for_project_context, descriptor_for_project_feature,
        descriptor_for_project_receipt, test_transport_routes, McpRequest,
    },
    node_agent_project_docs_mcp_native_context_tools::RECEIPT_TOOL,
    node_agent_project_feature_mcp::TOOL_NAME as FEATURE_TOOL,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

struct ContextFixture {
    root: PathBuf,
    receipt_path: PathBuf,
}

impl ContextFixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon_project_context_{label}_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join(".elon")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("AI_CURRENT.md"),
            "# Current status\n\nPRIVATE_SOURCE_BODY_MARKER must never leave the workspace.\n",
        )
        .unwrap();
        fs::write(
            root.join("docs/README.md"),
            "# Project overview\n\nThe current implementation entrypoint.\n",
        )
        .unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            root.join(".elon/document-sections.json"),
            serde_json::to_vec_pretty(&json!({
                "version": 1,
                "profile": "software-api",
                "home": {
                    "title": "Context fixture",
                    "summary": "Bounded context delivery test",
                    "entrypoint": "AI_CURRENT.md",
                    "start_here": ["AI_CURRENT.md"]
                },
                "sections": [
                    {"id":"overview","label":"Overview","detail":"Current truth","color":"#4FA9B8","entrypoint":"AI_CURRENT.md"}
                ],
                "assignments": {
                    "AI_CURRENT.md": "custom:overview",
                    "docs/README.md": "custom:overview"
                },
                "governance_facets": {
                    "AI_CURRENT.md": {"retrieval":"required","lifecycle":"active","authority":"authoritative","document_type":"current_status"},
                    "docs/README.md": {"retrieval":"on_demand","lifecycle":"active","authority":"authoritative","document_type":"overview"}
                },
                "document_metadata": {
                    "AI_CURRENT.md": {"doc_type":"current_status","owner":"test","reviewed_at":"2026-08-10","version_status":"current"},
                    "docs/README.md": {"doc_type":"overview","owner":"test","reviewed_at":"2026-08-10","version_status":"current"}
                },
                "knowledge_graph": {
                    "nodes": [{
                        "id": "current-status",
                        "view": "capabilities",
                        "kind": "capability",
                        "label": "Current status",
                        "detail": "Verified current project state",
                        "color": "#4FA9B8",
                        "entrypoint": "AI_CURRENT.md",
                        "document_paths": ["AI_CURRENT.md"],
                        "implementation_refs": ["file:src/main.rs"]
                    }],
                    "edges": []
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let receipt_path = root.join("context-session.json");
        Self { root, receipt_path }
    }

    fn call(&self, arguments: Value) -> Value {
        let request: McpRequest = serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": TOOL_NAME, "arguments": arguments}
        }))
        .unwrap();
        handle_request(&self.root, &request, Some(&self.receipt_path)).unwrap()
    }
}

impl Drop for ContextFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn context_profile_exposes_one_bounded_read_only_tool() {
    let definition = definition();
    assert_eq!(definition["name"], TOOL_NAME);
    assert_eq!(
        definition["inputSchema"]["properties"]["max_tokens"]["maximum"],
        2400
    );
    assert_eq!(
        definition["inputSchema"]["properties"]["max_documents"]["maximum"],
        8
    );
    assert_eq!(
        definition["inputSchema"]["properties"]["max_response_tokens"]["maximum"],
        2000
    );
    assert!(definition["inputSchema"]["properties"]["previous_plan_id"].is_object());
    assert!(handles(Some(PROFILE)));
    assert!(!handles(None));
}

#[test]
fn context_profile_lists_only_the_navigation_tool() {
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .unwrap();
    let response = handle_request(&PathBuf::from("."), &request, None).unwrap();
    assert_eq!(response["tools"].as_array().unwrap().len(), 1);
    assert_eq!(response["tools"][0]["name"], TOOL_NAME);
}

#[test]
fn context_session_reuses_plan_without_persisting_task_or_source_bodies() {
    const QUERY: &str = "PRIVATE_QUERY_MARKER summarize current project state";
    let fixture = ContextFixture::new("session-reuse");
    let arguments = json!({"query":QUERY,"max_documents":4,"max_tokens":800});

    let first = fixture.call(arguments.clone());
    let first_content = &first["structuredContent"];
    assert_eq!(first_content["delivery_receipt"]["mode"], "full");
    assert_eq!(first_content["contract"]["source_bodies_returned"], 0);
    assert!(
        first_content["performance_receipt"]["transport"]["mcp_tool_result_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        first_content["performance_receipt"]["transport"]["not_vendor_billing"],
        true
    );
    let first_json = serde_json::to_string(&first).unwrap();
    assert!(!first_json.contains("PRIVATE_SOURCE_BODY_MARKER"));

    let persisted = fs::read_to_string(&fixture.receipt_path).unwrap();
    assert!(!persisted.contains("PRIVATE_QUERY_MARKER"));
    assert!(!persisted.contains("PRIVATE_SOURCE_BODY_MARKER"));
    assert!(persisted.contains("AI_CURRENT.md"));

    let second = fixture.call(arguments);
    let second_content = &second["structuredContent"];
    assert_eq!(second_content["status"], "not_modified");
    assert_eq!(second_content["delivery_receipt"]["mode"], "not_modified");
    assert_eq!(
        second_content["delivery_receipt"]["automatic_previous_plan_reuse"],
        true
    );
    assert_eq!(second_content["delivery_receipt"]["delivery_count"], 2);
}

#[test]
fn force_refresh_bypasses_the_session_not_modified_receipt() {
    let fixture = ContextFixture::new("force-refresh");
    let arguments = json!({"query":"summarize current project state","max_documents":4});
    let first = fixture.call(arguments.clone());
    assert_eq!(
        first["structuredContent"]["delivery_receipt"]["mode"],
        "full"
    );

    let refreshed = fixture.call(json!({
        "query": arguments["query"],
        "max_documents": 4,
        "force_refresh": true
    }));
    assert_eq!(
        refreshed["structuredContent"]["delivery_receipt"]["mode"],
        "full"
    );
    assert_ne!(refreshed["structuredContent"]["status"], "not_modified");
    assert_ne!(refreshed["structuredContent"]["cache"]["status"], "hit");
}

#[tokio::test]
async fn streamable_transport_keeps_minimal_profiles_fixed_and_separate() {
    let root = std::env::temp_dir().join(format!(
        "elon_project_memory_profile_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, test_transport_routes())
            .await
            .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let context = descriptor_for_project_context(root.to_str().unwrap(), port).unwrap();
    let feature = descriptor_for_project_feature(root.to_str().unwrap(), port).unwrap();
    let receipt = descriptor_for_project_receipt(root.to_str().unwrap(), port).unwrap();

    for (descriptor, expected_tool) in [
        (&context, TOOL_NAME),
        (&feature, FEATURE_TOOL),
        (&receipt, RECEIPT_TOOL),
    ] {
        let response = client
            .post(descriptor["url"].as_str().unwrap())
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let body: Value = response.json().await.unwrap();
        let tools = body["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], expected_tool);
    }

    let switched_url = context["url"]
        .as_str()
        .unwrap()
        .replace("profile=context", "profile=receipt");
    let switched = client
        .post(switched_url)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(switched.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.abort();
    for descriptor in [&context, &feature, &receipt] {
        let session_id = descriptor["sessionId"].as_str().unwrap();
        fs::remove_dir_all(
            std::env::temp_dir()
                .join("elon-project-docs-mcp")
                .join(session_id),
        )
        .unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
