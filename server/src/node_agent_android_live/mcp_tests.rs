use serde_json::json;

use super::broker::LiveUiBroker;
use super::fit_run::FitRunService;
use super::mcp::{handle_request, McpRequest};

#[tokio::test]
async fn mcp_lists_compact_ui_tools() {
    let broker = std::sync::Arc::new(LiveUiBroker::new());
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
    let fit_runs = FitRunService::live(broker.clone());
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("tools/list must return a JSON-RPC response");
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_get_screen_summary"));
    assert!(tools.iter().any(|tool| tool["name"] == "ui_confirm_route"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_run_visual_solver"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_commit_bound_styles"));
    assert!(tools.iter().any(|tool| tool["name"] == "ui_start_fit_run"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_create_android_screen_scaffold"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_trace_window_insets_sequence"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_trace_relational_layout_geometry"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_start_capability_upgrade"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_complete_capability_upgrade"));
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_check_workflow_completion"));
    let desktop_import = tools
        .iter()
        .find(|tool| tool["name"] == "ui_import_desktop_task")
        .expect("desktop task import tool must be exposed");
    assert_eq!(
        desktop_import["inputSchema"]["properties"]["attachments"]["maxItems"],
        64
    );
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_write_cross_platform_verification"));
}

#[tokio::test]
async fn initialized_notification_has_no_json_rpc_response() {
    let broker = std::sync::Arc::new(LiveUiBroker::new());
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
        "method": "notifications/initialized",
        "params": {}
    }))
    .unwrap();
    let fit_runs = FitRunService::live(broker.clone());
    assert!(handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .is_none());
}
