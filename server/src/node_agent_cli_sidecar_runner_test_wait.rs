use crate::{
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_io::read_new_output_records,
};
use std::{path::Path, time::Duration};

pub(super) async fn wait_for_attachable_session(registry: &CliSidecarRegistry, task_id: &str) {
    for _ in 0..50 {
        if registry
            .session_for_task(task_id)
            .expect("sidecar session lookup should work")
            .map(|session| session.is_attachable_at(now_ms()))
            .unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("sidecar session did not become attachable");
}

pub(super) async fn wait_for_pty_output(output_path: &Path) {
    for _ in 0..100 {
        let mut offset = 0;
        if read_new_output_records(output_path, &mut offset)
            .is_ok_and(|records| records.iter().any(|record| record.record_type == "chunk"))
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("sidecar did not emit initial terminal output");
}
