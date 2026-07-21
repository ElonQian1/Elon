use std::time::Duration;

use anyhow::{bail, Result};
use serde_json::{json, Value};

use crate::node_agent_android_inspector::adb_capture::{
    capture_screen_png, wake_device_for_user_interaction,
};
use crate::node_agent_android_inspector::adb_command::{
    run_adb_text, validate_device_id, validate_package_name,
};

use super::broker::LiveUiSession;
use super::frame_artifact::persist_launcher_surface_artifact;
use super::visual_diff::PixelRect;

pub(crate) async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let device_id = arguments
        .get("deviceId")
        .and_then(Value::as_str)
        .unwrap_or(&session.device_id)
        .trim();
    let package_name = arguments
        .get("packageName")
        .and_then(Value::as_str)
        .unwrap_or(&session.package_name)
        .trim();
    validate_device_id(device_id)?;
    validate_package_name(package_name)?;
    let settle_ms = arguments
        .get("settleMs")
        .and_then(Value::as_u64)
        .unwrap_or(900);
    if !(200..=5_000).contains(&settle_ms) {
        bail!("settleMs 必须在 200..5000");
    }
    let icon_rect = arguments
        .get("iconRect")
        .cloned()
        .map(serde_json::from_value::<PixelRect>)
        .transpose()?;

    wake_device_for_user_interaction(device_id).await;
    let home_output = run_adb_text(
        &[
            "-s".to_string(),
            device_id.to_string(),
            "shell".to_string(),
            "am".to_string(),
            "start".to_string(),
            "-W".to_string(),
            "-a".to_string(),
            "android.intent.action.MAIN".to_string(),
            "-c".to_string(),
            "android.intent.category.HOME".to_string(),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(settle_ms)).await;
    let foreground = run_adb_text(
        &[
            "-s".to_string(),
            device_id.to_string(),
            "shell".to_string(),
            "dumpsys".to_string(),
            "window".to_string(),
            "windows".to_string(),
        ],
        Duration::from_secs(6),
        512 * 1024,
    )
    .await?;
    let foreground_line = foreground
        .lines()
        .find(|line| line.contains("mCurrentFocus") || line.contains("mFocusedApp"))
        .unwrap_or_default()
        .trim();
    if foreground_line.contains(package_name) {
        bail!("HOME 启动后前台仍是目标应用，未获得真实 Launcher 表面");
    }
    let png = capture_screen_png(device_id).await?;
    let surface = persist_launcher_surface_artifact(session, &png, None)?;
    let icon = icon_rect
        .map(|rect| persist_launcher_surface_artifact(session, &png, Some(rect)))
        .transpose()?;
    Ok(json!({
        "surface": surface,
        "iconCrop": icon,
        "deviceId": device_id,
        "packageName": package_name,
        "launcherForeground": foreground_line,
        "homeCommand": home_output.lines().take(8).collect::<Vec<_>>().join(" | "),
        "captureKind": "ANDROID_LAUNCHER_SURFACE",
        "maskAwareDiffTool": "ui_get_visual_diff"
    }))
}
