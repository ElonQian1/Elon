use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::broker::LiveUiBroker;
use super::build_verify::PrepareDebugRuntimeRequest;

pub(super) async fn prepare_debug_runtime(
    broker: &Arc<LiveUiBroker>,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let bootstrap_session = broker.session(session_id).await?;
    let project_root = bootstrap_session
        .project_root
        .clone()
        .ok_or_else(|| anyhow!("UI 设计会话未绑定项目目录"))?;
    let requested_device_id = arguments
        .get("deviceId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let device_id = match requested_device_id {
        Some(device_id) => device_id,
        None => {
            let devices =
                crate::node_agent_android_inspector::adb_wireless::list_device_inventory().await?;
            devices
                .iter()
                .find(|device| device.state == "device" && device.serial.starts_with("emulator-"))
                .or_else(|| devices.iter().find(|device| device.state == "device"))
                .map(|device| device.serial.clone())
                .ok_or_else(|| anyhow!("没有可用 Android 设备或模拟器"))?
        }
    };
    let profile = super::design_bootstrap::project_profile(&bootstrap_session)?;
    let base_package_name = arguments
        .get("basePackageName")
        .and_then(Value::as_str)
        .or_else(|| {
            profile
                .pointer("/android/applicationId")
                .and_then(Value::as_str)
        })
        .ok_or_else(|| {
            anyhow!("UI Profile 未识别 Android applicationId；请显式提供 basePackageName")
        })?
        .to_string();
    let debug_application_id_suffix = arguments
        .get("debugApplicationIdSuffix")
        .and_then(Value::as_str)
        .unwrap_or(".uitest")
        .to_string();
    let restart = arguments
        .get("restart")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let progress = broker
        .debug_runtime_preparations
        .poll_or_start(
            broker.clone(),
            PrepareDebugRuntimeRequest {
                device_id,
                base_package_name,
                project_root,
                debug_application_id_suffix,
                lease: None,
            },
            crate::node_agent_admin_open::admin_port_from_env(),
            restart,
        )
        .await?;
    let next_phase = match progress.status.as_str() {
        "COMPLETED" => "LIVE",
        "FAILED" => "RETRY_WITH_RESTART",
        _ => "POLL_PREPARATION",
    };
    Ok(json!({ "result": progress, "nextPhase": next_phase }))
}
