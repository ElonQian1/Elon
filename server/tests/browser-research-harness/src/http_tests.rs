use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower::ServiceExt;

fn workspace() -> PathBuf {
    std::env::current_dir().unwrap().canonicalize().unwrap()
}
fn app() -> (Router, Arc<NodeRuntime>) {
    let runtime = Arc::new(NodeRuntime::default());
    (
        node_agent_browser_research::routes().with_state(runtime.clone()),
        runtime,
    )
}
async fn call(app: &Router, method: &str, uri: &str, body: String) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let bytes = to_bytes(response.into_body(), 80 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn submit(app: &Router, root: &Path) -> Value {
    let (status, value) = call(
        app,
        "POST",
        "/api/browser-research/actions",
        json!({"project_root":root,"command":{"kind":"sites"}}).to_string(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!value
        .to_string()
        .contains(&root.to_string_lossy().replace('\\', "\\\\")));
    value["action"].clone()
}
fn mcp(runtime: &NodeRuntime, root: &Path, action: &str, payload: Value) -> Value {
    let response = node_agent_browser_research_mcp::handle_request(runtime, root,
        &node_agent_project_docs_mcp::McpRequest {
            method:"tools/call".into(), params:json!({"name":"browser_research","arguments":{"action":action,"payload":payload}}),
        }).unwrap();
    serde_json::from_str(response["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn http_invalid_and_oversized_inputs_are_fixed_and_never_echo_private_text() {
    let (app, _) = app();
    for body in ["{PRIVATE_BROKEN_JSON".to_owned(),
        json!({"project_root":"PRIVATE_PATH","command":{"kind":"trade"}}).to_string(),
        json!({"project_root":"PRIVATE_PATH","command":{"kind":"sites","arbitrary_script":"PRIVATE_SCRIPT"}}).to_string(),
        "x".repeat(72 * 1024)] {
        let (status,value)=call(&app,"POST","/api/browser-research/actions",body).await;
        assert_eq!(status,StatusCode::BAD_REQUEST);
        assert_eq!(value["ok"],false);assert!(!value.to_string().contains("PRIVATE"));
    }
    let (status, value) = call(
        &app,
        "GET",
        "/api/browser-research/actions/pending?limit=0",
        String::new(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"], "invalid_limit");
}

#[tokio::test]
async fn http_claim_and_receipt_execute_the_real_queue_without_duplicate_execution() {
    let (app, _) = app();
    let action = submit(&app, &workspace()).await;
    let id = action["action_id"].as_str().unwrap();
    let claim_uri = format!("/api/browser-research/actions/{id}/claim");
    let (status, claim) = call(&app, "POST", &claim_uri, String::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(claim["action"]["status"], "executing");
    let (status, again) = call(&app, "POST", &claim_uri, String::new()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(again["error"], "action_not_claimable");
    let receipt_uri = format!("/api/browser-research/actions/{id}/receipt");
    let bad = json!({"claim_token":claim["claim_token"],"status":"succeeded","result":{"authorization":"PRIVATE_SECRET"}});
    let (status, value) = call(&app, "POST", &receipt_uri, bad.to_string()).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(value["error"], "credentials_forbidden");
    assert!(!value.to_string().contains("PRIVATE_SECRET"));
    let result = json!({"schema":"yilong.browser-research.result.v1","kind":"sites","items":[],
        "url":"https://example.org/?signature=%5Bcredential_excluded%5D"});
    let good = json!({"claim_token":claim["claim_token"],"status":"succeeded","result":result});
    for _ in 0..2 {
        let (status, value) = call(&app, "POST", &receipt_uri, good.to_string()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["status"], "succeeded");
        assert!(value.get("result").is_none());
    }
    let (status, value) = call(
        &app,
        "GET",
        &format!("/api/browser-research/actions/{id}"),
        String::new(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["terminal"], true);
    assert_eq!(value["action"]["receipt"]["result"], result);
    assert!(!value
        .to_string()
        .contains(claim["claim_token"].as_str().unwrap()));
}

#[tokio::test]
async fn http_project_scope_is_canonical_and_mcp_cannot_read_or_cancel_another_project() {
    let (app, runtime) = app();
    let root = workspace();
    let other = root.parent().unwrap();
    let first = submit(&app, &root).await;
    let second = submit(&app, other).await;
    assert_ne!(first["project_key"], second["project_key"]);
    // The trusted local-admin bridge sees all pending projects; descriptor MCP does not.
    let (status, pending) = call(
        &app,
        "GET",
        "/api/browser-research/actions/pending?limit=8",
        String::new(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pending["actions"].as_array().unwrap().len(), 2);
    for action in ["action_status", "cancel"] {
        let denied = mcp(
            &runtime,
            other,
            action,
            json!({"action_id":first["action_id"]}),
        );
        assert_eq!(denied["error"], "action_not_found");
    }
    let cancelled = mcp(
        &runtime,
        &root,
        "cancel",
        json!({"action_id":first["action_id"]}),
    );
    assert_eq!(cancelled["action"]["status"], "cancelled");
    assert_eq!(cancelled["terminal"], true);
    let (status, _) = call(
        &app,
        "POST",
        &format!(
            "/api/browser-research/actions/{}/claim",
            first["action_id"].as_str().unwrap()
        ),
        String::new(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}
