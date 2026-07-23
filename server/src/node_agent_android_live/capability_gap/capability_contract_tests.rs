use serde_json::json;

use super::{check_capabilities, SUPPORTED_CAPABILITIES};
use crate::node_agent_android_live::broker::LiveUiBroker;

#[tokio::test]
async fn ui_check_capabilities_accepts_resumable_long_running_runtime_preparation() {
    assert!(SUPPORTED_CAPABILITIES.contains(&"MCP_LONG_RUNNING_TOOL_COMPLETION"));
    assert!(SUPPORTED_CAPABILITIES.contains(&"RESUMABLE_DEBUG_RUNTIME_PREPARATION"));
    assert!(SUPPORTED_CAPABILITIES.contains(&"ANDROID_RENDERER_DEVICE_HEALTH_RECOVERY"));

    let broker = LiveUiBroker::new();
    let session = broker
        .create_session(
            "test-device".into(),
            "com.example.test".into(),
            None,
            38_181,
        )
        .await;
    let result = check_capabilities(
        &session,
        &json!({
            "requiredCapabilities": [
                "MCP_LONG_RUNNING_TOOL_COMPLETION",
                "RESUMABLE_DEBUG_RUNTIME_PREPARATION",
                "ANDROID_RENDERER_DEVICE_HEALTH_RECOVERY"
            ]
        }),
    )
    .await
    .unwrap();

    assert_ne!(result["status"], "PLATFORM_GAP");
    assert_eq!(result["missing"], json!([]));
    let ready = result["ready"].as_array().unwrap();
    assert!(ready.contains(&json!("MCP_LONG_RUNNING_TOOL_COMPLETION")));
    assert!(ready.contains(&json!("RESUMABLE_DEBUG_RUNTIME_PREPARATION")));
    assert!(ready.contains(&json!("ANDROID_RENDERER_DEVICE_HEALTH_RECOVERY")));
}
