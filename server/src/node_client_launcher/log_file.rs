// server/src/node_client_launcher/log_file.rs

use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::paths;

const MAX_DETAIL_CHARS: usize = 600;
const LOGS_DIR_NAME: &str = "logs";
const LAUNCHER_LOG_FILE: &str = "client-launcher.jsonl";

pub(crate) fn logs_dir(install_dir: &Path) -> PathBuf {
    paths::internal_dir(install_dir).join(LOGS_DIR_NAME)
}

pub(crate) fn launcher_log_file(install_dir: &Path) -> PathBuf {
    logs_dir(install_dir).join(LAUNCHER_LOG_FILE)
}

pub(crate) fn record_event(install_dir: &Path, action: &str, ok: bool, detail: &str) {
    let file = launcher_log_file(install_dir);
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = json!({
        "at_ms": now_ms(),
        "action": action,
        "ok": ok,
        "detail": truncate_chars(detail, MAX_DETAIL_CHARS),
        "pid": std::process::id(),
    })
    .to_string();
    if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(file) {
        let _ = writeln!(handle, "{line}");
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{launcher_log_file, logs_dir, record_event};
    use serde_json::Value;
    use std::fs;

    #[test]
    fn launcher_log_path_lives_under_internal_logs() {
        let root = std::env::temp_dir().join("elon-launcher-log-path");

        assert!(logs_dir(&root).ends_with("_internal/logs"));
        assert!(launcher_log_file(&root).ends_with("_internal/logs/client-launcher.jsonl"));
    }

    #[test]
    fn record_event_writes_bounded_jsonl_without_failing_caller() {
        let root = unique_test_dir("write");
        let long = "x".repeat(900);

        record_event(&root, "install", false, &long);
        let text = fs::read_to_string(launcher_log_file(&root)).expect("log should be written");
        let value: Value = serde_json::from_str(text.trim()).expect("log line should be json");

        assert_eq!(value["action"], "install");
        assert_eq!(value["ok"], false);
        assert_eq!(value["detail"].as_str().unwrap().chars().count(), 600);
        assert!(value["pid"].as_u64().is_some());

        let _ = fs::remove_dir_all(root);
    }

    fn unique_test_dir(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "elon-launcher-log-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
