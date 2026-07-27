use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
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
    let auto_start_emulator = arguments
        .get("autoStartEmulator")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let fallback_to_emulator = arguments
        .get("fallbackToEmulator")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let device_selection = super::emulator_start::select_or_start(
        requested_device_id,
        auto_start_emulator,
        fallback_to_emulator,
    )
    .await?;
    let device_id = device_selection.device_id.clone();
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
    let requested_debug_application_id_suffix = arguments
        .get("debugApplicationIdSuffix")
        .and_then(Value::as_str)
        .unwrap_or(".uitest")
        .to_string();
    let debug_application_id_suffix = requested_debug_application_id_suffix;
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
                isolated_emulator_package: arguments
                    .get("isolatedEmulatorPackage")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                lkg_enabled: arguments
                    .get("lkgEnabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                candidate: arguments
                    .get("candidate")
                    .cloned()
                    .map(serde_json::from_value)
                    .transpose()
                    .context("candidate 格式无效")?,
                lease: None,
                integration_plan: None,
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
    Ok(json!({
        "result": progress,
        "nextPhase": next_phase,
        "deviceSelection": device_selection,
    }))
}

pub(super) async fn debug_integration_status(
    broker: &Arc<LiveUiBroker>,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let session = broker.session(session_id).await?;
    let project_root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("UI 设计会话未绑定项目目录"))?;
    let device_id = arguments
        .get("deviceId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("ui_get_debug_integration_status 缺少 deviceId"))?;
    let project_id = arguments
        .get("projectId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(project_root);
    let device_identity = if device_id == session.device_id {
        session.device_identity.as_str()
    } else {
        device_id
    };
    Ok(json!({ "status": broker.debug_integration.status_for(
        project_root, project_id, device_identity
    )? }))
}
