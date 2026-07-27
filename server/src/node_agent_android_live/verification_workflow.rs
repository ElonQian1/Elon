use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;

use super::broker::LiveUiBroker;

pub(super) const TOOL_NAME: &str = "ui_verify_with_fallback";

pub(super) fn tool_definition() -> Value {
    let pwa_schema = crate::node_agent_pwa_runtime::tool_definition()["inputSchema"].clone();
    json!({
        "name":TOOL_NAME,
        "description":"按 PWA 快速验证 → Android 模拟器回退 → 真机专项验收的策略执行。PWA 成功即返回紧凑证据；PWA 不适合或运行失败时自动准备真实 Android Renderer，并明确标记证据等级。",
        "inputSchema":{
            "type":"object",
            "additionalProperties":false,
            "required":["pwaSuitable"],
            "properties":{
                "pwaSuitable":{"type":"boolean","description":"共享 Web/布局/普通交互为 true；OEM、权限、键盘、启动器、硬件或性能专项为 false"},
                "resumeAndroid":{"type":"boolean","default":false,"description":"首次 PWA 已失败并进入 Android 准备后，轮询时设为 true，避免重复启动浏览器与截图"},
                "pwa":pwa_schema,
                "android":{
                    "type":"object",
                    "additionalProperties":false,
                    "properties":{
                        "basePackageName":{"type":"string"},
                        "deviceId":{"type":"string"},
                        "autoStartEmulator":{"type":"boolean","default":true},
                        "fallbackToEmulator":{"type":"boolean","default":true},
                        "debugApplicationIdSuffix":{"type":"string","default":".uitest"},
                        "isolatedEmulatorPackage":{"type":"boolean","default":false},
                        "lkgEnabled":{"type":"boolean","default":false},
                        "restart":{"type":"boolean","default":false}
                    }
                },
                "realDeviceRequired":{"type":"boolean","default":false,"description":"OEM、权限、键盘、启动器、硬件或性能专项设为 true；模拟器结果只作为降级证据"},
                "fallbackToEmulator":{"type":"boolean","default":true}
            }
        },
        "annotations":{
            "readOnlyHint":false,
            "destructiveHint":false,
            "idempotentHint":false,
            "openWorldHint":false
        }
    })
}

pub(super) async fn verify(
    broker: &Arc<LiveUiBroker>,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let pwa_suitable = arguments
        .get("pwaSuitable")
        .and_then(Value::as_bool)
        .ok_or_else(|| anyhow!("ui_verify_with_fallback 缺少 pwaSuitable"))?;
    let real_device_required = arguments
        .get("realDeviceRequired")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let fallback_to_emulator = arguments
        .get("fallbackToEmulator")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let resume_android = arguments
        .get("resumeAndroid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let session = broker.session(session_id).await?;

    if pwa_suitable && !resume_android {
        let pwa = arguments
            .get("pwa")
            .cloned()
            .ok_or_else(|| anyhow!("pwaSuitable=true 时必须提供 pwa 捕获参数"))?;
        let result =
            crate::node_agent_pwa_runtime::capture_tool(session.project_root.as_deref(), pwa).await;
        if result["ok"].as_bool() == Some(true) {
            return Ok(json!({
                "status":"VERIFIED",
                "verificationMode":"PWA",
                "evidenceLevel":"SHARED_SURFACE",
                "completed":!real_device_required,
                "requiresRealDevice":real_device_required,
                "evidence":compact_pwa_success(&result),
                "next":if real_device_required {"PREPARE_REAL_DEVICE"} else {"COMPLETE_SHARED_SURFACE_CHECK"}
            }));
        }
        let code = result
            .pointer("/diagnostic/code")
            .and_then(Value::as_str)
            .unwrap_or("CAPTURE_FAILED");
        if !fallback_to_emulator || !fallback_eligible(code) {
            return Ok(json!({
                "status":"PWA_FAILED",
                "verificationMode":"PWA",
                "completed":false,
                "evidence":compact_pwa_failure(&result),
                "next":"FIX_PWA_VALIDATION_INPUT"
            }));
        }
        return prepare_android(
            broker,
            session_id,
            arguments,
            real_device_required,
            Some(compact_pwa_failure(&result)),
        )
        .await;
    }

    prepare_android(broker, session_id, arguments, real_device_required, None).await
}

async fn prepare_android(
    broker: &Arc<LiveUiBroker>,
    session_id: &str,
    arguments: &Value,
    real_device_required: bool,
    pwa_failure: Option<Value>,
) -> Result<Value> {
    let mut android = arguments
        .get("android")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let android_object = android
        .as_object_mut()
        .ok_or_else(|| anyhow!("android 参数必须是 object"))?;
    android_object
        .entry("autoStartEmulator")
        .or_insert(json!(true));
    android_object
        .entry("fallbackToEmulator")
        .or_insert(json!(true));
    let prepared =
        super::mcp_runtime_preparation::prepare_debug_runtime(broker, session_id, &android).await?;
    let preparation_status = prepared
        .pointer("/result/status")
        .and_then(Value::as_str)
        .unwrap_or("IN_PROGRESS");
    let selection_source = prepared
        .pointer("/deviceSelection/source")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let emulator_evidence = selection_source.contains("emulator");
    Ok(json!({
        "status":match preparation_status {
            "COMPLETED" => "ANDROID_RENDERER_READY",
            "FAILED" => "ANDROID_FALLBACK_FAILED",
            _ => "ANDROID_FALLBACK_IN_PROGRESS",
        },
        "verificationMode":if emulator_evidence {"ANDROID_EMULATOR"} else {"ANDROID_DEVICE"},
        "evidenceLevel":if real_device_required && emulator_evidence {
            "PROVISIONAL_EMULATOR_FALLBACK"
        } else {
            "ANDROID_RENDERER"
        },
        "completed":false,
        "capabilityGapRequired":preparation_status == "FAILED",
        "gapDisposition":match preparation_status {
            "COMPLETED" => "ALTERNATIVE_VERIFICATION_READY",
            "FAILED" => "REPORT_ONLY_AFTER_PLATFORM_GAP_IS_PROVEN",
            _ => "ALTERNATIVE_VERIFICATION_PENDING",
        },
        "requiresRealDevice":real_device_required && emulator_evidence,
        "pwaFailure":pwa_failure,
        "android":compact_android_preparation(&prepared),
        "next":match preparation_status {
            "COMPLETED" => "REPLAY_STATE_AND_CAPTURE_ANDROID_EVIDENCE",
            "FAILED" => "RETRY_ANDROID_PREPARATION_WITH_RESUME_ANDROID_TRUE",
            _ => "POLL_UI_VERIFY_WITH_RESUME_ANDROID_TRUE",
        }
    }))
}

fn fallback_eligible(code: &str) -> bool {
    matches!(
        code,
        "AUTHENTICATION_REQUIRED"
            | "AUTHENTICATION_FAILED"
            | "BROWSER_NOT_FOUND"
            | "NAVIGATION_FAILED"
            | "CAPTURE_TIMEOUT"
            | "BROWSER_PROTOCOL_ERROR"
            | "BROWSER_PROTOCOL_TIMEOUT"
            | "INTERACTION_STEP_FAILED"
    )
}

fn compact_pwa_success(result: &Value) -> Value {
    json!({
        "path":result.pointer("/artifact/path"),
        "sha256":result.pointer("/artifact/sha256"),
        "width":result.pointer("/artifact/width"),
        "height":result.pointer("/artifact/height"),
        "route":result.pointer("/route/path"),
        "sourceRevision":result.pointer("/revision/sourceRevision"),
        "executedStepCount":result.pointer("/interaction/executedStepCount"),
        "base64Embedded":false,
    })
}

fn compact_pwa_failure(result: &Value) -> Value {
    json!({
        "code":result.pointer("/diagnostic/code"),
        "message":result.pointer("/diagnostic/message"),
        "nextStep":result.pointer("/diagnostic/nextStep"),
    })
}

fn compact_android_preparation(result: &Value) -> Value {
    json!({
        "operationId":result.pointer("/result/operationId"),
        "status":result.pointer("/result/status"),
        "phase":result.pointer("/result/phase"),
        "retryAfterMs":result.pointer("/result/retryAfterMs"),
        "deviceId":result.pointer("/deviceSelection/deviceId"),
        "deviceSource":result.pointer("/deviceSelection/source"),
        "avdName":result.pointer("/deviceSelection/avdName"),
        "nextPhase":result.pointer("/nextPhase"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_policy_distinguishes_runtime_failure_from_bad_input() {
        assert!(fallback_eligible("AUTHENTICATION_REQUIRED"));
        assert!(fallback_eligible("INTERACTION_STEP_FAILED"));
        assert!(!fallback_eligible("URL_ORIGIN_NOT_ALLOWED"));
        assert!(!fallback_eligible("INVALID_ARGUMENTS"));
    }

    #[test]
    fn compact_evidence_never_embeds_base64_or_browser_noise() {
        let compact = compact_pwa_success(&json!({
            "artifact":{"path":"capture.png","sha256":"abc","width":360,"height":640},
            "route":{"path":"/projects"},
            "revision":{"sourceRevision":"r1"},
            "interaction":{"executedStepCount":2},
            "browser":{"userAgent":"huge"},
            "png":"base64-secret"
        }));
        assert_eq!(compact["path"], "capture.png");
        assert_eq!(compact["base64Embedded"], false);
        assert!(compact.get("browser").is_none());
        assert!(!compact.to_string().contains("base64-secret"));
    }

    #[test]
    fn tool_schema_exposes_low_cost_android_resume() {
        let definition = tool_definition();
        assert_eq!(
            definition["inputSchema"]["properties"]["resumeAndroid"]["default"],
            false
        );
    }
}
