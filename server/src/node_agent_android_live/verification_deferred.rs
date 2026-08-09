use anyhow::Error;
use serde_json::{json, Value};

pub(super) fn renderer_unavailable(
    error: &Error,
    real_device_required: bool,
    pwa_failure: Option<Value>,
    physical_failure: Option<Value>,
) -> Value {
    let message = bounded_redacted_message(error);
    let code = renderer_error_code(&message);
    json!({
        "status":if real_device_required {
            "REAL_DEVICE_VERIFICATION_DEFERRED"
        } else {
            "VERIFICATION_DEFERRED"
        },
        "verificationMode":if real_device_required {"ANDROID_DEVICE"} else {"ANDROID_EMULATOR"},
        "evidenceLevel":"NONE",
        "completed":false,
        "capabilityGapRequired":false,
        "gapDisposition":"VERIFICATION_DEFERRED",
        "requiresRealDevice":real_device_required,
        "pwaFailure":pwa_failure,
        "physicalFailure":physical_failure,
        "android":{
            "status":"DEFERRED",
            "phase":"DEVICE_SELECTION",
            "retryAfterMs":Value::Null,
            "deviceId":Value::Null,
            "deviceSource":"unavailable",
            "error":{"code":code,"message":message},
        },
        "ANDROID_RENDERER":"UNAVAILABLE",
        "REAL_DEVICE_STATUS":if real_device_required {"REQUIRED_FOLLOWUP"} else {"NOT_REQUIRED"},
        "rendererResourceId":Value::Null,
        "leaseOwner":Value::Null,
        "sourceSha":Value::Null,
        "next":"STOP_AFTER_RENDERER_UNAVAILABLE",
        "recoveryAction":match code {
            "ANDROID_EMULATOR_NOT_INSTALLED" => "INSTALL_ANDROID_EMULATOR",
            "ANDROID_AVD_NOT_CONFIGURED" => "CREATE_ANDROID_AVD",
            "RENDERER_CAPACITY_UNAVAILABLE" => "WAIT_FOR_IDLE_RENDERER",
            _ => "REPAIR_ANDROID_RUNTIME_ENVIRONMENT",
        },
    })
}

fn bounded_redacted_message(error: &Error) -> String {
    crate::node_agent_cli_redaction::redact_text(&error.to_string())
        .chars()
        .take(500)
        .collect()
}

fn renderer_error_code(message: &str) -> &'static str {
    if message.contains("未找到 Android emulator 可执行文件") {
        "ANDROID_EMULATOR_NOT_INSTALLED"
    } else if message.contains("没有已创建的 AVD")
        || message.contains("AVD") && message.contains("不存在")
    {
        "ANDROID_AVD_NOT_CONFIGURED"
    } else if message.contains("模拟器池已满") || message.contains("没有空闲 Android") {
        "RENDERER_CAPACITY_UNAVAILABLE"
    } else {
        "ANDROID_RENDERER_PREPARATION_FAILED"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_emulator_is_structured_non_blocking_deferral() {
        let response = renderer_unavailable(
            &anyhow::anyhow!("未找到 Android emulator 可执行文件"),
            false,
            None,
            None,
        );
        assert_eq!(response["status"], "VERIFICATION_DEFERRED");
        assert_eq!(response["ANDROID_RENDERER"], "UNAVAILABLE");
        assert_eq!(response["REAL_DEVICE_STATUS"], "NOT_REQUIRED");
        assert_eq!(response["capabilityGapRequired"], false);
        assert_eq!(
            response.pointer("/android/error/code").unwrap(),
            "ANDROID_EMULATOR_NOT_INSTALLED"
        );
        assert_eq!(response["recoveryAction"], "INSTALL_ANDROID_EMULATOR");
    }

    #[test]
    fn deferred_renderer_error_is_redacted_and_bounded() {
        let secret = "a".repeat(700);
        let response = renderer_unavailable(
            &anyhow::anyhow!("runtime failed api_key={secret}"),
            true,
            None,
            None,
        );
        let message = response
            .pointer("/android/error/message")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(response["status"], "REAL_DEVICE_VERIFICATION_DEFERRED");
        assert!(message.contains("[REDACTED]"));
        assert!(!message.contains(&secret));
        assert!(message.chars().count() <= 500);
    }
}
