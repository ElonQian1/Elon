use serde_json::Value;

pub(super) fn bounded_u64(value: Option<&Value>, default: u64, max: u64) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(default).min(max)
}

pub(super) fn sanitize_page_kind(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("auth") => "auth",
        Some("conversation") => "conversation",
        Some("home") => "home",
        Some("feature") => "feature",
        _ => "unknown",
    }
}

pub(super) fn sanitize_access_reason(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("login_required") => "login_required",
        Some("rate_limited") => "rate_limited",
        _ => "",
    }
}

pub(super) fn sanitize_access_source(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("visible_page") => "visible_page",
        Some("private_response") => "private_response",
        _ => "",
    }
}

pub(super) fn sanitize_private_stream_state(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("streaming") => "streaming",
        Some("completed") => "completed",
        _ => "idle",
    }
}
