use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MANIFEST_PATHS: [&str; 2] = [".elon/project-landing.json", ".elon/landing.json"];
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_SHORT_TEXT: usize = 160;
const MAX_LONG_TEXT: usize = 1200;
const MAX_URL: usize = 2048;
const MAX_ITEMS: usize = 12;
const MAX_VARIANTS: usize = 8;

pub(crate) fn load_workspace_landing(workspace: &Path) -> Option<Value> {
    for relative_path in MANIFEST_PATHS {
        let manifest_path = workspace.join(relative_path);
        if manifest_path.is_file() {
            return Some(load_manifest_file(&manifest_path, relative_path));
        }
    }
    None
}

#[allow(dead_code)]
pub(crate) fn normalize_landing_snapshot(value: &Value) -> Option<Value> {
    let mut landing = normalize_manifest(value.clone())?;
    if !has_display_content(&Value::Object(landing.clone())) {
        return None;
    }
    landing.insert(
        "source".to_string(),
        json!({
            "mode": "node_agent_snapshot",
            "status": "available",
        }),
    );
    Some(Value::Object(landing))
}

#[allow(dead_code)]
pub(crate) fn has_display_content(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.iter().any(|(key, value)| {
        key != "source"
            && match value {
                Value::Null => false,
                Value::String(value) => !value.trim().is_empty(),
                Value::Array(value) => !value.is_empty(),
                Value::Object(value) => !value.is_empty(),
                Value::Bool(_) | Value::Number(_) => true,
            }
    })
}

fn load_manifest_file(path: &Path, relative_path: &str) -> Value {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    if metadata.len() > MAX_MANIFEST_BYTES {
        return source_only(
            relative_path,
            "invalid",
            Some(format!("manifest 超过 {} KB", MAX_MANIFEST_BYTES / 1024)),
        );
    }

    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => return source_only(relative_path, "invalid", Some(error.to_string())),
    };
    let mut landing = match normalize_manifest(parsed) {
        Some(landing) => landing,
        None => {
            return source_only(
                relative_path,
                "invalid",
                Some("manifest 根节点必须是对象".to_string()),
            )
        }
    };
    landing.insert(
        "source".to_string(),
        source_value(relative_path, "available", None),
    );
    Value::Object(landing)
}


mod normalize;
use self::normalize::{normalize_manifest, source_only, source_value};
