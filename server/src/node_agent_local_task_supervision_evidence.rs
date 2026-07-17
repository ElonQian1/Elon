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
    {
        for change in changes {
            collect_changed_file(change, files);
        }
    }
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
