use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

use super::broker::LiveUiBroker;

pub(super) const TOOL_NAME: &str = "ui_verify_with_fallback";

pub(super) fn tool_definition() -> Value {
    let pwa_schema = crate::node_agent_pwa_runtime::tool_definition()["inputSchema"].clone();
    json!({
        "name":TOOL_NAME,
        "description":"按 PWA 快速验证 → Android 模拟器验证 → 用户反馈不正确后真机复核的策略执行。普通任务不占用物理设备；显式真机复核在同一 MCP 会话内轮询并明确标记证据等级。",
        "inputSchema":{
            "type":"object",
            "additionalProperties":false,
            "required":["pwaSuitable"],
            "properties":{
                "pwaSuitable":{"type":"boolean","description":"共享 Web/布局/普通交互为 true；无法用 PWA 复现的 Android 专项为 false"},
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
                        "preferEmulator":{"type":"boolean","default":false},
                        "fastFailPhysicalVisualProbe":{"type":"boolean","default":true},
                        "debugApplicationIdSuffix":{"type":"string","default":".uitest"},
                        "isolatedEmulatorPackage":{"type":"boolean","default":false},
                        "lkgEnabled":{"type":"boolean","default":false},
                        "taskId":{"type":"string"},
                        "restart":{"type":"boolean","default":false}
                    }
                },
                "realDeviceRequired":{"type":"boolean","default":false,"description":"仅当用户反馈修改结果不正确或明确要求真机复核时设为 true；普通 UI、Logo、Launcher、OEM、权限和硬件任务默认不占用真机"},
                "fallbackToEmulator":{"type":"boolean","default":true},
                "physicalDeviceBudgetMs":{"type":"integer","minimum":5000,"maximum":60000,"default":30000,"description":"用户触发真机复核后的单次准备预算；失败或超时立即延期，不重复配对或重建会话"}
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
    let mut android = configure_android_arguments(arguments, real_device_required)?;
    let mut prepared =
        super::mcp_runtime_preparation::prepare_debug_runtime(broker, session_id, &android).await?;
    let mut physical_failure = None;
    let physical_budget_ms = arguments
        .get("physicalDeviceBudgetMs")
        .and_then(Value::as_u64)
        .unwrap_or(30_000)
        .clamp(5_000, 60_000);
    let fallback_enabled = !real_device_required
        && arguments
            .get("fallbackToEmulator")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    let initial_status = preparation_status(&prepared);
    let initial_source = selection_source(&prepared);
    let initial_is_emulator = initial_source.contains("emulator");
    let elapsed_ms = preparation_elapsed_ms(&prepared).unwrap_or_default();
    if should_fallback_physical_renderer(
        initial_status,
        initial_is_emulator,
        real_device_required,
        fallback_enabled,
        elapsed_ms,
        physical_budget_ms,
    ) {
        physical_failure = Some(json!({
            "status":initial_status,
            "phase":prepared.pointer("/result/phase"),
            "error":prepared.pointer("/result/error"),
            "deviceId":prepared.pointer("/deviceSelection/deviceId"),
            "elapsedMs":elapsed_ms,
            "budgetMs":physical_budget_ms,
            "reason":if initial_status == "FAILED" {
                "PHYSICAL_RUNTIME_PREPARATION_FAILED"
            } else {
                "PHYSICAL_DEVICE_BUDGET_EXHAUSTED"
            }
        }));
        let mut emulator_android = android;
        let emulator_object = emulator_android
            .as_object_mut()
            .ok_or_else(|| anyhow!("android 参数必须是 object"))?;
        emulator_object.remove("deviceId");
        emulator_object.insert("preferEmulator".into(), json!(true));
        emulator_object.insert("fastFailPhysicalVisualProbe".into(), json!(false));
        emulator_object.insert("fallbackToEmulator".into(), json!(true));
        emulator_object.insert("isolatedEmulatorPackage".into(), json!(true));
        emulator_object.insert("restart".into(), json!(false));
        prepared = super::mcp_runtime_preparation::prepare_debug_runtime(
            broker,
            session_id,
            &emulator_android,
        )
        .await?;
    }
    let preparation_status = preparation_status(&prepared);
    let selection_source = selection_source(&prepared);
    let emulator_evidence = selection_source.contains("emulator");
    let selection_fallback = selection_source.starts_with("fallback_")
        || prepared
            .pointer("/deviceSelection/fallbackReason")
            .and_then(Value::as_str)
            .is_some();
    let real_device_status = if real_device_required && preparation_status != "COMPLETED" {
        "REQUIRED_FOLLOWUP"
    } else if emulator_evidence && real_device_required {
        "REQUIRED_FOLLOWUP"
    } else if emulator_evidence && (physical_failure.is_some() || selection_fallback) {
        "DEFERRED_USER_CONFIRMATION"
    } else if emulator_evidence {
        "NOT_REQUIRED"
    } else if preparation_status == "COMPLETED" {
        "READY"
    } else {
        "PROBING"
    };
    Ok(json!({
        "status":match preparation_status {
            "COMPLETED" => "ANDROID_RENDERER_READY",
            "FAILED" if real_device_required => "REAL_DEVICE_VERIFICATION_DEFERRED",
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
            "FAILED" if real_device_required => "REAL_DEVICE_VERIFICATION_DEFERRED",
            "FAILED" => "REPORT_ONLY_AFTER_PLATFORM_GAP_IS_PROVEN",
            _ => "ALTERNATIVE_VERIFICATION_PENDING",
        },
        "requiresRealDevice":real_device_required && preparation_status != "COMPLETED",
        "pwaFailure":pwa_failure,
        "physicalFailure":physical_failure,
        "android":compact_android_preparation(&prepared),
        "ANDROID_RENDERER":if emulator_evidence {"EMULATOR"} else {"PHYSICAL_DEVICE"},
        "REAL_DEVICE_STATUS":real_device_status,
        "rendererResourceId":prepared.pointer("/rendererLease/rendererResourceId"),
        "leaseOwner":prepared.pointer("/rendererLease/owner"),
        "sourceSha":prepared.pointer("/rendererLease/owner/sourceSha"),
        "next":match preparation_status {
            "COMPLETED" => "REPLAY_STATE_AND_CAPTURE_ANDROID_EVIDENCE",
            "FAILED" if real_device_required => "STOP_AND_REQUEST_RUNTIME_RECOVERY",
            "FAILED" => "STOP_AFTER_ANDROID_PREPARATION_FAILURE",
            _ => "POLL_UI_VERIFY_WITH_RESUME_ANDROID_TRUE",
        }
    }))
}

fn configure_android_arguments(arguments: &Value, real_device_required: bool) -> Result<Value> {
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
    if real_device_required {
        android_object.remove("preferEmulator");
        let emulator_device_id = android_object
            .get("deviceId")
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("emulator-"));
        if emulator_device_id {
            android_object.remove("deviceId");
        }
        android_object.insert("fallbackToEmulator".into(), json!(false));
    } else {
        let physical_device_id = android_object
            .get("deviceId")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.starts_with("emulator-"));
        if physical_device_id {
            android_object.remove("deviceId");
        }
        android_object.insert("preferEmulator".into(), json!(true));
        android_object
            .entry("isolatedEmulatorPackage")
            .or_insert(json!(true));
        android_object
            .entry("fallbackToEmulator")
            .or_insert(json!(arguments
                .get("fallbackToEmulator")
                .and_then(Value::as_bool)
                .unwrap_or(true)));
    }
    android_object
        .entry("fastFailPhysicalVisualProbe")
        .or_insert(json!(!real_device_required));
    Ok(android)
}

fn preparation_status(prepared: &Value) -> &str {
    prepared
        .pointer("/result/status")
        .and_then(Value::as_str)
        .unwrap_or("IN_PROGRESS")
}

fn selection_source(prepared: &Value) -> &str {
    prepared
        .pointer("/deviceSelection/source")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

fn preparation_elapsed_ms(prepared: &Value) -> Option<u64> {
    let recorded_at = prepared
        .pointer("/result/evidence/0/recordedAt")
        .and_then(Value::as_str)?;
    let started = DateTime::parse_from_rfc3339(recorded_at)
        .ok()?
        .with_timezone(&Utc);
    Some(
        Utc::now()
            .signed_duration_since(started)
            .num_milliseconds()
            .max(0) as u64,
    )
}

fn should_fallback_physical_renderer(
    status: &str,
    emulator_selected: bool,
    real_device_required: bool,
    fallback_enabled: bool,
    elapsed_ms: u64,
    budget_ms: u64,
) -> bool {
    !emulator_selected
        && !real_device_required
        && fallback_enabled
        && (status == "FAILED" || (status == "IN_PROGRESS" && elapsed_ms >= budget_ms))
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
        "emulatorSlotId":result.pointer("/deviceSelection/emulatorSlotId"),
        "fallbackReason":result.pointer("/deviceSelection/fallbackReason"),
        "rendererLease":result.pointer("/rendererLease"),
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
        assert_eq!(
            definition["inputSchema"]["properties"]["physicalDeviceBudgetMs"]["default"],
            30_000
        );
    }

    #[test]
    fn ordinary_android_verification_prefers_an_isolated_emulator() {
        let configured = configure_android_arguments(
            &json!({"android":{"deviceId":"192.168.31.171:5555","preferEmulator":false}}),
            false,
        )
        .unwrap();
        assert_eq!(configured["preferEmulator"], true);
        assert_eq!(configured["isolatedEmulatorPackage"], true);
        assert_eq!(configured["fallbackToEmulator"], true);
        assert!(configured.get("deviceId").is_none());
    }

    #[test]
    fn feedback_triggered_real_device_verification_never_falls_back() {
        let configured = configure_android_arguments(
            &json!({"android":{"deviceId":"emulator-5554","preferEmulator":true}}),
            true,
        )
        .unwrap();
        assert_eq!(configured["fallbackToEmulator"], false);
        assert!(configured.get("preferEmulator").is_none());
        assert!(configured.get("deviceId").is_none());
    }

    #[test]
    fn physical_visual_tasks_fallback_after_failure_or_budget() {
        assert!(should_fallback_physical_renderer(
            "FAILED", false, false, true, 5_000, 60_000
        ));
        assert!(should_fallback_physical_renderer(
            "IN_PROGRESS",
            false,
            false,
            true,
            60_000,
            60_000
        ));
        assert!(!should_fallback_physical_renderer(
            "IN_PROGRESS",
            false,
            false,
            true,
            59_999,
            60_000
        ));
        assert!(!should_fallback_physical_renderer(
            "FAILED", false, true, true, 120_000, 60_000
        ));
        assert!(!should_fallback_physical_renderer(
            "FAILED", true, false, true, 120_000, 60_000
        ));
    }
}
