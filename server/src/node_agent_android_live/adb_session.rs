use std::time::Duration;

use anyhow::{bail, Result};

use crate::node_agent_android_inspector::adb_command::{
    run_adb_text, validate_device_id, validate_package_name,
};

use super::broker::LiveUiSession;

pub(crate) const DEFAULT_DEVICE_PORT: u16 = 38_917;
const RUNTIME_RECEIVER: &str = "com.elon.uiruntime.view.UiRuntimeControlReceiver";
const START_ACTION: &str = "com.elon.uiruntime.START";
const STOP_ACTION: &str = "com.elon.uiruntime.STOP";

pub(crate) async fn start_runtime(session: &LiveUiSession, host_port: u16) -> Result<String> {
    validate_device_id(&session.device_id)?;
    validate_package_name(&session.package_name)?;
    let device_port = session.device_port;
    run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "reverse".to_string(),
            format!("tcp:{device_port}"),
            format!("tcp:{host_port}"),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await?;
    let component = format!("{}/{}", session.package_name, RUNTIME_RECEIVER);
    let output = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "am".to_string(),
            "broadcast".to_string(),
            "-a".to_string(),
            START_ACTION.to_string(),
            "-n".to_string(),
            component,
            "--es".to_string(),
            "session_id".to_string(),
            session.id.clone(),
            "--es".to_string(),
            "session_token".to_string(),
            session.token.clone(),
            "--ei".to_string(),
            "device_port".to_string(),
            device_port.to_string(),
        ],
        Duration::from_secs(10),
        128 * 1024,
    )
    .await?;
    if output.contains("result=-1") || output.contains("Error:") {
        bail!("启动 Android Live Runtime 失败: {}", output.trim());
    }
    Ok(output.trim().to_string())
}

pub(crate) async fn stop_runtime(session: &LiveUiSession) -> Result<()> {
    let component = format!("{}/{}", session.package_name, RUNTIME_RECEIVER);
    let _ = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "am".to_string(),
            "broadcast".to_string(),
            "-a".to_string(),
            STOP_ACTION.to_string(),
            "-n".to_string(),
            component,
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await;
    let _ = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "reverse".to_string(),
            "--remove".to_string(),
            format!("tcp:{}", session.device_port),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await;
    Ok(())
}
