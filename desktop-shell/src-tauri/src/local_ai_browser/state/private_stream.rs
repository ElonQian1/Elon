use serde_json::Value;

pub(super) fn observed(snapshot: Option<&Value>) -> bool {
    snapshot
        .and_then(|event| event.get("privateStreamObserved"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn revision(snapshot: Option<&Value>) -> u64 {
    snapshot
        .and_then(|event| event.get("privateStreamRevision"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

pub(super) fn state(snapshot: Option<&Value>) -> &'static str {
    match snapshot
        .and_then(|event| event.get("privateStreamState"))
        .and_then(Value::as_str)
    {
        Some("streaming") => "streaming",
        Some("completed") => "completed",
        _ => "idle",
    }
}

pub(super) fn is_streaming(snapshot: Option<&Value>) -> bool {
    if observed(snapshot) {
        match state(snapshot) {
            "streaming" => return true,
            "completed" => return false,
            _ => {}
        }
    }
    snapshot
        .and_then(|event| event.get("streaming"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
