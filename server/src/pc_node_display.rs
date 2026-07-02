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
mod tests {
    use super::*;

    #[test]
    fn progress_name_prefers_device_name_with_short_suffix() {
        assert_eq!(
            pc_node_progress_name_from_parts("ELON-4060", "node-usr_5c-dd33ed36"),
            "ELON-4060（dd33ed36）"
        );
    }

    #[test]
    fn progress_name_hides_raw_node_id_when_no_label_exists() {
        assert_eq!(
            pc_node_progress_name_from_parts("", "node-usr_5c-dd33ed36"),
            "节点 dd33ed36"
        );
    }
}
