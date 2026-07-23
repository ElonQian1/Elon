use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartLiveSessionRequest {
    pub device_id: String,
    pub package_name: String,
    pub project_root: Option<String>,
    pub lease: Option<crate::node_agent_android_device_lease::AndroidDeviceLeaseProof>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeSocketQuery {
    pub session_id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveSessionView {
    pub id: String,
    pub device_id: String,
    pub package_name: String,
    pub project_root: Option<String>,
    pub device_port: u16,
    pub created_at: String,
    pub connected: bool,
    pub runtime_build_id: Option<String>,
    pub runtime_version: Option<String>,
    pub tree_revision: u64,
    pub node_count: usize,
    pub history_count: usize,
    pub redo_count: usize,
    pub source_proof: Option<LiveSourceProofView>,
    pub last_seen_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveSourceProofView {
    pub generation_revision: String,
    pub origin_workspace_revision: String,
    pub runtime_build_id: Option<String>,
    pub source_parity_loss: f64,
    pub verified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveGeometry {
    pub bounds_in_display_px: LiveRect,
    pub density: f32,
    pub font_scale: f32,
    pub rotation: i32,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LivePropertyValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LivePropertySnapshot {
    #[serde(default)]
    pub effective: Option<LivePropertyValue>,
    #[serde(default)]
    pub measured: Option<LivePropertyValue>,
    pub change_level: String,
    pub commit_mode: String,
    #[serde(default)]
    pub binding: Option<Value>,
    #[serde(default)]
    pub constraints: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveUiNode {
    pub runtime_node_id: String,
    pub definition_id: String,
    #[serde(default)]
    pub instance_key: Option<String>,
    #[serde(default)]
    pub parent_runtime_node_id: Option<String>,
    pub screen_id: String,
    pub kind: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    pub class_name: String,
    #[serde(default)]
    pub source: Option<Value>,
    pub geometry: LiveGeometry,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, LivePropertySnapshot>,
    #[serde(default)]
    pub capabilities: std::collections::BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LivePatchTarget {
    pub scope: String,
    #[serde(default)]
    pub runtime_node_id: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default)]
    pub instance_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LivePatchOperation {
    pub property: String,
    pub value: LivePropertyValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveStylePatch {
    #[serde(default = "protocol_version")]
    pub protocol_version: u32,
    #[serde(default)]
    pub message_type: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub gesture_id: Option<String>,
    #[serde(default)]
    pub sequence: u64,
    #[serde(default)]
    pub base_tree_revision: Option<u64>,
    pub target: LivePatchTarget,
    #[serde(default = "default_true")]
    pub atomic: bool,
    #[serde(default = "default_true")]
    pub ephemeral: bool,
    pub operations: Vec<LivePatchOperation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeWelcome {
    pub protocol_version: u32,
    pub message_type: &'static str,
    pub session_id: String,
    pub accepted: bool,
}

pub(crate) fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}

fn default_true() -> bool {
    true
}

impl LiveStylePatch {
    pub(crate) fn prepare(&mut self, session_id: &str) {
        self.protocol_version = PROTOCOL_VERSION;
        self.message_type = "patch.apply".to_string();
        self.session_id = session_id.to_string();
        if self.request_id.trim().is_empty() {
            self.request_id = format!("req_{}", uuid::Uuid::new_v4().simple());
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(format!(
                "Live UI 协议版本不兼容: {} != {}",
                self.protocol_version, PROTOCOL_VERSION
            ));
        }
        if self.operations.is_empty() || self.operations.len() > 32 {
            return Err("Patch operations 数量必须为 1..32".to_string());
        }
        if self
            .target
            .runtime_node_id
            .as_deref()
            .unwrap_or("")
            .is_empty()
            && self
                .target
                .definition_id
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            return Err("Patch target 必须包含 runtimeNodeId 或 definitionId".to_string());
        }
        if !matches!(
            self.target.scope.trim().to_ascii_uppercase().as_str(),
            "INSTANCE" | "DEFINITION" | "TOKEN"
        ) {
            return Err("Patch target scope 只允许 INSTANCE/DEFINITION/TOKEN".to_string());
        }
        for operation in &self.operations {
            validate_operation(operation)?;
        }
        Ok(())
    }
}

fn validate_operation(operation: &LivePatchOperation) -> Result<(), String> {
    let property = operation.property.trim();
    if !allowed_property(property) {
        return Err(format!("不支持实时修改属性: {property}"));
    }
    let value_type = operation.value.value_type.trim().to_ascii_lowercase();
    match value_type.as_str() {
        "dp" | "sp" | "float" => {
            let value = operation
                .value
                .value
                .as_f64()
                .ok_or_else(|| format!("{property} 必须是数值"))?;
            if !value.is_finite() || !(-10_000.0..=10_000.0).contains(&value) {
                return Err(format!("{property} 数值超出允许范围"));
            }
            if property == "opacity" && !(0.0..=1.0).contains(&value) {
                return Err("opacity 必须在 0..1".to_string());
            }
        }
        "argb" | "color" => {
            let value = operation
                .value
                .value
                .as_str()
                .ok_or_else(|| format!("{property} 必须是颜色字符串"))?;
            if !valid_color(value) {
                return Err(format!("{property} 必须是 #RRGGBB 或 #AARRGGBB"));
            }
        }
        "text" | "enum" | "dimension" => {
            let value = operation
                .value
                .value
                .as_str()
                .ok_or_else(|| format!("{property} 必须是字符串"))?;
            if value.len() > 4_000 {
                return Err(format!("{property} 字符串过长"));
            }
        }
        "bool" => {
            if !operation.value.value.is_boolean() {
                return Err(format!("{property} 必须是布尔值"));
            }
        }
        _ => return Err(format!("不支持值类型: {}", operation.value.value_type)),
    }
    Ok(())
}

fn allowed_property(value: &str) -> bool {
    matches!(
        value,
        "width"
            | "height"
            | "minWidth"
            | "minHeight"
            | "margin.start"
            | "margin.top"
            | "margin.end"
            | "margin.bottom"
            | "padding.start"
            | "padding.top"
            | "padding.end"
            | "padding.bottom"
            | "backgroundColor"
            | "contentColor"
            | "borderColor"
            | "borderWidth"
            | "cornerRadius.all"
            | "text"
            | "textSize"
            | "fontWeight"
            | "lineHeight"
            | "letterSpacing"
            | "opacity"
            | "visibility"
            | "translationX"
            | "translationY"
            | "scaleX"
            | "scaleY"
    )
}

fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}
