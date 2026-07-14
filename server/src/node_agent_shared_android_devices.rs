use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::{
    node_agent_android_inspector::{
        adb_wireless::reconnect_devices,
        device_profiles::{merge_shared_profiles, SharedDeviceProfileInput},
        types::ReconnectRequest,
    },
    NodeRuntime,
};

const SYNC_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Debug, Deserialize)]
struct SharedDevicesEnvelope {
    #[serde(default)]
    devices: Vec<CloudSharedDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudSharedDevice {
    project_id: String,
    hardware_serial: String,
    display_name: String,
    manufacturer: Option<String>,
    model: Option<String>,
    android_sdk: Option<u32>,
    android_release: Option<String>,
    last_endpoint: String,
    wireless_mode: String,
}

pub(crate) fn spawn(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        loop {
            match sync_once(&runtime).await {
                Ok(SyncOutcome::SkippedMissingToken) => {
                    debug!("尚未登录，跳过项目共享 Android 设备同步")
                }
                Ok(SyncOutcome::Synced(count)) if count > 0 => {
                    info!("已同步 {count} 个项目共享 Android 测试手机档案")
                }
                Ok(SyncOutcome::Synced(_)) => {}
                Err(error) => warn!("同步项目共享 Android 设备失败: {error:#}"),
            }
            tokio::time::sleep(SYNC_INTERVAL).await;
        }
    });
}

#[derive(Debug, PartialEq, Eq)]
enum SyncOutcome {
    SkippedMissingToken,
    Synced(usize),
}

async fn sync_once(runtime: &NodeRuntime) -> Result<SyncOutcome> {
    let Some(token) = runtime.user_token().await else {
        return Ok(SyncOutcome::SkippedMissingToken);
    };
    let client = crate::node_agent_cloud_net::direct_cloud_client(Duration::from_secs(12))
        .context("创建共享设备同步 HTTP 客户端失败")?;
    let url = format!(
        "{}/api/me/modules/ui-tuner/shared-android-devices",
        runtime.cloud_http_url().trim_end_matches('/')
    );
    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context("请求共享设备列表失败")?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("共享设备接口返回 HTTP {}", status.as_u16());
    }
    let body: SharedDevicesEnvelope = response.json().await.context("解析共享设备响应失败")?;
    let inputs: Vec<_> = body
        .devices
        .into_iter()
        .filter_map(shared_input_if_safe)
        .collect();
    let merged = merge_shared_profiles(&inputs)?;
    if !merged.is_empty() {
        let _ = reconnect_devices(ReconnectRequest { profile_id: None }).await;
    }
    Ok(SyncOutcome::Synced(merged.len()))
}

fn shared_input_if_safe(device: CloudSharedDevice) -> Option<SharedDeviceProfileInput> {
    if !valid_hardware_serial(&device.hardware_serial)
        || !private_adb_endpoint(&device.last_endpoint)
    {
        warn!(
            project_id = %device.project_id,
            "忽略云端返回的不安全 Android 设备档案"
        );
        return None;
    }
    Some(SharedDeviceProfileInput {
        project_id: device.project_id,
        hardware_serial: device.hardware_serial,
        display_name: device.display_name,
        manufacturer: device.manufacturer,
        model: device.model,
        android_sdk: device.android_sdk,
        android_release: device.android_release,
        last_endpoint: device.last_endpoint,
        wireless_mode: normalize_wireless_mode(&device.wireless_mode).to_string(),
    })
}

fn private_adb_endpoint(value: &str) -> bool {
    let Ok(address) = SocketAddr::from_str(value.trim()) else {
        return false;
    };
    if address.port() == 0 {
        return false;
    }
    match address.ip() {
        IpAddr::V4(ip) => ip.is_private(),
        IpAddr::V6(ip) => ip.is_unique_local(),
    }
}

fn valid_hardware_serial(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn normalize_wireless_mode(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "legacy" => "legacy",
        "tls" => "tls",
        "manual" => "manual",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_private_socket_endpoints() {
        assert!(private_adb_endpoint("192.168.31.171:5555"));
        assert!(private_adb_endpoint("10.0.0.8:37123"));
        assert!(!private_adb_endpoint("8.8.8.8:5555"));
        assert!(!private_adb_endpoint("127.0.0.1:5555"));
        assert!(!private_adb_endpoint("phone.local:5555"));
    }

    #[test]
    fn validates_serial_and_normalizes_mode() {
        assert!(valid_hardware_serial("e0d909c3"));
        assert!(!valid_hardware_serial("bad serial"));
        assert_eq!(normalize_wireless_mode("LEGACY"), "legacy");
        assert_eq!(normalize_wireless_mode("surprise"), "unknown");
    }
}
