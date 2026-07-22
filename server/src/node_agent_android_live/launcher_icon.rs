use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use super::frame_artifact::persist_launcher_surface_artifact;

const MAX_ICON_BYTES: usize = 8 * 1024 * 1024;

pub(super) async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let package_name = arguments
        .get("packageName")
        .and_then(Value::as_str)
        .unwrap_or(&session.package_name)
        .trim();
    crate::node_agent_android_inspector::adb_command::validate_package_name(package_name)?;
    let icon_size_px = arguments
        .get("iconSizePx")
        .and_then(Value::as_u64)
        .unwrap_or(512);
    if !(48..=1_024).contains(&icon_size_px) {
        bail!("iconSizePx 必须在 48..1024");
    }
    let before = session.view().await;
    if !before.connected {
        bail!(
            "PACKAGE_ICON_RUNTIME_REQUIRED: session={} package={}；默认模式不会降级为 MIUI 逐页模拟",
            session.id,
            package_name
        );
    }
    let response = session
        .request_launcher_icon(package_name, icon_size_px as u32)
        .await?;
    let icon = parse_runtime_icon(&response, package_name)?;
    let artifact = persist_launcher_surface_artifact(session, &icon.bytes, None)?;
    let after = session.view().await;
    if !after.connected || after.id != before.id {
        bail!(
            "PACKAGE_ICON_RUNTIME_SESSION_CHANGED: session={}",
            session.id
        );
    }
    Ok(json!({
        "surface": &artifact,
        "iconRect": artifact.rect,
        "iconCrop": &artifact,
        "deviceId": session.device_id,
        "packageName": package_name,
        "appLabel": icon.app_label,
        "captureKind": "ANDROID_PACKAGE_ICON_RENDER",
        "captureMode": "PACKAGE_ICON",
        "locator": {
            "source": icon.source,
            "adaptive": icon.adaptive,
            "renderMode": icon.render_mode,
            "bounded": true,
            "pagesInspected": 0,
            "candidateLaunches": 0,
            "oemAutomationUsed": false,
            "legacyFallbackUsed": false,
            "evidence": [
                "Runtime used public LauncherApps/PackageManager APIs for packageName",
                "No HOME transition, workspace page scan, app drawer scan, tap, or swipe"
            ]
        },
        "recovery": {
            "runtimeWasConnected": before.connected,
            "sameRuntimeSessionId": session.id,
            "runtimeConnectedAfterCapture": after.connected,
            "runtimeRestored": true,
            "foregroundChanged": false
        },
        "adaptiveMaskTool": "ui_render_android_launcher_masks",
        "oemFinalPresentationMode": "OEM_FIXED_POSITION"
    }))
}

struct RuntimeIcon {
    bytes: Vec<u8>,
    app_label: String,
    source: String,
    adaptive: bool,
    render_mode: String,
}

fn parse_runtime_icon(value: &Value, expected_package: &str) -> Result<RuntimeIcon> {
    if value.get("messageType").and_then(Value::as_str) != Some("icon.snapshot") {
        bail!("Android Runtime 未返回 icon.snapshot");
    }
    if value.get("packageName").and_then(Value::as_str) != Some(expected_package) {
        bail!("Android Runtime 图标 packageName 不匹配");
    }
    let data_url = value
        .get("dataUrl")
        .and_then(Value::as_str)
        .context("Android Runtime 图标缺少 dataUrl")?;
    let payload = data_url
        .strip_prefix("data:image/png;base64,")
        .context("Android Runtime 图标必须是 PNG dataUrl")?;
    let bytes = B64
        .decode(payload)
        .context("Android Runtime 图标 Base64 无效")?;
    if bytes.is_empty() || bytes.len() > MAX_ICON_BYTES {
        bail!("Android Runtime 图标大小必须在 1..8MiB");
    }
    let source = value
        .get("source")
        .and_then(Value::as_str)
        .filter(|source| matches!(*source, "LAUNCHER_APPS" | "PACKAGE_MANAGER"))
        .context("Android Runtime 图标 source 无效")?
        .to_string();
    Ok(RuntimeIcon {
        bytes,
        app_label: value
            .get("appLabel")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        source,
        adaptive: value
            .get("adaptive")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        render_mode: value
            .get("renderMode")
            .and_then(Value::as_str)
            .unwrap_or("DRAWABLE")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_launcher_apps_png_for_exact_package() {
        let payload = B64.encode(b"png");
        let icon = parse_runtime_icon(
            &json!({
                "messageType":"icon.snapshot",
                "packageName":"com.example.app",
                "dataUrl":format!("data:image/png;base64,{payload}"),
                "source":"LAUNCHER_APPS",
                "adaptive":true,
                "renderMode":"UNMASKED_ADAPTIVE_LAYERS"
            }),
            "com.example.app",
        )
        .unwrap();
        assert_eq!(icon.bytes, b"png");
        assert!(icon.adaptive);
    }

    #[test]
    fn rejects_package_substitution() {
        let payload = B64.encode(b"png");
        assert!(parse_runtime_icon(
            &json!({
                "messageType":"icon.snapshot",
                "packageName":"com.evil.app",
                "dataUrl":format!("data:image/png;base64,{payload}"),
                "source":"PACKAGE_MANAGER"
            }),
            "com.example.app",
        )
        .is_err());
    }
}
