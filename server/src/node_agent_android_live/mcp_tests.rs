use serde_json::json;

use super::broker::LiveUiBroker;
use super::fit_run::FitRunService;
use super::mcp::{handle_request, McpRequest};

#[tokio::test]
async fn launcher_icon_capabilities_are_declared_ready_without_live_runtime() {
    let broker = std::sync::Arc::new(LiveUiBroker::new());
    let root = std::env::temp_dir().join(format!(
        "elon-launcher-capability-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let session = broker
        .create_session(
            "device-1".to_string(),
            "com.example.debug".to_string(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call","params":{
            "name":"ui_check_capabilities",
            "arguments":{"requiredCapabilities":[
                "ANDROID_LAUNCHER_SURFACE_CAPTURE",
                "ANDROID_ADAPTIVE_ICON_MASK_VISUAL_DIFF"
            ]}
        }
    }))
    .unwrap();
    let fit_runs = FitRunService::live(broker.clone());
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("capability response");
    let result = &response["result"]["structuredContent"];
    assert!(result["ready"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "ANDROID_LAUNCHER_SURFACE_CAPTURE"));
    assert!(result["ready"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "ANDROID_ADAPTIVE_ICON_MASK_VISUAL_DIFF"));
    std::fs::remove_dir_all(root).unwrap();
}

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
    let launcher = tools
        .iter()
        .find(|tool| tool["name"] == "ui_capture_android_launcher_surface")
        .expect("launcher surface capture tool must be discoverable");
    assert_eq!(launcher["annotations"]["readOnlyHint"], true);
    let visual_diff = tools
        .iter()
        .find(|tool| tool["name"] == "ui_get_visual_diff")
        .expect("visual diff tool");
    assert_eq!(
        visual_diff["inputSchema"]["properties"]["mask"]["properties"]["adaptiveIconMask"]
            ["properties"]["shape"]["enum"][0],
        "CIRCLE"
    );
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
    let pwa_capture = tools
        .iter()
        .find(|tool| tool["name"] == "ui_capture_pwa_runtime")
        .expect("PWA runtime capture tool must be discoverable");
    assert_eq!(pwa_capture["annotations"]["openWorldHint"], false);
    assert_eq!(pwa_capture["inputSchema"]["additionalProperties"], false);
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

#[tokio::test]
async fn mcp_call_renders_real_local_pwa_fixture_without_desktop_browser() {
    use std::fs;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    let root = std::env::temp_dir().join(format!(
        "elon-pwa-mcp-e2e-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let fixture = tokio::spawn(async move {
        let html = r#"<!doctype html><style>html,body{margin:0;background:#162b3d}#ready{width:100px;height:60px;background:#56ccf2}</style><main id="ready">MCP proof</main>"#;
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    let broker = std::sync::Arc::new(LiveUiBroker::new());
    let session = broker
        .create_session(
            "pwa-bootstrap".to_string(),
            "pwa.runtime".to_string(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc":"2.0","id":9,"method":"tools/call","params":{
            "name":"ui_capture_pwa_runtime",
            "arguments":{
                "url":format!("http://{address}/proof?mode=fixture"),
                "viewport":{"width":320,"height":480,"deviceScaleFactor":1},
                "waitFor":{"condition":"networkidle","selector":"#ready","timeoutMs":8000,"settleMs":100},
                "evidence":{"sourceRevision":format!("fixture-sha256:{}", "b".repeat(64)),"routeRevision":"mcp-fixture-r1"}
            }
        }
    }))
    .unwrap();
    let fit_runs = FitRunService::live(broker.clone());
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("tools/call response");
    let captured = &response["result"]["structuredContent"];
    if captured
        .pointer("/diagnostic/code")
        .and_then(serde_json::Value::as_str)
        == Some("BROWSER_NOT_FOUND")
    {
        fixture.abort();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    assert_eq!(captured["ok"], true, "{captured:#}");
    assert_eq!(captured["artifact"]["width"], 320);
    assert_eq!(captured["artifact"]["height"], 480);
    assert_eq!(captured["artifact"]["mediaType"], "image/png");
    assert_eq!(captured["route"]["path"], "/proof");
    assert_eq!(captured["revision"]["routeRevision"], "mcp-fixture-r1");
    assert_eq!(captured["browser"]["headless"], true);
    assert_eq!(captured["processCleanup"]["browserProcessReaped"], true);
    assert_eq!(captured["processCleanup"]["temporaryProfileRemoved"], true);
    assert_eq!(captured["contextPackReference"]["embedBase64"], false);
    assert_eq!(captured["base64Embedded"], false);
    assert!(std::path::Path::new(captured["artifact"]["path"].as_str().unwrap()).is_file());
    fixture.abort();
    fs::remove_dir_all(root).unwrap();
}
