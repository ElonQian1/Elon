use serde_json::json;

use super::broker::LiveUiBroker;
use super::fit_run::{
    CreateFitRunRequest, FitEnvironment, FitRect, FitRunService, FitSessionContext, FitTargetPair,
};
use super::mcp::{handle_request, McpRequest};
use super::protocol::{LiveGeometry, LiveRect, LiveUiNode};

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
async fn capability_gap_can_be_reported_while_runtime_is_disconnected() {
    let broker = std::sync::Arc::new(LiveUiBroker::new());
    let root = std::env::temp_dir().join(format!(
        "elon-disconnected-gap-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let session = broker
        .create_session(
            "offline-device".to_string(),
            "offline.runtime".to_string(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    assert!(!session.view().await.connected);
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":"ui_report_capability_gap",
            "arguments":{
                "taskId":"offline-gap-regression",
                "executionMode":"BUSINESS_THREAD",
                "deliveryImpact":"DELIVERY_BLOCKING",
                "originThreadId":"offline-origin",
                "missingCapabilities":["PLATFORM_TOOL_DEFECT"],
                "evidence":["Android Runtime is intentionally disconnected"],
                "proposedChanges":["Keep capability-gap reporting on the local MCP control plane"],
                "resumeTarget":"Resume after the platform upgrade is rechecked"
            }
        }
    }))
    .unwrap();
    let fit_runs = FitRunService::live(broker.clone());
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("disconnected capability-gap response");
    let result = &response["result"]["structuredContent"];
    assert_eq!(result["gap"]["status"], "DEFERRED", "{result:#}");
    assert_eq!(
        result["gap"]["delegation"]["executionMode"],
        "BUSINESS_THREAD"
    );
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
    let activate_preview = tools
        .iter()
        .find(|tool| tool["name"] == "ui_activate_preview_scenario")
        .expect("preview scenario activation tool must be discoverable");
    assert_eq!(
        activate_preview["inputSchema"]["properties"]["scenario"]["type"],
        "string"
    );
    assert!(
        activate_preview["inputSchema"]["properties"]["scenario"]["enum"].is_null(),
        "registered preview scenarios must not be frozen in the MCP schema"
    );
    let build_verify = tools
        .iter()
        .find(|tool| tool["name"] == "ui_build_and_verify")
        .expect("background build verification tool must be discoverable");
    assert_eq!(
        build_verify["inputSchema"]["properties"]["operationId"]["type"],
        "string"
    );
    assert_eq!(
        build_verify["inputSchema"]["properties"]["preview"]["properties"]["scenario"]["type"],
        "string"
    );
    assert!(
        build_verify["inputSchema"]["properties"]["preview"]["properties"]["scenario"]["enum"]
            .is_null(),
        "ui_build_and_verify must accept every runtime-registered scenario"
    );
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "ui_create_android_screen_scaffold"));
    let sequence_trace = tools
        .iter()
        .find(|tool| tool["name"] == "ui_trace_window_insets_sequence")
        .expect("window sequence trace tool must be exposed");
    assert!(
        sequence_trace["inputSchema"]["properties"]["steps"]["items"]["properties"]["action"]
            ["properties"]["type"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "ACTIVATE_NODE")
    );
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
    assert_eq!(
        launcher["inputSchema"]["properties"]["mode"]["default"],
        "PACKAGE_ICON"
    );
    assert_eq!(
        launcher["inputSchema"]["properties"]["deviceId"]["maxLength"],
        128
    );
    let visual_diff = tools
        .iter()
        .find(|tool| tool["name"] == "ui_get_visual_diff")
        .expect("visual diff tool");
    assert_eq!(
        visual_diff["inputSchema"]["properties"]["mask"]["properties"]["adaptiveIconMask"]
            ["properties"]["shape"]["enum"][0],
        "CIRCLE"
    );
    assert_eq!(
        visual_diff["inputSchema"]["properties"]["currentArtifact"]["properties"]["source"]
            ["const"],
        "ANDROID_LAUNCHER"
    );
    assert_eq!(
        visual_diff["inputSchema"]["properties"]["currentArtifact"]["required"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    let mask_renderer = tools
        .iter()
        .find(|tool| tool["name"] == "ui_render_android_launcher_masks")
        .expect("launcher mask renderer");
    assert_eq!(mask_renderer["annotations"]["readOnlyHint"], true);
    assert_eq!(
        mask_renderer["inputSchema"]["properties"]["shapes"]["items"]["enum"]
            .as_array()
            .map(Vec::len),
        Some(3)
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
async fn build_verify_polling_rejects_untrusted_operation_ids_without_starting_work() {
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
        "jsonrpc":"2.0","id":5,"method":"tools/call","params":{
            "name":"ui_build_and_verify",
            "arguments":{"operationId":"ui_build_verify_../../escape"}
        }
    }))
    .unwrap();
    let fit_runs = FitRunService::live(broker.clone());
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("invalid operation polling must return a JSON-RPC error");
    assert!(response["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("operationId 格式无效")));
}

#[tokio::test]
async fn mcp_attaches_explicit_state_replay_to_an_existing_fit_run() {
    let root = std::env::temp_dir().join(format!(
        "elon-mcp-fit-replay-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let broker = std::sync::Arc::new(LiveUiBroker::new());
    let session = broker
        .create_session(
            "device-1".into(),
            "com.example.debug".into(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    let target = LiveUiNode {
        runtime_node_id: "runtime-chat-composer".into(),
        definition_id: "chat.message.composer".into(),
        instance_key: Some("primary".into()),
        parent_runtime_node_id: None,
        screen_id: "chat".into(),
        kind: "text-field".into(),
        text: None,
        resource_id: None,
        class_name: "EditText".into(),
        source: None,
        geometry: LiveGeometry {
            bounds_in_display_px: LiveRect {
                left: 0,
                top: 100,
                right: 200,
                bottom: 150,
                width: 200,
                height: 50,
            },
            density: 1.0,
            font_scale: 1.0,
            rotation: 0,
            visible: true,
        },
        properties: Default::default(),
        capabilities: Default::default(),
    };
    session
        .set_runtime_state_for_test(vec![target], Some("build-1".into()))
        .await;
    let fit_runs = FitRunService::live(broker.clone());
    let run = fit_runs
        .create_run(
            FitSessionContext {
                session_id: session.id.clone(),
                project_root: root.display().to_string(),
                package_name: session.package_name.clone(),
                device_id: session.device_id.clone(),
                runtime_build_id: Some("build-1".into()),
                tree_revision: 1,
                source_revision: None,
            },
            CreateFitRunRequest {
                task_id: Some("task-mcp-replay".into()),
                pair: FitTargetPair {
                    target_design_id: "design-chat".into(),
                    target_sha256: "abc123".into(),
                    target_rect: FitRect {
                        left: 0,
                        top: 0,
                        right: 200,
                        bottom: 50,
                    },
                    runtime_node_id: "runtime-chat-composer".into(),
                    definition_id: "chat.message.composer".into(),
                    component_kind: Some("text-field".into()),
                    parent_layout_kind: Some("column".into()),
                    instance_key: Some("primary".into()),
                    current_rect: FitRect {
                        left: 0,
                        top: 100,
                        right: 200,
                        bottom: 150,
                    },
                    projected_target_rect: FitRect {
                        left: 0,
                        top: 100,
                        right: 200,
                        bottom: 150,
                    },
                    calibration_id: None,
                    confidence: Some(1.0),
                },
                environment: FitEnvironment::default(),
                properties: Vec::new(),
                budget: Default::default(),
                thresholds: Default::default(),
                visual_mask: Default::default(),
                auto_start: false,
            },
        )
        .await
        .unwrap();
    let captured_at = chrono::Utc::now();
    let request: McpRequest = serde_json::from_value(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call","params":{
            "name":"ui_control_fit_run",
            "arguments":{
                "runId":run.run_id,
                "action":"ATTACH_STATE_REPLAY",
                "projectRoot":root.display().to_string(),
                "scenario":"CHAT_PAGE",
                "targetRuntimeNodeId":"runtime-chat-composer",
                "targetDefinitionId":"chat.message.composer",
                "targetInstanceKey":"primary",
                "stateReplay":{
                    "scenarioId":"CHAT_PAGE",
                    "capturedAt":captured_at.to_rfc3339(),
                    "expiresAt":(captured_at + chrono::Duration::minutes(10)).to_rfc3339(),
                    "steps":[
                        {"name":"open-chat","action":{"type":"ACTIVATE_NODE","definitionId":"home.navigation.chat","occurrence":0}},
                        {"name":"settle","action":{"type":"WAIT","durationMs":500}}
                    ]
                }
            }
        }
    }))
    .unwrap();
    let response = handle_request(&broker, &fit_runs, &session.id, request)
        .await
        .expect("ATTACH_STATE_REPLAY response");
    let result = &response["result"]["structuredContent"]["result"];
    assert_eq!(result["idempotent"], false, "{response:#}");
    assert_eq!(result["run"]["environment"]["scenario"], "CHAT_PAGE");
    assert_eq!(
        result["run"]["auditEvents"][0]["outcome"], "ATTACHED",
        "{response:#}"
    );
    std::fs::remove_dir_all(root).unwrap();
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
