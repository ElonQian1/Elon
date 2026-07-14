use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::future::join_all;

use super::adb_capture::{adb_status, connect_device, list_devices};
use super::adb_command::{
    is_mdns_tls_device_id, run_adb_text, run_adb_text_with_stdin, validate_connect_address,
    validate_device_id, validate_pairing_code,
};
use super::device_profiles::{
    forget_profile, list_profiles, mark_paired, remember_connection, upsert_profile,
};
use super::types::{
    AdbMdnsService, AndroidDevice, AndroidDeviceIdentity, AndroidDeviceProfile,
    AndroidDeviceProfileView, AndroidWirelessStatus, EnableTcpIpRequest, PairDeviceRequest,
    ReconnectRequest, RegisterDeviceRequest,
};

const TEXT_LIMIT: usize = 256 * 1024;

pub(crate) async fn register_device(req: RegisterDeviceRequest) -> Result<AndroidDeviceProfile> {
    validate_device_id(&req.device_id)?;
    let identity = probe_identity(&req.device_id).await?;
    let endpoint = req
        .device_id
        .contains(':')
        .then_some(req.device_id.as_str());
    upsert_profile(&identity, req.display_name.as_deref(), endpoint)
}

pub(crate) async fn pair_device(req: PairDeviceRequest) -> Result<(String, AndroidWirelessStatus)> {
    validate_connect_address(&req.pairing_address)?;
    validate_pairing_code(&req.pairing_code)?;
    let args = vec!["pair".to_string(), req.pairing_address.trim().to_string()];
    let output = run_adb_text_with_stdin(
        &args,
        &req.pairing_code,
        Duration::from_secs(20),
        64 * 1024,
        "无线 ADB 配对",
    )
    .await?;
    mark_paired(req.profile_id.as_deref())?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let status = reconnect_devices(ReconnectRequest {
        profile_id: req.profile_id,
    })
    .await?;
    Ok((output.trim().to_string(), status))
}

pub(crate) async fn connect_and_remember(
    address: &str,
    profile_id: Option<&str>,
) -> Result<String> {
    let output = connect_device(address).await?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    if let Ok(identity) = probe_identity(address).await {
        let paired = profile_id
            .and_then(|id| {
                list_profiles()
                    .ok()?
                    .into_iter()
                    .find(|profile| profile.id == id)
            })
            .is_some_and(|profile| profile.paired);
        let mode = connection_mode(address, paired);
        remember_connection(&identity.hardware_serial, address, mode)?;
    }
    Ok(output)
}

pub(crate) async fn enable_tcpip(
    req: EnableTcpIpRequest,
) -> Result<(String, AndroidWirelessStatus)> {
    validate_device_id(&req.device_id)?;
    let port = req.port.unwrap_or(5555);
    if port == 0 {
        bail!("无线 ADB 端口无效");
    }
    let identity = probe_identity(&req.device_id).await?;
    let ip = device_wifi_ip(&req.device_id)
        .await?
        .context("未找到手机 Wi-Fi 地址，请确认手机已连接 Wi-Fi")?;
    let args = vec![
        "-s".to_string(),
        req.device_id.clone(),
        "tcpip".to_string(),
        port.to_string(),
    ];
    let tcpip_output = run_adb_text(&args, Duration::from_secs(10), 64 * 1024).await?;
    tokio::time::sleep(Duration::from_millis(900)).await;
    let endpoint = format!("{ip}:{port}");
    let connect_output = connect_device(&endpoint).await?;
    remember_connection(&identity.hardware_serial, &endpoint, "legacy")?;
    if req.profile_id.is_none() {
        let _ = upsert_profile(&identity, None, Some(&endpoint))?;
    }
    let status = wireless_status().await?;
    Ok((
        format!("{}\n{}", tcpip_output.trim(), connect_output.trim()),
        status,
    ))
}

pub(crate) async fn reconnect_devices(req: ReconnectRequest) -> Result<AndroidWirelessStatus> {
    let profiles = list_profiles()?;
    let requested = req.profile_id.as_deref();
    let allowed_serials: Vec<&str> = profiles
        .iter()
        .filter(|profile| requested.is_none_or(|id| profile.id == id))
        .map(|profile| profile.hardware_serial.as_str())
        .collect();
    let mut endpoints: Vec<String> = profiles
        .iter()
        .filter(|profile| requested.is_none_or(|id| profile.id == id))
        .filter_map(|profile| profile.last_endpoint.clone())
        .collect();
    for service in discover_mdns_services().await.unwrap_or_default() {
        if matches!(
            service.service_type.as_str(),
            "_adb-tls-connect._tcp" | "_adb._tcp"
        ) && allowed_serials
            .iter()
            .any(|serial| mdns_service_matches_hardware_serial(&service.name, serial))
        {
            endpoints.push(service.address);
        }
    }
    endpoints.sort();
    endpoints.dedup();
    join_all(endpoints.iter().map(|endpoint| connect_device(endpoint))).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    wireless_status().await
}

pub(crate) async fn wireless_status() -> Result<AndroidWirelessStatus> {
    let devices = list_device_inventory().await?;
    let profiles = list_profiles()?;
    let mdns_services = discover_mdns_services().await.unwrap_or_default();
    let mut connections: HashMap<String, &AndroidDevice> = HashMap::new();
    for device in devices.iter().filter(|device| device.state == "device") {
        if let Some(hardware_serial) = device.hardware_serial.as_ref() {
            let replace = connections
                .get(hardware_serial)
                .is_none_or(|current| connection_priority(device) > connection_priority(current));
            if replace {
                connections.insert(hardware_serial.clone(), device);
            }
        }
    }
    let profile_views = profiles
        .into_iter()
        .map(|mut profile| -> Result<AndroidDeviceProfileView> {
            let connected_device = connections.get(&profile.hardware_serial).copied();
            let connected_device_id = connected_device.map(|device| device.serial.clone());
            if let Some(endpoint) = connected_device
                .and_then(|device| wireless_endpoint_for_device(device, &mdns_services))
            {
                let mode = if connected_device
                    .is_some_and(|device| is_mdns_tls_device_id(&device.serial))
                {
                    "tls"
                } else {
                    connection_mode(&endpoint, profile.paired)
                };
                if profile.last_endpoint.as_deref() != Some(&endpoint)
                    || profile.wireless_mode != mode
                    || (mode == "tls" && !profile.paired)
                {
                    remember_connection(&profile.hardware_serial, &endpoint, mode)
                        .context("保存 Android 无线设备档案失败")?;
                    profile.last_endpoint = Some(endpoint);
                    profile.wireless_mode = mode.to_string();
                }
                if mode == "tls" {
                    profile.paired = true;
                }
            }
            let connection_state = match connected_device {
                Some(device) if device.connection_type == "wireless" => "connected_wireless",
                Some(_) => "connected_usb",
                None if profile.paired => "paired_offline",
                None => "offline",
            }
            .to_string();
            Ok(AndroidDeviceProfileView {
                profile,
                connection_state,
                connected_device_id,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(AndroidWirelessStatus {
        ok: true,
        adb: adb_status().await,
        devices,
        profiles: profile_views,
        mdns_services,
    })
}

pub(crate) async fn list_device_inventory() -> Result<Vec<AndroidDevice>> {
    let mut devices = list_devices().await?;
    for device in devices.iter_mut().filter(|device| device.state == "device") {
        device.hardware_serial = getprop(&device.serial, "ro.serialno")
            .await
            .ok()
            .and_then(optional_text);
    }
    Ok(devices)
}

pub(crate) fn forget_device(profile_id: &str) -> Result<bool> {
    forget_profile(profile_id)
}

async fn probe_identity(device_id: &str) -> Result<AndroidDeviceIdentity> {
    validate_device_id(device_id)?;
    let hardware_serial = getprop(device_id, "ro.serialno").await?;
    if hardware_serial.is_empty() {
        bail!("手机没有返回稳定序列号，无法建立设备档案");
    }
    let sdk = getprop(device_id, "ro.build.version.sdk")
        .await?
        .parse::<u32>()
        .ok();
    Ok(AndroidDeviceIdentity {
        hardware_serial,
        manufacturer: optional_text(getprop(device_id, "ro.product.manufacturer").await?),
        model: optional_text(getprop(device_id, "ro.product.model").await?),
        android_sdk: sdk,
        android_release: optional_text(getprop(device_id, "ro.build.version.release").await?),
    })
}

async fn getprop(device_id: &str, property: &str) -> Result<String> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "shell".to_string(),
        "getprop".to_string(),
        property.to_string(),
    ];
    Ok(run_adb_text(&args, Duration::from_secs(4), 64 * 1024)
        .await?
        .trim()
        .to_string())
}

async fn device_wifi_ip(device_id: &str) -> Result<Option<String>> {
    let args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "shell".to_string(),
        "ip".to_string(),
        "-f".to_string(),
        "inet".to_string(),
        "addr".to_string(),
        "show".to_string(),
        "wlan0".to_string(),
    ];
    let output = run_adb_text(&args, Duration::from_secs(5), TEXT_LIMIT).await?;
    Ok(parse_wifi_ip(&output))
}

async fn discover_mdns_services() -> Result<Vec<AdbMdnsService>> {
    let args = vec!["mdns".to_string(), "services".to_string()];
    let output = run_adb_text(&args, Duration::from_secs(5), TEXT_LIMIT).await?;
    Ok(parse_mdns_services(&output))
}

fn parse_mdns_services(output: &str) -> Vec<AdbMdnsService> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let address = line.split_whitespace().last()?;
            let address_index = line.rfind(address)?;
            let before_address = line[..address_index].trim_end();
            let service_type = before_address.split_whitespace().last()?;
            let service_index = before_address.rfind(service_type)?;
            let name = before_address[..service_index].trim_end();
            if name.is_empty() || !service_type.starts_with("_adb") || !address.contains(':') {
                return None;
            }
            Some(AdbMdnsService {
                name: name.to_string(),
                service_type: service_type.to_string(),
                address: address.to_string(),
            })
        })
        .collect()
}

fn wireless_endpoint_for_device(
    device: &AndroidDevice,
    mdns_services: &[AdbMdnsService],
) -> Option<String> {
    if device.connection_type != "wireless" {
        return None;
    }
    if device.serial.contains(':') {
        return Some(device.serial.clone());
    }
    mdns_services
        .iter()
        .find(|service| {
            service.service_type == "_adb-tls-connect._tcp"
                && device
                    .serial
                    .strip_suffix(&service.service_type)
                    .and_then(|name| name.strip_suffix('.'))
                    == Some(service.name.as_str())
        })
        .map(|service| service.address.clone())
}

fn connection_priority(device: &AndroidDevice) -> u8 {
    if is_mdns_tls_device_id(&device.serial) {
        2
    } else if device.connection_type == "wireless" {
        1
    } else {
        0
    }
}

fn mdns_service_matches_hardware_serial(service_name: &str, hardware_serial: &str) -> bool {
    let base_name = service_name
        .rsplit_once(" (")
        .and_then(|(base, suffix)| {
            suffix
                .strip_suffix(')')
                .filter(|number| !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit()))
                .map(|_| base)
        })
        .unwrap_or(service_name);
    let exact = format!("adb-{hardware_serial}");
    base_name == exact || base_name.starts_with(&format!("{exact}-"))
}

fn parse_wifi_ip(output: &str) -> Option<String> {
    for line in output.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if let Some(index) = parts.iter().position(|part| *part == "inet") {
            if let Some(ip) = parts.get(index + 1) {
                return Some(ip.split('/').next().unwrap_or(ip).to_string());
            }
        }
    }
    None
}

fn optional_text(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn connection_mode(address: &str, paired: bool) -> &'static str {
    if address.trim().ends_with(":5555") {
        "legacy"
    } else if paired {
        "tls"
    } else {
        "manual"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mdns_connect_services() {
        let output = "List of discovered mdns services\n\
adb-ASUJ6R6324002425-ZDy0od (3)\t_adb-tls-connect._tcp\t192.168.31.83:36115\n\
adb-ASUJ6R6324002425-ZDy0od (2)\t_adb-tls-connect._tcp\t192.168.31.83:39593\n\
adb-ASUJ6R6324002425-ZDy0od\t_adb-tls-connect._tcp\t192.168.31.83:43079\n";
        let services = parse_mdns_services(output);
        assert_eq!(services.len(), 3);
        assert_eq!(services[0].name, "adb-ASUJ6R6324002425-ZDy0od (3)");
        assert_eq!(services[0].address, "192.168.31.83:36115");
        assert_eq!(services[1].name, "adb-ASUJ6R6324002425-ZDy0od (2)");

        let device = AndroidDevice {
            serial: "adb-ASUJ6R6324002425-ZDy0od (3)._adb-tls-connect._tcp".to_string(),
            state: "device".to_string(),
            hardware_serial: Some("ASUJ6R6324002425".to_string()),
            connection_type: "wireless".to_string(),
            product: None,
            model: None,
            device: None,
            transport_id: Some("94".to_string()),
        };
        assert_eq!(
            wireless_endpoint_for_device(&device, &services).as_deref(),
            Some("192.168.31.83:36115")
        );

        let mut endpoint_device = device.clone();
        endpoint_device.serial = "192.168.31.83:36115".to_string();
        assert!(connection_priority(&device) > connection_priority(&endpoint_device));
        assert!(mdns_service_matches_hardware_serial(
            "adb-ASUJ6R6324002425-ZDy0od (3)",
            "ASUJ6R6324002425"
        ));
        assert!(!mdns_service_matches_hardware_serial(
            "adb-ASUJ6R6324002425-ZDy0od (3)",
            "ASUJ6R6"
        ));
    }

    #[test]
    fn parses_wifi_source_address() {
        let address = "24: wlan0: <UP> mtu 1500\n    inet 192.168.1.88/24 brd 192.168.1.255 scope global wlan0\n";
        assert_eq!(parse_wifi_ip(address).as_deref(), Some("192.168.1.88"));
    }

    #[test]
    fn classifies_legacy_and_tls_connections_without_profile_id_guessing() {
        assert_eq!(connection_mode("192.168.31.171:5555", false), "legacy");
        assert_eq!(connection_mode("192.168.31.171:5555", true), "legacy");
        assert_eq!(connection_mode("192.168.31.171:37123", false), "manual");
        assert_eq!(connection_mode("192.168.31.171:37123", true), "tls");
    }
}
