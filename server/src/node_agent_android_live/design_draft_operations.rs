use std::path::Path;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum DraftOperation {
    SetStyle {
        property: String,
        before: Option<String>,
        after: String,
        state: Option<String>,
        breakpoint: Option<String>,
    },
    SetText {
        before: Option<String>,
        after: String,
    },
    ReplaceAsset {
        before_asset: Option<String>,
        after_asset: String,
        alt: Option<String>,
    },
    SetVariant {
        name: String,
        value: String,
    },
    InsertNode {
        node_kind: String,
        position: String,
        reference_selector: Option<String>,
    },
    RemoveNode {},
    MoveNode {
        position: String,
        reference_selector: Option<String>,
    },
    SetResponsiveStyle {
        property: String,
        after: String,
        min_width: Option<u32>,
        max_width: Option<u32>,
    },
}

pub(super) fn normalize_operations(values: Vec<DraftOperation>) -> Result<Vec<DraftOperation>> {
    if values.len() > 64 {
        bail!("DraftOperation 超过 64 项");
    }
    values.into_iter().map(normalize).collect()
}

pub(super) fn capability_view(operations: &[DraftOperation], platforms: &[String]) -> Value {
    let mut entries = Vec::new();
    let mut live_preview_supported = true;
    let mut requires_source_writeback = false;
    for (index, operation) in operations.iter().enumerate() {
        for platform in platforms {
            let capability = operation.capability(platform);
            live_preview_supported &= capability.status == "LIVE_PREVIEW";
            requires_source_writeback |= capability.status == "SOURCE_HANDOFF";
            entries.push(json!({
                "operationIndex":index,
                "operationType":operation.kind(),
                "platform":platform,
                "status":capability.status,
                "adapter":capability.adapter,
                "reason":capability.reason
            }));
        }
    }
    json!({
        "schema":"elon.ui-design-operation-capabilities.v1",
        "livePreviewSupported":!operations.is_empty() && live_preview_supported,
        "requiresSourceWriteback":requires_source_writeback,
        "entries":entries
    })
}

pub(super) fn live_style_patches(operations: &[DraftOperation]) -> Vec<Value> {
    operations
        .iter()
        .filter_map(|operation| match operation {
            DraftOperation::SetStyle {
                property, after, ..
            } => Some(json!({"property":property,"value":after})),
            _ => None,
        })
        .collect()
}

pub(super) fn live_style_patches_from_value(value: &Value) -> Result<Vec<Value>> {
    let operations = serde_json::from_value::<Vec<DraftOperation>>(value.clone())
        .map_err(|error| anyhow::anyhow!("DraftOperation 无效: {error}"))?;
    Ok(live_style_patches(&operations))
}

pub(super) fn operations_schema() -> Value {
    json!({
        "type":"array","maxItems":64,"items":{"oneOf":[
            operation_schema("SET_STYLE", json!({
                "property":{"type":"string","minLength":1,"maxLength":64},
                "before":{"type":"string","maxLength":500},
                "after":{"type":"string","minLength":1,"maxLength":500},
                "state":{"type":"string","maxLength":64},
                "breakpoint":{"type":"string","maxLength":64}
            }), &["property","after"]),
            operation_schema("SET_TEXT", json!({
                "before":{"type":"string","maxLength":10000},
                "after":{"type":"string","maxLength":10000}
            }), &["after"]),
            operation_schema("REPLACE_ASSET", json!({
                "beforeAsset":{"type":"string","maxLength":1000},
                "afterAsset":{"type":"string","minLength":1,"maxLength":1000},
                "alt":{"type":"string","maxLength":500}
            }), &["afterAsset"]),
            operation_schema("SET_VARIANT", json!({
                "name":{"type":"string","minLength":1,"maxLength":120},
                "value":{"type":"string","minLength":1,"maxLength":240}
            }), &["name","value"]),
            operation_schema("INSERT_NODE", json!({
                "nodeKind":{"type":"string","minLength":1,"maxLength":80},
                "position":{"enum":["before","after","first-child","last-child"]},
                "referenceSelector":{"type":"string","maxLength":1000}
            }), &["nodeKind","position"]),
            operation_schema("REMOVE_NODE", json!({}), &[]),
            operation_schema("MOVE_NODE", json!({
                "position":{"enum":["before","after","first-child","last-child"]},
                "referenceSelector":{"type":"string","maxLength":1000}
            }), &["position"]),
            operation_schema("SET_RESPONSIVE_STYLE", json!({
                "property":{"type":"string","minLength":1,"maxLength":64},
                "after":{"type":"string","minLength":1,"maxLength":500},
                "minWidth":{"type":"integer","minimum":0,"maximum":10000},
                "maxWidth":{"type":"integer","minimum":0,"maximum":10000}
            }), &["property","after"])
        ]}
    })
}

struct Capability {
    status: &'static str,
    adapter: &'static str,
    reason: &'static str,
}

impl DraftOperation {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::SetStyle { .. } => "SET_STYLE",
            Self::SetText { .. } => "SET_TEXT",
            Self::ReplaceAsset { .. } => "REPLACE_ASSET",
            Self::SetVariant { .. } => "SET_VARIANT",
            Self::InsertNode { .. } => "INSERT_NODE",
            Self::RemoveNode {} => "REMOVE_NODE",
            Self::MoveNode { .. } => "MOVE_NODE",
            Self::SetResponsiveStyle { .. } => "SET_RESPONSIVE_STYLE",
        }
    }

    fn capability(&self, platform: &str) -> Capability {
        match (self, platform) {
            (Self::SetStyle { .. }, "web" | "pwa" | "tauri") => Capability {
                status: "LIVE_PREVIEW",
                adapter: "BROWSER_INLINE_STYLE",
                reason: "可在隔离浏览器会话中可逆预览",
            },
            (Self::SetResponsiveStyle { .. }, "android") => Capability {
                status: "UNSUPPORTED",
                adapter: "NONE",
                reason: "Android 不使用 CSS viewport breakpoint",
            },
            (_, "web" | "pwa" | "tauri" | "android") => Capability {
                status: "SOURCE_HANDOFF",
                adapter: "AI_SOURCE_WRITEBACK",
                reason: "已具备机器意图契约，需源码适配器审查并写回",
            },
            _ => Capability {
                status: "UNSUPPORTED",
                adapter: "NONE",
                reason: "未知目标平台",
            },
        }
    }
}

fn normalize(operation: DraftOperation) -> Result<DraftOperation> {
    Ok(match operation {
        DraftOperation::SetStyle {
            property,
            before,
            after,
            state,
            breakpoint,
        } => DraftOperation::SetStyle {
            property: clean_property(&property)?,
            before: before
                .map(|value| clean_text(&value, 500, "before", false))
                .transpose()?,
            after: clean_style_value(&after)?,
            state: state
                .map(|value| clean_token(&value, 64, "state"))
                .transpose()?,
            breakpoint: breakpoint
                .map(|value| clean_token(&value, 64, "breakpoint"))
                .transpose()?,
        },
        DraftOperation::SetText { before, after } => DraftOperation::SetText {
            before: before
                .map(|value| clean_text(&value, 10_000, "before", true))
                .transpose()?,
            after: clean_text(&after, 10_000, "after", true)?,
        },
        DraftOperation::ReplaceAsset {
            before_asset,
            after_asset,
            alt,
        } => DraftOperation::ReplaceAsset {
            before_asset: before_asset.map(|value| clean_asset(&value)).transpose()?,
            after_asset: clean_asset(&after_asset)?,
            alt: alt
                .map(|value| clean_text(&value, 500, "alt", true))
                .transpose()?,
        },
        DraftOperation::SetVariant { name, value } => DraftOperation::SetVariant {
            name: clean_token(&name, 120, "variant name")?,
            value: clean_text(&value, 240, "variant value", false)?,
        },
        DraftOperation::InsertNode {
            node_kind,
            position,
            reference_selector,
        } => DraftOperation::InsertNode {
            node_kind: clean_token(&node_kind, 80, "nodeKind")?,
            position: clean_position(&position)?,
            reference_selector: clean_selector(reference_selector)?,
        },
        DraftOperation::RemoveNode {} => DraftOperation::RemoveNode {},
        DraftOperation::MoveNode {
            position,
            reference_selector,
        } => DraftOperation::MoveNode {
            position: clean_position(&position)?,
            reference_selector: clean_selector(reference_selector)?,
        },
        DraftOperation::SetResponsiveStyle {
            property,
            after,
            min_width,
            max_width,
        } => {
            if min_width.is_some_and(|value| value > 10_000)
                || max_width.is_some_and(|value| value > 10_000)
                || min_width.zip(max_width).is_some_and(|(min, max)| min > max)
            {
                bail!("响应式宽度范围无效");
            }
            DraftOperation::SetResponsiveStyle {
                property: clean_property(&property)?,
                after: clean_style_value(&after)?,
                min_width,
                max_width,
            }
        }
    })
}

fn clean_property(value: &str) -> Result<String> {
    let value = clean_text(value, 64, "property", false)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("DraftOperation property 包含不安全字符");
    }
    Ok(value)
}

fn clean_style_value(value: &str) -> Result<String> {
    let value = clean_text(value, 500, "style value", false)?;
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("url(") || lowered.contains("javascript:") {
        bail!("style value 不允许外部 URL 或脚本");
    }
    Ok(value)
}

fn clean_asset(value: &str) -> Result<String> {
    let value = clean_text(value, 1_000, "asset", false)?.replace('\\', "/");
    let path = Path::new(&value);
    if path.is_absolute()
        || value.contains("://")
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("asset 必须是安全的项目相对路径");
    }
    Ok(value)
}

fn clean_token(value: &str, max: usize, field: &str) -> Result<String> {
    let value = clean_text(value, max, field, false)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        bail!("{field} 包含不安全字符");
    }
    Ok(value)
}

fn clean_position(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !matches!(
        value.as_str(),
        "before" | "after" | "first-child" | "last-child"
    ) {
        bail!("position 无效");
    }
    Ok(value)
}

fn clean_selector(value: Option<String>) -> Result<Option<String>> {
    value
        .map(|value| clean_text(&value, 1_000, "referenceSelector", false))
        .transpose()
}

fn clean_text(value: &str, max: usize, field: &str, allow_empty: bool) -> Result<String> {
    let value = if allow_empty { value } else { value.trim() };
    if (!allow_empty && value.is_empty()) || value.chars().count() > max || value.contains('\0') {
        bail!("{field} 为空、过长或包含 NUL");
    }
    Ok(value.to_string())
}

fn operation_schema(kind: &str, mut properties: Value, required: &[&str]) -> Value {
    properties["type"] = json!({"const":kind});
    let mut required = required.to_vec();
    required.insert(0, "type");
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}
