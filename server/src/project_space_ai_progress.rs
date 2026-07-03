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
