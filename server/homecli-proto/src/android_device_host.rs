use serde::{Deserialize, Serialize};

/// The node relays an allow-listed Android inspector/live REST surface through
/// its authenticated cloud WebSocket session.
pub const CAP_ANDROID_DEVICE_HOST_V1: &str = "android_device_host_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidDeviceHostRequest {
    pub req_id: String,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    pub body_b64: Option<String>,
}
