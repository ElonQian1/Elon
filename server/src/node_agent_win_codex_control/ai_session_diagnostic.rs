use serde_json::{json, Value};

const MAX_COUNT: u64 = 100_000;

const WINDOW_STATUSES: &[&str] = &["opening", "loading", "blocked", "ready", "error", "closed"];
const PAGE_KINDS: &[&str] = &["unknown", "auth", "conversation", "home", "feature"];
const CACHE_STATUSES: &[&str] = &["empty", "live", "cached"];
const EVENT_KINDS: &[&str] = &[
    "session_created",
    "adapter_ready",
    "message_snapshot",
    "conversation_snapshot",
    "composer_controls_snapshot",
    "navigation_snapshot",
    "ui_manifest_snapshot",
    "command_result",
    "browser_diagnostic",
    "stale_message_snapshot_ignored",
];
const ERROR_CODES: &[&str] = &[
    "navigation_blocked",
    "host_error",
    "page_error",
    "promise_rejection",
    "adapter_bootstrap_failed",
];
const COMMAND_ACTIONS: &[&str] = &[
    "snapshot",
    "send_prompt",
    "stop_generation",
    "regenerate_response",
    "new_conversation",
    "list_conversations",
    "open_conversation",
    "open_project",
    "start_google_login",
    "list_model_options",
    "list_composer_tools",
    "collect_model_options",
    "collect_composer_tools",
    "select_model_option",
    "select_composer_tool",
    "request_attachment_upload",
    "open_model_selector",
    "open_composer_tools",
    "start_dictation",
    "cancel_dictation",
    "submit_dictation",
    "remove_attachment",
    "dismiss_composer_menu",
    "list_navigation",
    "collect_navigation",
    "select_navigation",
    "dismiss_navigation",
    "snapshot_ui_manifest",
    "invoke_ui_control",
];

pub(super) fn sanitize(value: Option<&Value>) -> Result<Option<Value>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err("生产官方会话诊断必须是对象或 null。".to_string());
    }
    Ok(Some(json!({
        "present": bool_value(value, "present"),
        "window_status": fixed_or(value, "window_status", WINDOW_STATUSES, "unknown"),
        "window_visible": bool_value(value, "window_visible"),
        "loading": bool_value(value, "loading"),
        "adapter_connected": bool_value(value, "adapter_connected"),
        "semantic_snapshot_ready": bool_value(value, "semantic_snapshot_ready"),
        "composer_ready": bool_value(value, "composer_ready"),
        "context_ready": bool_value(value, "context_ready"),
        "context_transition_pending": bool_value(value, "context_transition_pending"),
        "page_kind": fixed_or(value, "page_kind", PAGE_KINDS, "unknown"),
        "cache_status": fixed_or(value, "cache_status", CACHE_STATUSES, "unknown"),
        "semantic_cache_status": fixed_or(value, "semantic_cache_status", CACHE_STATUSES, "unknown"),
        "navigation_cache_status": fixed_or(value, "navigation_cache_status", CACHE_STATUSES, "unknown"),
        "navigation_snapshot_ready": bool_value(value, "navigation_snapshot_ready"),
        "navigation_live": bool_value(value, "navigation_live"),
        "directory_complete": bool_value(value, "directory_complete"),
        "directory_observed_count": count(value, "directory_observed_count"),
        "directory_available_count": count(value, "directory_available_count"),
        "conversation_count": count(value, "conversation_count"),
        "project_count": count(value, "project_count"),
        "pinned_count": count(value, "pinned_count"),
        "local_conversation_count": count(value, "local_conversation_count"),
        "active_conversation": bool_value(value, "active_conversation"),
        "last_error_code": fixed(value, "last_error_code", ERROR_CODES),
        "last_event_kind": fixed(value, "last_event_kind", EVENT_KINDS),
        "last_command_action": fixed(value, "last_command_action", COMMAND_ACTIONS),
        "last_command_ok": value.get("last_command_ok").and_then(Value::as_bool),
        "message_count": count(value, "message_count"),
        "assistant_message_count": count(value, "assistant_message_count"),
        "streaming": bool_value(value, "streaming"),
        "updated_at_ms": value.get("updated_at_ms").and_then(Value::as_u64).unwrap_or(0),
    })))
}

fn bool_value(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn count(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(MAX_COUNT)
}

fn fixed(value: &Value, key: &str, allowed: &[&str]) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|candidate| allowed.contains(candidate))
        .map(str::to_string)
}

fn fixed_or(value: &Value, key: &str, allowed: &[&str], fallback: &str) -> String {
    fixed(value, key, allowed).unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_session_diagnostic_is_rebuilt_from_allowlisted_structure() {
        let sanitized = sanitize(Some(&json!({
            "present":true,
            "window_status":"ready",
            "adapter_connected":true,
            "semantic_snapshot_ready":true,
            "composer_ready":true,
            "context_ready":true,
            "page_kind":"conversation",
            "cache_status":"live",
            "semantic_cache_status":"live",
            "navigation_cache_status":"cached",
            "navigation_snapshot_ready":true,
            "directory_available_count":999999,
            "conversation_count":12,
            "last_error_code":"private_error",
            "last_event_kind":"message_snapshot",
            "last_command_action":"send_prompt",
            "last_command_ok":true,
            "message_count":4,
            "assistant_message_count":2,
            "draft":"private prompt",
            "url":"https://chatgpt.com/c/private",
            "owner":"private owner",
            "cookie":"private cookie",
            "token":"private token",
            "exception_detail":"private exception"
        })))
        .unwrap()
        .unwrap();
        assert_eq!(sanitized["directory_available_count"], MAX_COUNT);
        assert_eq!(sanitized["conversation_count"], 12);
        assert_eq!(sanitized["last_error_code"], Value::Null);
        assert_eq!(sanitized["last_command_action"], "send_prompt");
        let encoded = sanitized.to_string();
        for secret in [
            "private prompt",
            "chatgpt.com",
            "private owner",
            "private cookie",
            "private token",
            "private exception",
        ] {
            assert!(!encoded.contains(secret));
        }
    }

    #[test]
    fn malformed_or_unknown_fields_fail_closed_or_degrade() {
        assert!(sanitize(Some(&json!("not-an-object"))).is_err());
        assert_eq!(sanitize(None).unwrap(), None);
        let sanitized = sanitize(Some(&json!({
            "present":true,
            "window_status":"private-status",
            "page_kind":"private-page",
            "cache_status":"private-cache",
            "last_command_action":"private-action"
        })))
        .unwrap()
        .unwrap();
        assert_eq!(sanitized["window_status"], "unknown");
        assert_eq!(sanitized["page_kind"], "unknown");
        assert_eq!(sanitized["cache_status"], "unknown");
        assert_eq!(sanitized["last_command_action"], Value::Null);
    }
}
