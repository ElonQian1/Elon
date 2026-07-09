use std::time::Duration;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;

use super::adb_command::{
    run_adb, run_adb_text, validate_connect_address, validate_device_id, validate_package_name,
};
use super::adb_path::adb_path;
use super::png_probe::png_dimensions;
use super::source_map::attach_source_map;
use super::types::{
    AdbStatus, AndroidDevice, CaptureRequest, DeviceUiSnapshot, ScreenshotPayload, UiXmlSummary,
};
use super::xml_parser::{parse_runtime_nodes, validate_ui_xml};

const DEFAULT_PACKAGE: &str = "com.elon.app";
const MAX_TEXT: usize = 2 * 1024 * 1024;
const MAX_SCREENSHOT: usize = 8 * 1024 * 1024;

pub(crate) async fn adb_status() -> AdbStatus {
    let adb_path = adb_path();
    let args = vec!["version".to_string()];
    match run_adb_text(&args, Duration::from_secs(5), 64 * 1024).await {
        Ok(version) => AdbStatus {
            available: true,
            adb_path,
            version: Some(version.lines().next().unwrap_or("").trim().to_string()),
            error: None,
        },
        Err(error) => AdbStatus {
            available: false,
            adb_path,
            version: None,
            error: Some(error.to_string()),
        },
    }
}

pub(crate) async fn list_devices() -> Result<Vec<AndroidDevice>> {
    let args = vec!["devices".to_string(), "-l".to_string()];
    let output = run_adb_text(&args, Duration::from_secs(5), 256 * 1024).await?;
    Ok(parse_devices(&output))
}

pub(crate) async fn connect_device(address: &str) -> Result<String> {
    validate_connect_address(address)?;
    let args = vec!["connect".to_string(), address.trim().to_string()];
    run_adb_text(&args, Duration::from_secs(10), 64 * 1024).await
}

pub(crate) async fn launch_app(device_id: &str, package_name: &str) -> Result<String> {
    validate_device_id(device_id)?;
    validate_package_name(package_name)?;
    let args = vec![
        "-s".to_string(),
        device_id.trim().to_string(),
        "shell".to_string(),
        "monkey".to_string(),
        "-p".to_string(),
        package_name.trim().to_string(),
        "-c".to_string(),
        "android.intent.category.LAUNCHER".to_string(),
        "1".to_string(),
    ];
    run_adb_text(&args, Duration::from_secs(8), 128 * 1024).await
}

pub(crate) async fn capture_snapshot(req: CaptureRequest) -> Result<DeviceUiSnapshot> {
    let device_id = req.device_id.trim().to_string();
    validate_device_id(&device_id)?;
    let package_name = req
        .package_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PACKAGE)
        .to_string();
    validate_package_name(&package_name)?;

    if req.launch_app.unwrap_or(false) {
        let _ = launch_app(&device_id, &package_name).await?;
        tokio::time::sleep(Duration::from_millis(700)).await;
    }

    let activity_name = current_activity(&device_id).await.ok();
    let screenshot =
        capture_screenshot(&device_id, req.include_screenshot_data_url.unwrap_or(true))
            .await
            .context("ADB 截图失败")?;
    let xml_raw = dump_xml(&device_id).await.context("ADB XML dump 失败")?;
    validate_ui_xml(&xml_raw)?;
    let mut nodes = parse_runtime_nodes(&xml_raw)?;
    let source_root = attach_source_map(&mut nodes, req.project_root.as_deref());
    Ok(DeviceUiSnapshot {
        ok: true,
        device_id,
        package_name: Some(package_name),
        activity_name,
        captured_at: Utc::now().to_rfc3339(),
        screenshot: Some(screenshot),
        xml: UiXmlSummary {
            node_count: nodes.len(),
            length: xml_raw.len(),
            raw_xml: req.include_raw_xml.unwrap_or(false).then_some(xml_raw),
        },
        nodes,
        source_root,
    })
}

async fn capture_screenshot(device_id: &str, include_data_url: bool) -> Result<ScreenshotPayload> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "exec-out".to_string(),
        "screencap".to_string(),
        "-p".to_string(),
    ];
    let output = run_adb(&args, Duration::from_secs(10), MAX_SCREENSHOT).await?;
    let (width, height) = png_dimensions(&output.stdout)?;
    let data_url =
        include_data_url.then(|| format!("data:image/png;base64,{}", B64.encode(&output.stdout)));
    Ok(ScreenshotPayload {
        data_url,
        mime_type: "image/png",
        width,
        height,
        bytes: output.stdout.len(),
    })
}

async fn dump_xml(device_id: &str) -> Result<String> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "exec-out".to_string(),
        "uiautomator".to_string(),
        "dump".to_string(),
        "/dev/stdout".to_string(),
    ];
    let output = run_adb_text(&args, Duration::from_secs(8), MAX_TEXT).await?;
    Ok(output)
}

async fn current_activity(device_id: &str) -> Result<String> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "shell".to_string(),
        "dumpsys".to_string(),
        "window".to_string(),
    ];
    let output = run_adb_text(&args, Duration::from_secs(4), 512 * 1024).await?;
    for line in output.lines() {
        if let Some(value) = line.split("mCurrentFocus=").nth(1) {
            return Ok(value.trim().to_string());
        }
    }
    anyhow::bail!("未找到当前窗口");
}

fn parse_devices(output: &str) -> Vec<AndroidDevice> {
    output
        .lines()
        .skip(1)
        .filter_map(parse_device_line)
        .collect()
}

fn parse_device_line(line: &str) -> Option<AndroidDevice> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split_whitespace();
    let serial = parts.next()?.to_string();
    let state = parts.next().unwrap_or("unknown").to_string();
    let mut product = None;
    let mut model = None;
    let mut device = None;
    let mut transport_id = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("product:") {
            product = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("model:") {
            model = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("device:") {
            device = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("transport_id:") {
            transport_id = Some(value.to_string());
        }
    }
    Some(AndroidDevice {
        serial,
        state,
        product,
        model,
        device,
        transport_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adb_devices_long_output() {
        let output = "List of devices attached\n\
e0d909c3               device product:shennong model:23116PN5BC device:shennong transport_id:1\n";
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].serial, "e0d909c3");
        assert_eq!(devices[0].model.as_deref(), Some("23116PN5BC"));
    }
}
