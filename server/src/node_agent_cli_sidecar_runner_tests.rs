// server/src/node_agent_cli_sidecar_runner_tests.rs

use crate::{
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_runner::{
        follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig, CliSidecarOutputEvent,
    },
};
use std::{fs, path::PathBuf};
use tokio::sync::watch;

#[tokio::test]
async fn sidecar_runner_registers_real_child_and_replays_output() {
    let root = temp_dir("runner-registers-real-child");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-sidecar-real-child";
    let session_id = "sidecar-real-child";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = shell_echo_command();

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        timeout_secs: 10,
        stdin_piped_empty: false,
    })
    .await
    .expect("sidecar should run shell child");

    let sessions = registry
        .latest_sessions(5)
        .expect("sidecar sessions should load");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, session_id);
    assert_eq!(sessions[0].state, "finished");
    assert_eq!(sessions[0].sidecar_pid, Some(std::process::id()));
    assert!(sessions[0].child_pid.is_some());

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let mut events = Vec::new();
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |event| {
        events.push(event)
    })
    .await
    .expect("sidecar output should replay");

    assert!(result.exit_ok);
    assert!(result.stdout_text.contains("sidecar-out"));
    assert!(result.stderr_text.contains("sidecar-err"));
    assert!(events
        .iter()
        .any(|event| matches!(event, CliSidecarOutputEvent::ChildStarted(_))));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stdout(text) if text.contains("sidecar-out")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stderr(text) if text.contains("sidecar-err")
    )));

    let _ = fs::remove_dir_all(root);
}

fn shell_echo_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "echo sidecar-out && echo sidecar-err 1>&2".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "echo sidecar-out; echo sidecar-err >&2".to_string(),
            ],
        )
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "elon-cli-sidecar-{}-{}-{}",
        name,
        std::process::id(),
        now_ms()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}
