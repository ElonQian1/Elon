//! Turns an early sidecar-worker exit into a durable terminal output record.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::process::Child;

use crate::{
    node_agent_cli_sidecar::CliSidecarRegistry,
    node_agent_cli_sidecar_io::{append_output, read_new_output_records, CliSidecarOutputRecord},
};

pub(super) fn spawn(
    child: Child,
    registry: CliSidecarRegistry,
    task_id: String,
    output_path: PathBuf,
) {
    tokio::spawn(async move {
        if let Err(error) = monitor(child, &registry, &task_id, &output_path).await {
            tracing::warn!(%task_id, %error, "failed to settle an exited CLI sidecar worker");
        }
    });
}

async fn monitor(
    mut child: Child,
    registry: &CliSidecarRegistry,
    task_id: &str,
    output_path: &Path,
) -> Result<()> {
    let status = child.wait().await?;
    if output_has_terminal_record(output_path)? {
        return Ok(());
    }
    let message = format!(
        "CLI sidecar worker exited before producing a terminal record (pid={}, status={})",
        child
            .id()
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        status
    );
    append_output(output_path, CliSidecarOutputRecord::error(message))?;
    let _ = registry.mark_task_terminal(task_id, "failed");
    Ok(())
}

fn output_has_terminal_record(output_path: &Path) -> Result<bool> {
    let mut offset = 0;
    Ok(read_new_output_records(output_path, &mut offset)?
        .iter()
        .any(|record| record.record_type == "exit"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn worker_exit_before_output_becomes_a_fast_recoverable_terminal() {
        let root = std::env::temp_dir().join(format!(
            "elon-sidecar-worker-early-exit-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("output.jsonl");
        std::fs::write(&output, b"").unwrap();
        let registry = CliSidecarRegistry::new(root.join("registry"));
        let mut command = early_exit_command();
        let child = command.spawn().unwrap();

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            monitor(child, &registry, "task-early-exit", &output),
        )
        .await
        .expect("worker monitor must not leave the task running")
        .unwrap();

        let mut offset = 0;
        let records = read_new_output_records(&output, &mut offset).unwrap();
        let terminal = records.last().expect("synthetic terminal record");
        assert_eq!(terminal.record_type, "exit");
        assert_eq!(terminal.success, Some(false));
        assert!(terminal
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("before producing a terminal record"));
        let (_cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
        let followed = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            crate::node_agent_cli_sidecar_runner::follow_sidecar_output(
                &registry,
                "task-early-exit",
                &output,
                &mut cancel_rx,
                |_| {},
            ),
        )
        .await
        .expect("local task follower should settle immediately")
        .unwrap();
        assert!(!followed.exit_ok);
        assert!(followed.terminal_error.is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    fn early_exit_command() -> tokio::process::Command {
        let mut command = tokio::process::Command::new("cmd.exe");
        command.args(["/D", "/C", "exit 17"]);
        command
    }

    #[cfg(not(windows))]
    fn early_exit_command() -> tokio::process::Command {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "exit 17"]);
        command
    }
}
