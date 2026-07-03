pub(crate) fn is_pc_cli_heartbeat_progress(event_type: &str, message: &str) -> bool {
    event_type == "progress"
        && message.contains("正在处理中")
        && message.contains("已等待")
        && message.contains("Codex")
}

pub(crate) fn pc_dispatch_started_progress(value: &serde_json::Value) -> Option<String> {
    let agent_id = value
        .get("agent_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    let cli = value
        .get("cli")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("pc-ai");
    let cwd_configured = value
        .get("cwd_configured")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let message = if cwd_configured {
        format!(
            "已派发到 PC 节点 {}，等待 {} CLI 输出。",
            short_pc_node_id(agent_id),
            pc_cli_label(cli)
        )
    } else {
        format!(
            "已派发到 PC 节点 {}，等待 {} CLI 确认。",
            short_pc_node_id(agent_id),
            pc_cli_label(cli)
        )
    };
    serde_json::to_string(&serde_json::json!({
        "type": "runtime_status",
        "phase": "pc_dispatched",
        "runtime": pc_cli_label(cli),
        "message": message,
        "agent_id": agent_id,
        "cwd_configured": cwd_configured,
    }))
    .ok()
}

pub(crate) fn pc_cli_no_output_timeout_progress(timeout_secs: u64) -> Option<String> {
    serde_json::to_string(&serde_json::json!({
        "type": "runtime_status",
        "phase": "pc_cli_no_output_timeout",
        "runtime": "Codex",
        "message": format!(
            "已等待 {} 秒，但 PC 节点没有返回任何 Codex CLI 输出、命令或工具事件；本轮已停止。",
            timeout_secs
        ),
        "timeout_secs": timeout_secs,
        "expected_events": ["tool_call", "tool_result", "assistant_message", "usage", "cli_done"],
    }))
    .ok()
}

fn pc_cli_label(cli: &str) -> &'static str {
    match cli {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        "api-runtime" => "Route B",
        "server-runtime" => "Route C",
        _ => "PC AI",
    }
}

fn short_pc_node_id(agent_id: &str) -> String {
    let clean = agent_id.trim();
    if clean.chars().count() <= 18 {
        return clean.to_string();
    }
    let head = clean.chars().take(11).collect::<String>();
    let tail = clean
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}...{tail}")
}

#[cfg(test)]
mod tests {
    use super::{pc_cli_no_output_timeout_progress, pc_dispatch_started_progress};

    #[test]
    fn pc_dispatch_started_progress_names_node_and_cli() {
        let raw = serde_json::json!({
            "type": "pc_dispatch_started",
            "agent_id": "node-usr_5c-dd33ed36",
            "cli": "codex",
            "cwd_configured": false
        });
        let progress = pc_dispatch_started_progress(&raw).unwrap();
        let value: serde_json::Value = serde_json::from_str(&progress).unwrap();

        assert_eq!(value["type"], "runtime_status");
        assert_eq!(value["phase"], "pc_dispatched");
        assert_eq!(value["runtime"], "Codex");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("node-usr_5c...33ed36"));
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("等待 Codex CLI 确认"));
    }

    #[test]
    fn pc_cli_no_output_timeout_progress_is_structured() {
        let raw = pc_cli_no_output_timeout_progress(180).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["type"], "runtime_status");
        assert_eq!(value["phase"], "pc_cli_no_output_timeout");
        assert_eq!(value["runtime"], "Codex");
        assert_eq!(value["timeout_secs"], 180);
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("没有返回任何 Codex CLI 输出"));
    }
}
