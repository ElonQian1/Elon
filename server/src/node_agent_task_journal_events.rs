use serde_json::{json, Value};

const MAX_CHUNK_CHARS: usize = 12_000;
const MAX_ERROR_CHARS: usize = 2_000;

pub(crate) fn cli_chunk_event(
    req_id: &str,
    stream: &str,
    text: &str,
    at_ms: u128,
) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }
    parse_runtime_event(req_id, stream, text, at_ms).or_else(|| {
        Some(json!({
            "type": "cli_chunk",
            "req_id": req_id,
            "stream": normalize_stream(stream),
            "text": truncate_chars(text, MAX_CHUNK_CHARS),
            "at_ms": at_ms
        }))
    })
}

pub(crate) fn normalize_finish_status(status: &str, error: Option<&str>) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "done" | "ok" | "success" | "succeeded" => "done",
        "failed" | "failure" | "error" | "errored" => "failed",
        "canceled" | "cancelled" | "cancel" | "stopped" => "canceled",
        "interrupted" => "interrupted",
        "resume_required" => "resume_required",
        "finished" if looks_canceled(error) => "canceled",
        "finished" if has_error(error) => "failed",
        "finished" => "finished",
        _ if looks_canceled(error) => "canceled",
        _ if has_error(error) => "failed",
        _ => "finished",
    }
}

pub(crate) fn normalize_finish_error(error: Option<&str>) -> Option<String> {
    error
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_chars(value, MAX_ERROR_CHARS))
}

pub(crate) fn is_terminal_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "finished"
            | "done"
            | "failed"
            | "canceled"
            | "cancelled"
            | "interrupted"
            | "resume_required"
    )
}

fn parse_runtime_event(req_id: &str, stream: &str, text: &str, at_ms: u128) -> Option<Value> {
    let mut parsed: Value = serde_json::from_str(text.trim()).ok()?;
    let event_type = parsed.get("type").and_then(Value::as_str)?;
    if !matches!(
        event_type,
        "tool_call" | "tool_result" | "tool_approval_required" | "tool_approval_decision"
    ) {
        return None;
    }
    truncate_json_strings(&mut parsed, MAX_CHUNK_CHARS);
    Some(json!({
        "type": "tool_event",
        "req_id": req_id,
        "stream": normalize_stream(stream),
        "event": parsed,
        "text": truncate_chars(text, MAX_CHUNK_CHARS),
        "at_ms": at_ms
    }))
}

fn truncate_json_strings(value: &mut Value, max_chars: usize) {
    match value {
        Value::String(text) => {
            if text.chars().count() > max_chars {
                *text = truncate_chars(text, max_chars);
            }
        }
        Value::Array(items) => {
            for item in items {
                truncate_json_strings(item, max_chars);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                truncate_json_strings(item, max_chars);
            }
        }
        _ => {}
    }
}

fn normalize_stream(stream: &str) -> &'static str {
    match stream.trim().to_ascii_lowercase().as_str() {
        "stderr" => "stderr",
        "runtime" => "runtime",
        _ => "stdout",
    }
}

fn has_error(error: Option<&str>) -> bool {
    error.map(str::trim).is_some_and(|value| !value.is_empty())
}

fn looks_canceled(error: Option<&str>) -> bool {
    let Some(error) = error.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let lower = error.to_ascii_lowercase();
    lower.contains("cancel")
        || lower.contains("cancelled")
        || lower.contains("canceled")
        || lower.contains("stopped")
        || error.contains("取消")
        || error.contains("停止")
        || error.contains("终止")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push_str("\n...（本机 journal 输出已截断）");
    out
}

#[cfg(test)]
mod tests {
    use super::cli_chunk_event;
    use serde_json::json;

    #[test]
    fn oversized_tool_result_stays_structured_and_bounded() {
        let long_result = "x".repeat(15_000);
        let raw = serde_json::to_string(&json!({
            "type": "tool_result",
            "tool": "run_command",
            "status": "ok",
            "result": long_result,
        }))
        .unwrap();

        let event = cli_chunk_event("req-1", "runtime", &raw, 1).unwrap();

        assert_eq!(event["type"], "tool_event");
        assert_eq!(event["event"]["type"], "tool_result");
        assert!(event["event"]["result"]
            .as_str()
            .unwrap_or_default()
            .contains("本机 journal 输出已截断"));
        assert!(event["text"]
            .as_str()
            .unwrap_or_default()
            .contains("本机 journal 输出已截断"));
    }
}
