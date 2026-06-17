use anyhow::{Context, Result};
use std::{collections::HashMap, path::Path};

pub(crate) fn read_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();
    if !path.exists() {
        return Ok(values);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取配置文件 {}", path.display()))?;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(key.to_string(), unquote(value.trim()));
    }
    Ok(values)
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}
