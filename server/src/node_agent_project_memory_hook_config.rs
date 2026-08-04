//! Session-level Codex hook configuration for the project-memory receipt gate.

use anyhow::{Context, Result};
use serde_json::{json, Value};

const SKIP_MARKER: &str = "<elon-project-memory-receipt-skip version=\"1\">";

pub(crate) fn enabled(prompt: &str) -> bool {
    !prompt.contains(SKIP_MARKER) && env_bool("ELON_CODEX_PROJECT_MEMORY_HOOKS", true)
}

pub(crate) fn codex_config_args() -> Result<Vec<String>> {
    let executable = std::env::current_exe().context("无法定位项目记忆 Hook 执行程序")?;
    let path = executable.to_string_lossy();
    let unix_command = format!("{} --project-memory-hook", shell_quote(&path));
    let windows_command = format!("\"{}\" --project-memory-hook", path.replace('"', ""));
    let handler = |timeout: u64| {
        format!(
            "{{type=\"command\",command={},commandWindows={},timeout={timeout}}}",
            toml_string(&unix_command),
            toml_string(&windows_command)
        )
    };
    Ok(vec![
        "-c".to_string(),
        format!("hooks.PostToolUse=[{{hooks=[{}]}}]", handler(5)),
        "-c".to_string(),
        format!("hooks.Stop=[{{hooks=[{}]}}]", handler(5)),
        "-c".to_string(),
        format!("hooks.SessionEnd=[{{hooks=[{}]}}]", handler(1)),
    ])
}

pub(crate) fn capability_manifest() -> Value {
    json!({
        "schema": "elon.project_memory_hook_capabilities.v1",
        "configured_events": ["PostToolUse", "Stop", "SessionEnd"],
        "recognized_not_configured": [
            "SessionStart",
            "PreCompact",
            "PostCompact",
            "SubagentStart",
            "SubagentStop"
        ],
        "handler_type": "command",
        "plugin_bundle_status": "repository_bundle_available_not_installed",
        "trust_required": true,
        "trust_bypass_enabled": false,
        "future_event_behavior": "bounded_noop_until_runtime_verified",
        "configured_is_not_executed": true
    })
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        })
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_lifecycle_events_are_declared_but_not_enabled() {
        let manifest = capability_manifest();
        assert!(manifest["configured_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "PostToolUse"));
        assert!(manifest["recognized_not_configured"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "PostCompact"));
    }
}
