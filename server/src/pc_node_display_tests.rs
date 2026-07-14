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

#[test]
fn heartbeat_subject_replaces_raw_node_id_with_pc_name() {
    assert_eq!(
        pc_cli_heartbeat_subject(
            "node-usr_5c-dd33ed36",
            "ELON-4060（dd33ed36）",
            "node-usr_5c-dd33ed36"
        ),
        "ELON-4060（dd33ed36）"
    );
}

#[test]
fn heartbeat_subject_replaces_cli_name_with_pc_name() {
    assert_eq!(
        pc_cli_heartbeat_subject("codex", "ELON-4060（dd33ed36）", "node-usr_5c-dd33ed36"),
        "ELON-4060（dd33ed36）"
    );
}

#[test]
fn heartbeat_subject_keeps_real_model_label() {
    assert_eq!(
        pc_cli_heartbeat_subject(
            "GPT-5.5 · 推理 xhigh",
            "ELON-4060（dd33ed36）",
            "node-usr_5c-dd33ed36"
        ),
        "GPT-5.5 · 推理 xhigh"
    );
}
