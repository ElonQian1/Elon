use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) fn current_valid_codex_home_env() -> Option<String> {
    let value = std::env::var("CODEX_HOME").ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let home = PathBuf::from(&value);
    if !home.exists() {
        return None;
    }
    if path_in_managed_vault(&home) && managed_lease_expired(&home) {
        std::env::remove_var("CODEX_HOME");
        return None;
    }
    Some(value)
}

fn managed_lease_expired(home: &Path) -> bool {
    let path = home.join("elon-codex-vault-slot.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    value
        .get("lease_expires_at")
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .is_some_and(|dt| dt.with_timezone(&Utc) <= Utc::now())
}

fn path_in_managed_vault(path: &Path) -> bool {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = data_base_dir().join("Elon").join("codex-vault");
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    full.starts_with(root)
}

fn data_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".local").join("share"))
            })
            .unwrap_or_else(std::env::temp_dir)
    }
}
