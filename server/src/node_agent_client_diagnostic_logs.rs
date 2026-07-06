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
#[path = "node_agent_client_diagnostic_logs_tests.rs"]
mod tests;
