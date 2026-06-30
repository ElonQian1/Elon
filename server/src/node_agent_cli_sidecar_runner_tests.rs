// server/src/node_agent_cli_sidecar_runner_tests.rs

use crate::{
    node_agent_cli_pty::{default_cols, default_rows},
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_io::read_new_output_records,
    node_agent_cli_sidecar_runner::{
        follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig, CliSidecarOutputEvent,
    },
};
use std::{fs, path::PathBuf, time::Duration};
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
        initial_cols: default_cols(),
        initial_rows: default_rows(),
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
    assert!(result.stdout_text.contains("sidecar-err"));
    assert!(result.stderr_text.is_empty());
    assert!(events
        .iter()
        .any(|event| matches!(event, CliSidecarOutputEvent::ChildStarted(_))));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stdout(text) if text.contains("sidecar-out")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stdout(text) if text.contains("sidecar-err")
    )));
    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    assert!(records
        .iter()
        .any(|record| record.stream.as_deref() == Some("pty")));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn sidecar_runner_accepts_terminal_input_and_resize_after_attach() {
    let root = temp_dir("runner-terminal-input");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-sidecar-terminal-input";
    let session_id = "sidecar-terminal-input";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args, input) = interactive_shell_command();

    let run = tokio::spawn(run_sidecar(CliSidecarLaunchConfig {
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
        initial_cols: 90,
        initial_rows: 24,
    }));

    wait_for_attachable_session(&registry, task_id).await;
    assert!(registry
        .record_terminal_resize(task_id, 100, 30)
        .expect("resize command should be queued"));
    assert!(registry
        .record_terminal_input(task_id, &input)
        .expect("terminal input should be queued"));

    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .expect("sidecar should finish")
        .expect("join should succeed")
        .expect("sidecar run should succeed");

    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    let text = records
        .iter()
        .filter_map(|record| record.text.as_deref())
        .collect::<String>();
    assert!(text.contains("sidecar-input-ok"));
    assert!(records
        .iter()
        .any(|record| record.stream.as_deref() == Some("pty")));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn sidecar_runner_writes_recovered_tool_approval_to_pty() {
    let root = temp_dir("runner-tool-approval");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-sidecar-tool-approval";
    let session_id = "sidecar-tool-approval";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = approval_read_command();

    let run = tokio::spawn(run_sidecar(CliSidecarLaunchConfig {
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
        initial_cols: 90,
        initial_rows: 24,
    }));

    wait_for_attachable_session(&registry, task_id).await;
    assert!(registry
        .record_tool_approval_decision(task_id, "tap_recovered_1", "approve")
        .expect("approval decision should be queued"));

    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .expect("sidecar should finish")
        .expect("join should succeed")
        .expect("sidecar run should succeed");

    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    let text = records
        .iter()
        .filter_map(|record| record.text.as_deref())
        .collect::<String>();
    assert!(text.contains("decision:y"));

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

fn interactive_shell_command() -> (String, Vec<String>, String) {
    if cfg!(windows) {
        (
            "cmd".to_string(),
            vec!["/Q".to_string(), "/D".to_string(), "/K".to_string()],
            "echo sidecar-input-ok\r\nexit\r\n".to_string(),
        )
    } else {
        (
            "sh".to_string(),
            vec!["-i".to_string()],
            "echo sidecar-input-ok\nexit\n".to_string(),
        )
    }
}

fn approval_read_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "$x=[Console]::In.ReadLine(); Write-Output \"decision:$x\"".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "read x; printf 'decision:%s\\n' \"$x\"".to_string(),
            ],
        )
    }
}

async fn wait_for_attachable_session(registry: &CliSidecarRegistry, task_id: &str) {
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
