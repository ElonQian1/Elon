// server/src/project_landing/normalize_helpers.rs
//! normalize 辅助函数，从 normalize.rs 提取。
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use super::{MANIFEST_PATHS, MAX_ITEMS, MAX_LONG_TEXT, MAX_SHORT_TEXT, MAX_URL, MAX_VARIANTS};


pub(super) fn aggregate_variant_status(variants: &[Value], base_status: &str) -> String {
    let mut available_count = 0;
    let mut configured_missing_count = 0;
    for variant in variants.iter().filter_map(Value::as_object) {
        let status = variant
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("planned");
        if matches!(status, "available" | "external") {
            available_count += 1;
        } else if matches!(
            status,
            "needs_configuration" | "not_deployed" | "unavailable" | "pending"
        ) {
            configured_missing_count += 1;
        }
    }
    if available_count == variants.len() && available_count > 0 {
        return "available".to_string();
    }
    if available_count > 0 {
        return "partial".to_string();
    }
    if configured_missing_count > 0 {
        return "needs_configuration".to_string();
    }
    if base_status == "third_party" {
        return "third_party".to_string();
    }
    "planned".to_string()
}

pub(super) fn normalize_named_urls(value: Option<&Value>) -> Vec<Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    let values: Vec<&Value> = match value {
        Value::Array(items) => items.iter().take(MAX_ITEMS).collect(),
        Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    values
        .into_iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            let url = first_url(object, &["url", "href", "link", "src"])?;
            let mut output = Map::new();
            output.insert("url".to_string(), Value::String(url));
            insert_string(
                &mut output,
                "label",
                first_string(object, &["label", "name", "title", "alt"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "kind",
                first_string(object, &["kind", "type"], MAX_SHORT_TEXT),
            );
            insert_string(
                &mut output,
                "note",
                first_string(object, &["note", "description"], MAX_LONG_TEXT),
            );
            Some(Value::Object(output))
        })
        .collect()
}

pub(super) fn insert_text_array(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(items) = value.and_then(text_array_from_value) {
        if !items.is_empty() {
            output.insert(key.to_string(), Value::Array(items));
        }
    }
}

pub(super) fn text_array_from_value(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => Some(
            items
                .iter()
                .take(MAX_ITEMS)
                .filter_map(|item| {
                    if let Some(text) = clean_text_value(item, MAX_LONG_TEXT) {
                        return Some(Value::String(text));
                    }
                    let object = item.as_object()?;
                    first_string(object, &["title", "text", "label", "name"], MAX_LONG_TEXT)
                        .map(Value::String)
                })
                .collect(),
        ),
        Value::String(_) => {
            clean_text_value(value, MAX_LONG_TEXT).map(|text| vec![Value::String(text)])
        }
        _ => None,
    }
}

pub(super) fn insert_string(output: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        output.insert(key.to_string(), Value::String(value));
    }
}

pub(super) fn insert_bool(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value.and_then(Value::as_bool) {
        output.insert(key.to_string(), Value::Bool(value));
    }
}

pub(super) fn first_value<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| object.get(*key))
}

pub(super) fn first_array<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    first_value(object, keys).filter(|value| value.is_array() || value.is_string())
}

pub(super) fn first_string(
    object: &Map<String, Value>,
    keys: &[&str],
    max_chars: usize,
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| clean_text_value(value, max_chars))
}

pub(super) fn first_url(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| clean_text_value(value, MAX_URL))
        .and_then(|value| clean_url(&value))
}

pub(super) fn clean_text_value(value: &Value, max_chars: usize) -> Option<String> {
    let raw = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    let cleaned = raw
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .collect::<String>();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trim_chars(trimmed, max_chars))
}

pub(super) fn trim_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let trimmed: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{trimmed}...")
    } else {
        trimmed
    }
}

pub(super) fn clean_url(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    let safe = lower.starts_with("https://")
        || lower.starts_with("http://")
        || (value.starts_with('/') && !value.starts_with("//"));
    safe.then(|| trim_chars(value, MAX_URL))
}

pub(super) fn normalize_platform(value: Option<&str>) -> Option<String> {
    let raw = value?.trim().to_ascii_lowercase();
    let compact = raw
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .collect::<String>();
    let platform = match compact.as_str() {
        "android" | "apk" | "androidapk" => "android",
        "windows" | "win" | "window" | "windowsclient" => "windows",
        "web" | "browser" | "h5" | "website" => "web",
        "ios" | "iphone" | "ipad" => "ios",
        "macos" | "mac" | "osx" | "darwin" => "macos",
        "linux" => "linux",
        _ => return None,
    };
    Some(platform.to_string())
}

pub(super) fn normalize_status(
    value: Option<&str>,
    url: Option<&str>,
    manifest_url: Option<&str>,
) -> String {
    let explicit = value
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match explicit.as_str() {
        "available"
        | "external"
        | "unavailable"
        | "coming_soon"
        | "needs_configuration"
        | "not_deployed"
        | "third_party"
        | "partial"
        | "planned"
        | "pending" => explicit,
        _ if url.is_some() => "available".to_string(),
        _ if manifest_url.is_some() => "pending".to_string(),
        _ => "planned".to_string(),
    }
}

pub(super) fn source_only(relative_path: &str, status: &str, error: Option<String>) -> Value {
    json!({
        "source": source_value(relative_path, status, error),
    })
}

pub(super) fn source_value(relative_path: &str, status: &str, error: Option<String>) -> Value {
    let mut source = Map::new();
    source.insert(
        "mode".to_string(),
        Value::String("workspace_manifest".to_string()),
    );
    source.insert("status".to_string(), Value::String(status.to_string()));
    source.insert("file".to_string(), Value::String(relative_path.to_string()));
    if let Some(error) =
        error.and_then(|value| clean_text_value(&Value::String(value), MAX_LONG_TEXT))
    {
        source.insert("health_error".to_string(), Value::String(error));
    }
    Value::Object(source)
}

#[allow(dead_code)]
pub(super) fn manifest_path(workspace: &Path, relative_path: &str) -> PathBuf {
    workspace.join(relative_path)
}
