use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdbStatus {
    pub available: bool,
    pub adb_path: String,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidDevice {
    pub serial: String,
    pub state: String,
    pub product: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectRequest {
    pub address: String,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterDeviceRequest {
    pub device_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairDeviceRequest {
    pub pairing_address: String,
    pub pairing_code: String,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReconnectRequest {
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnableTcpIpRequest {
    pub device_id: String,
    pub profile_id: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForgetDeviceRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidDeviceProfile {
    pub id: String,
    pub display_name: String,
    pub hardware_serial: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub android_sdk: Option<u32>,
    pub android_release: Option<String>,
    pub wireless_mode: String,
    pub paired: bool,
    pub last_endpoint: Option<String>,
    pub created_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidDeviceProfileView {
    #[serde(flatten)]
    pub profile: AndroidDeviceProfile,
    pub connection_state: String,
    pub connected_device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdbMdnsService {
    pub name: String,
    pub service_type: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWirelessStatus {
    pub ok: bool,
    pub adb: AdbStatus,
    pub devices: Vec<AndroidDevice>,
    pub profiles: Vec<AndroidDeviceProfileView>,
    pub mdns_services: Vec<AdbMdnsService>,
}

#[derive(Debug, Clone)]
pub(crate) struct AndroidDeviceIdentity {
    pub hardware_serial: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub android_sdk: Option<u32>,
    pub android_release: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchAppRequest {
    pub device_id: String,
    pub package_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureRequest {
    pub device_id: String,
    pub package_name: Option<String>,
    pub include_raw_xml: Option<bool>,
    pub include_screenshot_data_url: Option<bool>,
    pub launch_app: Option<bool>,
    pub project_root: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceUiSnapshot {
    pub ok: bool,
    pub device_id: String,
    pub package_name: Option<String>,
    pub activity_name: Option<String>,
    pub captured_at: String,
    pub screenshot: Option<ScreenshotPayload>,
    pub xml: UiXmlSummary,
    pub nodes: Vec<RuntimeUiNode>,
    pub source_root: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenshotPayload {
    pub data_url: Option<String>,
    pub mime_type: &'static str,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiXmlSummary {
    pub node_count: usize,
    pub length: usize,
    pub raw_xml: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BoundsRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeUiNode {
    pub id: String,
    pub depth: usize,
    pub index_path: Vec<u32>,
    pub xpath: String,
    pub text: String,
    pub content_desc: String,
    pub resource_id: Option<String>,
    pub package_name: Option<String>,
    pub class_name: Option<String>,
    pub bounds: BoundsRect,
    pub clickable: bool,
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
    pub scrollable: bool,
    pub checkable: bool,
    pub checked: bool,
    pub selected: bool,
    pub password: bool,
    pub visible: bool,
    pub source: Option<SourceMapEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceMapEntry {
    pub file: String,
    pub line: Option<usize>,
    pub token: String,
    pub confidence: f32,
    pub reason: String,
}
