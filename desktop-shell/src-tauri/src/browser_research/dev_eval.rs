//! Development switch for page expression evaluation. Off in release builds unless the
//! operator starts the Win host with `ELON_BROWSER_RESEARCH_DEV_EVAL=1`; never toggled by MCP.
use super::{host, model::*, privacy};
use serde_json::{json, Value};

pub const SWITCH: &str = "ELON_BROWSER_RESEARCH_DEV_EVAL";
pub const MAX_EXPRESSION_BYTES: usize = 4096;

pub fn enabled() -> bool {
    enabled_with(
        cfg!(debug_assertions),
        std::env::var(SWITCH).ok().as_deref(),
    )
}

fn enabled_with(debug_build: bool, switch: Option<&str>) -> bool {
    debug_build || switch.map(str::trim) == Some("1")
}

pub fn validate_expression(expression: Option<&str>) -> Result<String, String> {
    let expression = expression.ok_or("invalid_expression")?;
    if expression.trim().is_empty()
        || expression.len() > MAX_EXPRESSION_BYTES
        || expression
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("invalid_expression".into());
    }
    Ok(expression.to_string())
}

pub fn evaluate(
    app: &tauri::AppHandle,
    handle: &host::HostHandle,
    session: &Session,
    expression: Option<&str>,
) -> Result<Value, String> {
    if !enabled() {
        return Err("dev_eval_disabled".into());
    }
    let expression = validate_expression(expression)?;
    let raw = host::evaluate(app, handle, expression)?;
    // The returned value is page text; apply the same credential policy as stored bodies.
    let (value_json, redacted) = privacy::clean_body(
        raw.get("value_json")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )?;
    Ok(
        json!({"schema":RESULT_SCHEMA,"kind":"evaluate","session_id":session.id,
        "generation":session.generation,"type":raw["type"],"value_json":value_json,
        "truncated":raw["truncated"],"redacted":redacted,"exception":raw["exception"],
        "dev_switch":SWITCH,"page_state_is_untrusted":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_builds_require_the_explicit_switch_value() {
        assert!(enabled_with(true, None));
        assert!(!enabled_with(false, None));
        assert!(!enabled_with(false, Some("true")));
        assert!(!enabled_with(false, Some("0")));
        assert!(enabled_with(false, Some("1")));
        assert!(enabled_with(false, Some(" 1 ")));
    }

    #[test]
    fn expressions_are_bounded_and_printable() {
        assert!(validate_expression(None).is_err());
        assert!(validate_expression(Some("   ")).is_err());
        assert!(validate_expression(Some("document.title\u{0}")).is_err());
        assert!(validate_expression(Some(&"x".repeat(MAX_EXPRESSION_BYTES + 1))).is_err());
        assert_eq!(
            validate_expression(Some("document.title\n\t+1")).unwrap(),
            "document.title\n\t+1"
        );
    }
}
