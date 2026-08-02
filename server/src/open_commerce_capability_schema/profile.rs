use anyhow::{bail, Result};
use serde_json::{Map, Value};
use std::collections::HashSet;

use super::{property_path, ContractSide};

const MAX_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_DEPTH: usize = 16;
const SUPPORTED_KEYWORDS: &[&str] = &[
    "$schema",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "type",
    "properties",
    "required",
    "additionalProperties",
    "minProperties",
    "maxProperties",
    "items",
    "minItems",
    "maxItems",
    "minLength",
    "maxLength",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "enum",
    "const",
    "format",
];

pub(super) fn validate_schema(schema: &Value, side: ContractSide) -> Result<()> {
    let encoded = serde_json::to_vec(schema)?;
    if encoded.len() > MAX_SCHEMA_BYTES {
        bail!("{}不能超过 64 KiB", side.label());
    }
    let root = schema
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{}必须是 JSON object", side.label()))?;
    validate_schema_node(root, "$", 0, side)?;
    if matches!(side, ContractSide::Input)
        && root
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|schema_type| schema_type != "object")
    {
        bail!("输入 schema 根节点 type 必须是 object");
    }
    Ok(())
}

fn validate_schema_node(
    schema: &Map<String, Value>,
    path: &str,
    depth: usize,
    side: ContractSide,
) -> Result<()> {
    if depth > MAX_SCHEMA_DEPTH {
        bail!("{}嵌套不能超过 {} 层", side.label(), MAX_SCHEMA_DEPTH);
    }
    for keyword in schema.keys() {
        if !SUPPORTED_KEYWORDS.contains(&keyword.as_str()) {
            bail!(
                "{}在 {} 使用了不支持的关键字 {}",
                side.label(),
                path,
                keyword
            );
        }
    }
    validate_type(schema, path, side)?;
    validate_properties(schema, path, depth, side)?;
    validate_required(schema, path, side)?;

    if schema
        .get("additionalProperties")
        .is_some_and(|value| !value.is_boolean())
    {
        bail!(
            "{}在 {} 仅支持布尔型 additionalProperties",
            side.label(),
            path
        );
    }
    if let Some(items) = schema.get("items") {
        let items = items
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("{}在 {} 的 items 必须是对象", side.label(), path))?;
        validate_schema_node(items, &format!("{path}[]"), depth + 1, side)?;
    }

    validate_non_negative_pair(schema, path, side, "minProperties", "maxProperties")?;
    validate_non_negative_pair(schema, path, side, "minItems", "maxItems")?;
    validate_non_negative_pair(schema, path, side, "minLength", "maxLength")?;
    validate_number_pair(schema, path, side, "minimum", "maximum")?;
    validate_number_pair(schema, path, side, "exclusiveMinimum", "exclusiveMaximum")?;
    if schema
        .get("enum")
        .is_some_and(|values| values.as_array().is_none_or(Vec::is_empty))
    {
        bail!("{}在 {} 的 enum 必须是非空数组", side.label(), path);
    }
    if let Some(format) = schema.get("format") {
        let format = format
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("{}在 {} 的 format 必须是字符串", side.label(), path))?;
        if !matches!(format, "uuid" | "date-time" | "uri") {
            bail!("{}在 {} 使用了不支持的 format", side.label(), path);
        }
    }
    Ok(())
}

fn validate_type(schema: &Map<String, Value>, path: &str, side: ContractSide) -> Result<()> {
    let Some(value) = schema.get("type") else {
        return Ok(());
    };
    let schema_type = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{}在 {} 的 type 必须是字符串", side.label(), path))?;
    if !matches!(
        schema_type,
        "object" | "array" | "string" | "integer" | "number" | "boolean" | "null"
    ) {
        bail!("{}在 {} 声明了不支持的 type", side.label(), path);
    }
    Ok(())
}

fn validate_properties(
    schema: &Map<String, Value>,
    path: &str,
    depth: usize,
    side: ContractSide,
) -> Result<()> {
    let Some(properties) = schema.get("properties") else {
        return Ok(());
    };
    let properties = properties
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{}在 {} 的 properties 必须是对象", side.label(), path))?;
    for (name, child) in properties {
        if name.is_empty() || name.chars().count() > 128 {
            bail!("{}在 {} 包含无效属性名", side.label(), path);
        }
        let child = child.as_object().ok_or_else(|| {
            anyhow::anyhow!("{}在 {}.{} 的属性定义必须是对象", side.label(), path, name)
        })?;
        validate_schema_node(child, &property_path(path, name), depth + 1, side)?;
    }
    Ok(())
}

fn validate_required(schema: &Map<String, Value>, path: &str, side: ContractSide) -> Result<()> {
    let Some(required) = schema.get("required") else {
        return Ok(());
    };
    let required = required.as_array().ok_or_else(|| {
        anyhow::anyhow!("{}在 {} 的 required 必须是字符串数组", side.label(), path)
    })?;
    let mut seen = HashSet::new();
    for item in required {
        let name = item
            .as_str()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{}在 {} 的 required 必须是非空字符串数组",
                    side.label(),
                    path
                )
            })?;
        if !seen.insert(name) {
            bail!("{}在 {} 的 required 包含重复字段", side.label(), path);
        }
    }
    Ok(())
}

fn validate_non_negative_pair(
    schema: &Map<String, Value>,
    path: &str,
    side: ContractSide,
    minimum: &str,
    maximum: &str,
) -> Result<()> {
    let min = schema
        .get(minimum)
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                anyhow::anyhow!("{}在 {} 的 {} 必须是非负整数", side.label(), path, minimum)
            })
        })
        .transpose()?;
    let max = schema
        .get(maximum)
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                anyhow::anyhow!("{}在 {} 的 {} 必须是非负整数", side.label(), path, maximum)
            })
        })
        .transpose()?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        bail!(
            "{}在 {} 的 {} 不能大于 {}",
            side.label(),
            path,
            minimum,
            maximum
        );
    }
    Ok(())
}

fn validate_number_pair(
    schema: &Map<String, Value>,
    path: &str,
    side: ContractSide,
    minimum: &str,
    maximum: &str,
) -> Result<()> {
    let min = schema
        .get(minimum)
        .map(|value| {
            value.as_f64().ok_or_else(|| {
                anyhow::anyhow!("{}在 {} 的 {} 必须是数字", side.label(), path, minimum)
            })
        })
        .transpose()?;
    let max = schema
        .get(maximum)
        .map(|value| {
            value.as_f64().ok_or_else(|| {
                anyhow::anyhow!("{}在 {} 的 {} 必须是数字", side.label(), path, maximum)
            })
        })
        .transpose()?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        bail!(
            "{}在 {} 的 {} 不能大于 {}",
            side.label(),
            path,
            minimum,
            maximum
        );
    }
    Ok(())
}
