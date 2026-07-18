use std::collections::BTreeSet;

use serde_json::Value;

pub(super) fn collect_changed_files(event: &Value, files: &mut BTreeSet<String>) {
    let candidates = event
        .get("diff")
        .and_then(|value| value.get("files"))
        .or_else(|| event.get("files"));
    if let Some(items) = candidates.and_then(Value::as_array) {
        for item in items {
            collect_changed_file(item, files);
        }
    }
    if let Some(path) = event.get("args").and_then(|args| args.get("path")) {
        collect_changed_file(path, files);
    }
    if let Some(changes) = event
        .get("args")
        .and_then(|args| args.get("changes"))
        .and_then(Value::as_array)
        .or_else(|| event.get("changes").and_then(Value::as_array))
    {
        for change in changes {
            collect_changed_file(change, files);
        }
    }
}

pub(super) fn codex_item_failed(item: &Value) -> bool {
    item.get("exit_code")
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0)
        || tool_result_failed(item)
}

pub(super) fn codex_failure_summary(item: &Value) -> Option<String> {
    let command = item
        .get("command")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("command");
    let exit = item
        .get("exit_code")
        .and_then(Value::as_i64)
        .map(|code| format!("exit={code}"))
        .unwrap_or_else(|| {
            item.get("status")
                .and_then(Value::as_str)
                .unwrap_or("failed")
                .to_string()
        });
    let tail = item
        .get("output")
        .and_then(|output| output.get("tail"))
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let summary = match tail {
        Some(tail) => format!("{command} · {exit} · {tail}"),
        None => format!("{command} · {exit}"),
    };
    Some(summary.chars().take(600).collect())
}

fn collect_changed_file(value: &Value, files: &mut BTreeSet<String>) {
    let path = value
        .as_str()
        .or_else(|| value.get("path").and_then(Value::as_str))
        .map(str::trim)
        .filter(|path| !path.is_empty());
    if let Some(path) = path {
        files.insert(path.to_string());
    }
}

pub(super) fn tool_result_failed(event: &Value) -> bool {
    let status = event
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        status.as_str(),
        "failed" | "failure" | "error" | "errored" | "canceled" | "cancelled"
    ) || event.get("success").and_then(Value::as_bool) == Some(false)
        || event
            .get("result")
            .and_then(Value::as_str)
            .and_then(|result| result.split_whitespace().next())
            .and_then(|prefix| prefix.strip_prefix("exit="))
            .and_then(|code| code.parse::<i32>().ok())
            .is_some_and(|code| code != 0)
}
