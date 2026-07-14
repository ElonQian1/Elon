use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;

use super::adb_command::{
    is_wireless_device_id, run_adb, run_adb_text, validate_connect_address, validate_device_id,
    validate_package_name,
};
use super::adb_path::adb_path;
use super::adb_wireless::list_device_inventory;
use super::device_profiles::list_profiles;
use super::png_probe::png_dimensions;
use super::snapshot_artifact::{persist_snapshot, PersistSnapshotInput};
use super::source_map::attach_source_map;
use super::types::{
    AdbStatus, AndroidDevice, AndroidDeviceProfile, CaptureRequest, DeviceUiSnapshot,
    ScreenshotPayload, UiXmlSummary,
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
    let resolved_device_id = resolve_online_device_id(device_id).await?;
    launch_app_exact(&resolved_device_id, package_name).await
}

async fn launch_app_exact(device_id: &str, package_name: &str) -> Result<String> {
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
    let requested_device_id = req.device_id.trim().to_string();
    validate_device_id(&requested_device_id)?;
    let device_id = resolve_online_device_id(&requested_device_id).await?;
    let package_name = req
        .package_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PACKAGE)
        .to_string();
    validate_package_name(&package_name)?;

    if req.launch_app.unwrap_or(false) {
        let _ = launch_app_exact(&device_id, &package_name).await?;
        tokio::time::sleep(Duration::from_millis(700)).await;
    }

    let include_data_url = req.include_screenshot_data_url.unwrap_or(true);
    let (activity_result, screenshot_result, xml_result) = tokio::join!(
        current_activity(&device_id),
        capture_screenshot(&device_id, include_data_url),
        dump_xml(&device_id),
    );
    let activity_name = activity_result.ok();
    let screenshot = screenshot_result.context("ADB 截图失败")?;
    let (xml_raw, mut nodes, xml_error) = match xml_result {
        Ok(xml_raw) => {
            match validate_ui_xml(&xml_raw).and_then(|_| parse_runtime_nodes(&xml_raw)) {
                Ok(nodes) => (xml_raw, nodes, None),
                Err(error) => (xml_raw, Vec::new(), Some(format!("{error:#}"))),
            }
        }
        Err(error) => (String::new(), Vec::new(), Some(format!("{error:#}"))),
    };
    let source_map = attach_source_map(&mut nodes, req.project_root.as_deref());
    let captured_at = Utc::now().to_rfc3339();
    let artifact = persist_snapshot(PersistSnapshotInput {
        device_id: &requested_device_id,
        package_name: Some(&package_name),
        activity_name: activity_name.as_deref(),
        captured_at: &captured_at,
        source_root: source_map.root.as_deref(),
        source_fingerprint: source_map.fingerprint.as_deref(),
        screenshot_png: &screenshot.bytes,
        screenshot_width: screenshot.payload.width,
        screenshot_height: screenshot.payload.height,
        raw_xml: &xml_raw,
        nodes: &nodes,
    })
    .context("持久化真机快照失败")?;
    Ok(DeviceUiSnapshot {
        ok: true,
        device_id: requested_device_id,
        package_name: Some(package_name),
        activity_name,
        captured_at,
        screenshot: Some(screenshot.payload),
        xml: UiXmlSummary {
            node_count: nodes.len(),
            length: xml_raw.len(),
            raw_xml: req.include_raw_xml.unwrap_or(false).then_some(xml_raw),
            error: xml_error,
        },
        nodes,
        source_root: source_map.root,
        source_fingerprint: source_map.fingerprint,
        source_bindings_path: source_map.bindings_path,
        artifact: Some(artifact),
    })
}

struct CapturedScreenshot {
    payload: ScreenshotPayload,
    bytes: Vec<u8>,
}

async fn capture_screenshot(device_id: &str, include_data_url: bool) -> Result<CapturedScreenshot> {
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
    Ok(CapturedScreenshot {
        payload: ScreenshotPayload {
            data_url,
            mime_type: "image/png",
            width,
            height,
            bytes: output.stdout.len(),
        },
        bytes: output.stdout,
    })
}

pub(crate) async fn capture_screen_png(device_id: &str) -> Result<Vec<u8>> {
    let resolved_device_id = resolve_online_device_id(device_id).await?;
    Ok(capture_screenshot(&resolved_device_id, false).await?.bytes)
}

async fn resolve_online_device_id(requested_device_id: &str) -> Result<String> {
    validate_device_id(requested_device_id)?;
    if adb_device_ready(requested_device_id).await {
        return Ok(requested_device_id.to_string());
    }

    let profiles = list_profiles().context("读取 Android 设备档案失败")?;
    let devices = list_device_inventory()
        .await
        .context("刷新 ADB 设备列表失败")?;
    if let Some(device_id) =
        select_equivalent_ready_device(requested_device_id, &devices, &profiles, false)
    {
        return Ok(device_id);
    }

    let reconnect_endpoint = matching_profile(requested_device_id, &devices, &profiles)
        .and_then(|profile| profile.last_endpoint.as_deref())
        .filter(|endpoint| !endpoint.trim().is_empty());
    let reconnect_error = if let Some(endpoint) = reconnect_endpoint {
        match connect_device(endpoint).await {
            Ok(_) => {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let refreshed = list_device_inventory()
                    .await
                    .context("无线 ADB 重连后刷新设备列表失败")?;
                if let Some(device_id) =
                    select_equivalent_ready_device(requested_device_id, &refreshed, &profiles, true)
                {
                    return Ok(device_id);
                }
                None
            }
            Err(error) => Some(format!("；无线重连 {endpoint} 失败：{error:#}")),
        }
    } else {
        None
    };

    bail!(
        "已选择的手机 transport {requested_device_id} 已离线，且没有找到同一台手机的在线 USB/无线连接{}",
        reconnect_error.unwrap_or_default()
    )
}

async fn adb_device_ready(device_id: &str) -> bool {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "get-state".to_string(),
    ];
    run_adb_text(&args, Duration::from_secs(2), 16 * 1024)
        .await
        .is_ok_and(|output| output.trim() == "device")
}

fn select_equivalent_ready_device(
    requested_device_id: &str,
    devices: &[AndroidDevice],
    profiles: &[AndroidDeviceProfile],
    allow_requested: bool,
) -> Option<String> {
    let hardware_serial = requested_hardware_serial(requested_device_id, devices, profiles)?;
    devices
        .iter()
        .filter(|device| device.state == "device")
        .filter(|device| allow_requested || device.serial != requested_device_id)
        .filter(|device| device.hardware_serial.as_deref() == Some(hardware_serial.as_str()))
        .max_by_key(|device| match device.connection_type.as_str() {
            "wireless" => 2,
            "usb" => 1,
            _ => 0,
        })
        .map(|device| device.serial.clone())
}

fn requested_hardware_serial(
    requested_device_id: &str,
    devices: &[AndroidDevice],
    profiles: &[AndroidDeviceProfile],
) -> Option<String> {
    devices
        .iter()
        .find(|device| device.serial == requested_device_id)
        .and_then(|device| device.hardware_serial.clone())
        .or_else(|| {
            matching_profile(requested_device_id, devices, profiles)
                .map(|profile| profile.hardware_serial.clone())
        })
        .or_else(|| {
            (!is_wireless_device_id(requested_device_id)).then(|| requested_device_id.to_string())
        })
}

fn matching_profile<'a>(
    requested_device_id: &str,
    devices: &[AndroidDevice],
    profiles: &'a [AndroidDeviceProfile],
) -> Option<&'a AndroidDeviceProfile> {
    let device_hardware_serial = devices
        .iter()
        .find(|device| device.serial == requested_device_id)
        .and_then(|device| device.hardware_serial.as_deref());
    profiles.iter().find(|profile| {
        profile.hardware_serial == requested_device_id
            || profile.last_endpoint.as_deref() == Some(requested_device_id)
            || device_hardware_serial == Some(profile.hardware_serial.as_str())
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
    let parts: Vec<_> = line.split_whitespace().collect();
    let (state_index, metadata_index, state) = (1..parts.len())
        .find_map(|index| match parts[index] {
            "no" if parts.get(index + 1).copied() == Some("permissions") => {
                Some((index, index + 2, "no permissions"))
            }
            value @ ("device" | "offline" | "unauthorized" | "authorizing" | "connecting"
            | "bootloader" | "recovery" | "sideload" | "rescue" | "host" | "detached") => {
                Some((index, index + 1, value))
            }
            _ => None,
        })
        .or_else(|| parts.get(1).map(|state| (1, 2, *state)))?;
    let serial = parts[..state_index].join(" ");
    let state = state.to_string();
    let mut product = None;
    let mut model = None;
    let mut device = None;
    let mut transport_id = None;
    for part in parts.into_iter().skip(metadata_index) {
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
    let connection_type = if serial.starts_with("emulator-") {
        "emulator"
    } else if is_wireless_device_id(&serial) {
        "wireless"
    } else {
        "usb"
    }
    .to_string();
    Some(AndroidDevice {
        serial,
        state,
        hardware_serial: None,
        connection_type,
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

    #[test]
    fn preserves_mdns_selector_with_collision_suffix() {
        let output = "List of devices attached\n\
adb-ASUJ6R6324002425-ZDy0od (3)._adb-tls-connect._tcp device product:AAK-AN00 model:AAK_AN00 device:HNAAK transport_id:94\n";
        let devices = parse_devices(output);
        assert_eq!(devices.len(), 1);
        assert_eq!(
            devices[0].serial,
            "adb-ASUJ6R6324002425-ZDy0od (3)._adb-tls-connect._tcp"
        );
        assert_eq!(devices[0].state, "device");
        assert_eq!(devices[0].connection_type, "wireless");
        assert_eq!(devices[0].transport_id.as_deref(), Some("94"));
    }

    #[test]
    fn falls_back_from_missing_usb_serial_to_same_hardware_wireless_transport() {
        let devices = vec![AndroidDevice {
            serial: "192.168.31.171:5555".to_string(),
            state: "device".to_string(),
            hardware_serial: Some("e0d909c3".to_string()),
            connection_type: "wireless".to_string(),
            product: Some("shennong".to_string()),
            model: Some("23116PN5BC".to_string()),
            device: Some("shennong".to_string()),
            transport_id: Some("511".to_string()),
        }];
        let profiles = vec![test_profile("e0d909c3", "192.168.31.171:5555")];

        assert_eq!(
            select_equivalent_ready_device("e0d909c3", &devices, &profiles, false).as_deref(),
            Some("192.168.31.171:5555")
        );
    }

    #[test]
    fn falls_back_from_missing_wireless_endpoint_to_same_hardware_usb_transport() {
        let devices = vec![AndroidDevice {
            serial: "e0d909c3".to_string(),
            state: "device".to_string(),
            hardware_serial: Some("e0d909c3".to_string()),
            connection_type: "usb".to_string(),
            product: None,
            model: Some("23116PN5BC".to_string()),
            device: None,
            transport_id: Some("698".to_string()),
        }];
        let profiles = vec![test_profile("e0d909c3", "192.168.31.171:5555")];

        assert_eq!(
            select_equivalent_ready_device("192.168.31.171:5555", &devices, &profiles, false,)
                .as_deref(),
            Some("e0d909c3")
        );
    }

    fn test_profile(hardware_serial: &str, endpoint: &str) -> AndroidDeviceProfile {
        AndroidDeviceProfile {
            id: "adp_test".to_string(),
            display_name: "测试手机".to_string(),
            hardware_serial: hardware_serial.to_string(),
            manufacturer: Some("Xiaomi".to_string()),
            model: Some("23116PN5BC".to_string()),
            android_sdk: Some(35),
            android_release: Some("15".to_string()),
            wireless_mode: "legacy".to_string(),
            paired: false,
            last_endpoint: Some(endpoint.to_string()),
            shared_project_ids: vec!["elon-self".to_string()],
            created_at: "2026-07-14T00:00:00Z".to_string(),
            last_seen_at: "2026-07-14T00:00:00Z".to_string(),
        }
    }
}
