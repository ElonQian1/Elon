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
