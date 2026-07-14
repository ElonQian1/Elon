use crate::{
    node_runtime::{node_runtime_by_id, short_node_id},
    types::AppState,
};

pub async fn pc_node_progress_name(state: &AppState, node_id: &str) -> String {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return "未知节点".to_string();
    }

    if let Ok(Some(runtime)) = node_runtime_by_id(state, node_id).await {
        return pc_node_progress_name_from_parts(&runtime.display_name, &runtime.node_id);
    }

    pc_node_progress_name_from_parts("", node_id)
}

pub fn pc_node_progress_name_from_parts(display_name: &str, node_id: &str) -> String {
    let suffix = compact_node_suffix(node_id);
    let display_name = display_name.trim();
    if display_name.is_empty()
        || display_name == node_id
        || display_name == suffix
        || display_name == short_node_id(node_id)
    {
        return format!("节点 {suffix}");
    }
    if display_name.contains(&suffix) {
        return display_name.to_string();
    }
    format!("{display_name}（{suffix}）")
}

pub fn pc_cli_heartbeat_subject(
    display_model: &str,
    node_progress_name: &str,
    node_id: &str,
) -> String {
    let display_model = display_model.trim();
    if display_model.is_empty()
        || display_model == node_id
        || display_model.contains(node_id)
        || display_model.starts_with("node-")
        || is_cli_name_label(display_model)
    {
        return node_progress_name.to_string();
    }
    display_model.to_string()
}

fn is_cli_name_label(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "codex" | "copilot" | "claude" | "gemini"
    )
}

fn compact_node_suffix(node_id: &str) -> String {
    let node_id = node_id.trim();
    node_id
        .rsplit(['-', '_'])
        .next()
        .filter(|part| {
            part.len() >= 6 && part.len() <= 16 && part.chars().all(|ch| ch.is_ascii_hexdigit())
        })
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| short_node_id(node_id))
}

#[cfg(test)]
#[path = "pc_node_display_tests.rs"]
mod tests;
