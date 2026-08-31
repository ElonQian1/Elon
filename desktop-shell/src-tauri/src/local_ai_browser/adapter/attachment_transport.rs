use serde_json::{json, Map, Value};

use super::scalar::bounded_u64;

pub(in crate::local_ai_browser) fn sanitize(event: &Map<String, Value>) -> Result<Value, String> {
    if event.get("transportVersion").and_then(Value::as_u64) != Some(1) {
        return Err("ChatGPT 附件传输事件版本无效。".to_string());
    }
    let sequence = event
        .get("sequence")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0 && *value <= 1_000_000_000)
        .ok_or_else(|| "ChatGPT 附件传输事件序号无效。".to_string())?;
    let state = event
        .get("state")
        .and_then(Value::as_str)
        .filter(|state| matches!(*state, "armed" | "started" | "completed" | "failed"))
        .ok_or_else(|| "ChatGPT 附件传输事件状态无效。".to_string())?;
    Ok(json!({
        "type": "attachment_transport",
        "transportVersion": 1,
        "sequence": sequence,
        "state": state,
        "completedCount": bounded_u64(event.get("completedCount"), 0, 10),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_bounded_structural_state() {
        let raw = json!({
            "type": "attachment_transport",
            "transportVersion": 1,
            "sequence": 12,
            "state": "completed",
            "completedCount": 999,
            "url": "https://chatgpt.com/backend-api/files/private-id",
            "authorization": "secret"
        });
        let event = sanitize(raw.as_object().unwrap()).unwrap();
        assert_eq!(event["state"], "completed");
        assert_eq!(event["completedCount"], 10);
        assert!(!event.to_string().contains("private-id"));
        assert!(!event.to_string().contains("secret"));
    }

    #[test]
    fn rejects_unsupported_or_malformed_lifecycle_events() {
        for raw in [
            json!({"transportVersion":2,"sequence":1,"state":"armed"}),
            json!({"transportVersion":1,"sequence":0,"state":"armed"}),
            json!({"transportVersion":1,"sequence":1,"state":"unknown"}),
        ] {
            assert!(sanitize(raw.as_object().unwrap()).is_err());
        }
    }
}
