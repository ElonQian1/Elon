//! Bounded JSON Schema profile used by open-commerce capability contracts.

use anyhow::Result;
use serde_json::{Map, Value};
use std::fmt;

const MAX_INSTANCE_DEPTH: usize = 32;

mod profile;

#[derive(Debug, Clone, Copy)]
pub(super) enum ContractSide {
    Input,
    Output,
}

impl ContractSide {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Input => "输入 schema",
            Self::Output => "输出 schema",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CapabilitySchemaViolation {
    pub code: &'static str,
    pub path: String,
    pub side: &'static str,
}

impl CapabilitySchemaViolation {
    fn new(side: ContractSide, path: &str, code: &'static str) -> Self {
        Self {
            code,
            path: path.to_string(),
            side: side.label(),
        }
    }
}

impl fmt::Display for CapabilitySchemaViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "调用{}不符合能力契约（path={}, code={}）",
            self.side, self.path, self.code
        )
    }
}

impl std::error::Error for CapabilitySchemaViolation {}

pub(crate) fn validate_input_schema(schema: &Value) -> Result<()> {
    profile::validate_schema(schema, ContractSide::Input)
}

pub(crate) fn validate_output_schema(schema: &Value) -> Result<()> {
    profile::validate_schema(schema, ContractSide::Output)
}

pub(crate) fn validate_input(
    schema: &Value,
    value: &Value,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    validate_instance_contract(schema, value, ContractSide::Input)
}

pub(crate) fn validate_output(
    schema: &Value,
    value: &Value,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    validate_instance_contract(schema, value, ContractSide::Output)
}

fn validate_instance_contract(
    schema: &Value,
    value: &Value,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    if profile::validate_schema(schema, side).is_err() {
        return Err(CapabilitySchemaViolation::new(side, "$", "invalid_schema"));
    }
    let schema = schema
        .as_object()
        .ok_or_else(|| CapabilitySchemaViolation::new(side, "$", "invalid_schema"))?;
    validate_value(schema, value, "$", 0, side)
}

fn validate_value(
    schema: &Map<String, Value>,
    value: &Value,
    path: &str,
    depth: usize,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    if depth > MAX_INSTANCE_DEPTH {
        return Err(CapabilitySchemaViolation::new(side, path, "max_depth"));
    }
    if schema
        .get("const")
        .is_some_and(|expected| expected != value)
    {
        return Err(CapabilitySchemaViolation::new(side, path, "const"));
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.contains(value) {
            return Err(CapabilitySchemaViolation::new(side, path, "enum"));
        }
    }
    if let Some(schema_type) = schema.get("type").and_then(Value::as_str) {
        if !matches_type(value, schema_type) {
            return Err(CapabilitySchemaViolation::new(side, path, "type"));
        }
    }

    if let Some(object) = value.as_object() {
        validate_object(schema, object, path, depth, side)?;
    }
    if let Some(array) = value.as_array() {
        validate_array(schema, array, path, depth, side)?;
    }
    if let Some(text) = value.as_str() {
        validate_string(schema, text, path, side)?;
    }
    if value.is_number() {
        validate_number(schema, value, path, side)?;
    }
    Ok(())
}

fn validate_object(
    schema: &Map<String, Value>,
    object: &Map<String, Value>,
    path: &str,
    depth: usize,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    check_usize_bounds(
        schema,
        object.len(),
        path,
        side,
        "minProperties",
        "maxProperties",
    )?;
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for name in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(name) {
                return Err(CapabilitySchemaViolation::new(
                    side,
                    &property_path(path, name),
                    "required",
                ));
            }
        }
    }
    let properties = schema.get("properties").and_then(Value::as_object);
    for (name, child_value) in object {
        match properties.and_then(|properties| properties.get(name)) {
            Some(child_schema) => {
                let child_schema = child_schema
                    .as_object()
                    .ok_or_else(|| CapabilitySchemaViolation::new(side, path, "invalid_schema"))?;
                validate_value(
                    child_schema,
                    child_value,
                    &property_path(path, name),
                    depth + 1,
                    side,
                )?;
            }
            None if schema.get("additionalProperties") == Some(&Value::Bool(false)) => {
                return Err(CapabilitySchemaViolation::new(
                    side,
                    &property_path(path, name),
                    "additionalProperties",
                ));
            }
            None => {}
        }
    }
    Ok(())
}

fn validate_array(
    schema: &Map<String, Value>,
    array: &[Value],
    path: &str,
    depth: usize,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    check_usize_bounds(schema, array.len(), path, side, "minItems", "maxItems")?;
    if let Some(items) = schema.get("items").and_then(Value::as_object) {
        for (index, item) in array.iter().enumerate() {
            validate_value(items, item, &format!("{path}[{index}]"), depth + 1, side)?;
        }
    }
    Ok(())
}

fn validate_string(
    schema: &Map<String, Value>,
    text: &str,
    path: &str,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    check_usize_bounds(
        schema,
        text.chars().count(),
        path,
        side,
        "minLength",
        "maxLength",
    )?;
    let valid_format = match schema.get("format").and_then(Value::as_str) {
        Some("uuid") => uuid::Uuid::parse_str(text).is_ok(),
        Some("date-time") => chrono::DateTime::parse_from_rfc3339(text).is_ok(),
        Some("uri") => reqwest::Url::parse(text).is_ok(),
        _ => true,
    };
    if !valid_format {
        return Err(CapabilitySchemaViolation::new(side, path, "format"));
    }
    Ok(())
}

fn validate_number(
    schema: &Map<String, Value>,
    value: &Value,
    path: &str,
    side: ContractSide,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    let Some(number) = value.as_f64() else {
        return Err(CapabilitySchemaViolation::new(side, path, "number_range"));
    };
    for (keyword, passes) in [
        (
            "minimum",
            schema
                .get("minimum")
                .and_then(Value::as_f64)
                .is_none_or(|min| number >= min),
        ),
        (
            "maximum",
            schema
                .get("maximum")
                .and_then(Value::as_f64)
                .is_none_or(|max| number <= max),
        ),
        (
            "exclusiveMinimum",
            schema
                .get("exclusiveMinimum")
                .and_then(Value::as_f64)
                .is_none_or(|min| number > min),
        ),
        (
            "exclusiveMaximum",
            schema
                .get("exclusiveMaximum")
                .and_then(Value::as_f64)
                .is_none_or(|max| number < max),
        ),
    ] {
        if !passes {
            return Err(CapabilitySchemaViolation::new(side, path, keyword));
        }
    }
    Ok(())
}

fn check_usize_bounds(
    schema: &Map<String, Value>,
    actual: usize,
    path: &str,
    side: ContractSide,
    minimum: &'static str,
    maximum: &'static str,
) -> std::result::Result<(), CapabilitySchemaViolation> {
    if schema
        .get(minimum)
        .and_then(Value::as_u64)
        .is_some_and(|min| actual < min as usize)
    {
        return Err(CapabilitySchemaViolation::new(side, path, minimum));
    }
    if schema
        .get(maximum)
        .and_then(Value::as_u64)
        .is_some_and(|max| actual > max as usize)
    {
        return Err(CapabilitySchemaViolation::new(side, path, maximum));
    }
    Ok(())
}

fn matches_type(value: &Value, schema_type: &str) -> bool {
    match schema_type {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

pub(super) fn property_path(parent: &str, name: &str) -> String {
    format!(
        "{parent}.{}",
        name.replace('\\', "\\\\").replace('.', "\\.")
    )
}
