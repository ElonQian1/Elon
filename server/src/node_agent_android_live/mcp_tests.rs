use serde_json::json;

use super::broker::LiveUiBroker;
use super::mcp::{handle_request, McpRequest};

#[tokio::test]
async fn mcp_lists_compact_ui_tools() {
    let broker = LiveUiBroker::new();
    let session = broker
        .create_session(
            "device-1".to_string(),
            "com.example.debug".to_string(),
            None,
            38917,
        )
        .await;
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }))
    .unwrap();
    let response = handle_request(&broker, &session.id, request).await;
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_get_screen_summary"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_run_visual_solver"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_commit_bound_styles"));
}
