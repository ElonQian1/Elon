use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

const MAX_RECENT_EVENTS: usize = 12;
const MAX_ACTION_CHARS: usize = 80;

pub(crate) fn diagnostic_log_summary(
    maintenance_log_file: &Path,
    launcher_log_file: Option<&Path>,
) -> Value {
    json!({
        "maintenance": log_file_summary(maintenance_log_file),
        "launcher": launcher_log_file
            .map(log_file_summary)
            .unwrap_or_else(|| json!({
                "exists": false,
                "reason": "launcher_log_path_unavailable",
                "line_count": 0,
                "parse_errors": 0,
                "actions": {},
                "recent_events": [],
            })),
    })
}

fn log_file_summary(path: &Path) -> Value {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            return json!({
                "exists": false,
                "path": path_to_string(path),
                "line_count": 0,
                "parse_errors": 0,
                "actions": {},
                "recent_events": [],
            });
        }
    };

    let mut line_count = 0u64;
    let mut parse_errors = 0u64;
    let mut actions = BTreeMap::<String, ActionCounts>::new();
    let mut recent_events = Vec::<Value>::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        line_count += 1;
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                parse_errors += 1;
                continue;
            }
        };
        let action = safe_action(&value);
        let ok = value.get("ok").and_then(Value::as_bool).unwrap_or(false);
        actions.entry(action.clone()).or_default().record(ok);
        push_recent_event(&mut recent_events, recent_event(&value, action, ok));
    }

    json!({
        "exists": true,
        "path": path_to_string(path),
        "line_count": line_count,
        "parse_errors": parse_errors,
        "actions": action_counts_json(actions),
        "recent_events": recent_events,
    })
}

fn push_recent_event(recent_events: &mut Vec<Value>, event: Value) {
    if recent_events.len() >= MAX_RECENT_EVENTS {
        recent_events.remove(0);
    }
    recent_events.push(event);
}

fn recent_event(value: &Value, action: String, ok: bool) -> Value {
    json!({
        "at_ms": value_u64(value.get("at_ms")),
        "action": action,
        "ok": ok,
        "pid": value_u64(value.get("pid")),
    })
}

fn safe_action(value: &Value) -> String {
    value
        .get("action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_chars(value, MAX_ACTION_CHARS))
        .unwrap_or_else(|| "unknown".to_string())
}

fn value_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64)
}

fn action_counts_json(actions: BTreeMap<String, ActionCounts>) -> Value {
    Value::Object(
        actions
            .into_iter()
            .map(|(action, counts)| {
                (
                    action,
                    json!({
                        "total": counts.total,
                        "ok": counts.ok,
                        "failed": counts.failed,
                    }),
                )
            })
            .collect(),
    )
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[derive(Default)]
struct ActionCounts {
    total: u64,
    ok: u64,
    failed: u64,
}

impl ActionCounts {
    fn record(&mut self, ok: bool) {
        self.total += 1;
        if ok {
            self.ok += 1;
        } else {
            self.failed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::diagnostic_log_summary;
    use std::{fs, path::PathBuf};

    #[test]
    fn summarizes_maintenance_and_launcher_logs_without_detail_values() {
        let dir = unique_test_dir("summary");
        fs::create_dir_all(&dir).unwrap();
        let maintenance = dir.join("client-maintenance.jsonl");
        let launcher = dir.join("client-launcher.jsonl");
        fs::write(
            &maintenance,
            concat!(
                "{\"at_ms\":1,\"action\":\"open_target\",\"ok\":true,\"detail\":\"secret-token\"}\n",
                "{\"at_ms\":2,\"action\":\"update\",\"ok\":false,\"detail\":\"api-key-value\"}\n",
                "not-json\n"
            ),
        )
        .unwrap();
        fs::write(
            &launcher,
            "{\"at_ms\":3,\"action\":\"install\",\"ok\":true,\"detail\":\"private path\",\"pid\":42}\n",
        )
        .unwrap();

        let summary = diagnostic_log_summary(&maintenance, Some(&launcher));
        let text = serde_json::to_string(&summary).unwrap();

        assert_eq!(summary["maintenance"]["line_count"], 3);
        assert_eq!(summary["maintenance"]["parse_errors"], 1);
        assert_eq!(summary["maintenance"]["actions"]["open_target"]["ok"], 1);
        assert_eq!(summary["maintenance"]["actions"]["update"]["failed"], 1);
        assert_eq!(summary["launcher"]["actions"]["install"]["ok"], 1);
        assert_eq!(summary["launcher"]["recent_events"][0]["pid"], 42);
        assert!(!text.contains("secret-token"));
        assert!(!text.contains("api-key-value"));
        assert!(!text.contains("private path"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_launcher_path_is_reported_without_failing_export() {
        let dir = unique_test_dir("missing");
        let summary = diagnostic_log_summary(&dir.join("missing.jsonl"), None);

        assert_eq!(summary["maintenance"]["exists"], false);
        assert_eq!(summary["launcher"]["exists"], false);
        assert_eq!(
            summary["launcher"]["reason"],
            "launcher_log_path_unavailable"
        );

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-diagnostic-log-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
