use crate::{
    node_agent_cli_pty::{default_cols, default_rows},
    node_agent_cli_sidecar::CliSidecarRegistry,
    node_agent_cli_sidecar_runner::{
        follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig, CliSidecarOutputEvent,
    },
};
use std::{fs, path::PathBuf, time::Duration};
use tokio::sync::watch;

#[tokio::test]
async fn progress_aware_timeout_keeps_progressing_task_alive() {
    let root = temp_dir("progress-aware-keeps-alive");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-progress-aware";
    let output_path = registry.output_path(task_id, "sidecar-progress-aware");
    let (program, args) = scaled_progress_command(true);
    let started = tokio::time::Instant::now();

    run_sidecar(scaled_pipe_config(
        &root,
        &registry,
        task_id,
        "sidecar-progress-aware",
        output_path.clone(),
        program,
        args,
    ))
    .await
    .unwrap();

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |_| {})
        .await
        .unwrap();
    assert!(result.exit_ok);
    assert!(started.elapsed() >= Duration::from_secs(3));
    assert!(result.stdout_text.contains("progress-3"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn pty_sidecar_uses_progress_policy_instead_of_legacy_fixed_timeout() {
    let root = temp_dir("progress-aware-pty-keeps-alive");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-progress-aware-pty";
    let output_path = registry.output_path(task_id, "sidecar-progress-aware-pty");
    let (program, mut args) = scaled_progress_command(true);
    args.retain(|arg| arg != "--json");
    let mut config = scaled_pipe_config(
        &root,
        &registry,
        task_id,
        "sidecar-progress-aware-pty",
        output_path.clone(),
        program,
        args,
    );
    // This is the old fixed deadline. A supervised PTY must ignore it in
    // favour of runtime_policy while real output keeps arriving.
    config.timeout_secs = 2;

    run_sidecar(config).await.unwrap();

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |_| {})
        .await
        .unwrap();
    assert!(result.exit_ok, "{result:?}");
    assert!(result.stdout_text.contains("progress-3"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn heartbeat_is_visible_but_does_not_mask_true_idle_timeout() {
    let root = temp_dir("progress-aware-idle-timeout");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-idle-timeout";
    let output_path = registry.output_path(task_id, "sidecar-idle-timeout");
    let (program, args) = scaled_progress_command(false);

    run_sidecar(scaled_pipe_config(
        &root,
        &registry,
        task_id,
        "sidecar-idle-timeout",
        output_path.clone(),
        program,
        args,
    ))
    .await
    .unwrap();

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let mut heartbeats = 0;
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |event| {
        if matches!(event, CliSidecarOutputEvent::Heartbeat) {
            heartbeats += 1;
        }
    })
    .await
    .unwrap();
    assert!(!result.exit_ok);
    assert!(!result.canceled);
    assert!(heartbeats >= 1);
    assert!(result
        .terminal_error
        .as_deref()
        .is_some_and(|error| error.contains("空闲超时")));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn progress_aware_sidecar_cancel_still_reclaims_process() {
    let root = temp_dir("progress-aware-cancel");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-progress-cancel";
    let output_path = registry.output_path(task_id, "sidecar-progress-cancel");
    let (program, args) = scaled_progress_command(false);
    let mut config = scaled_pipe_config(
        &root,
        &registry,
        task_id,
        "sidecar-progress-cancel",
        output_path.clone(),
        program,
        args,
    );
    config.runtime_policy.as_mut().unwrap().idle_timeout_secs = 30;
    let run = tokio::spawn(run_sidecar(config));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(registry.record_cancel_command(task_id).unwrap());
    tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .expect("cancel should reclaim sidecar process")
        .unwrap()
        .unwrap();

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |_| {})
        .await
        .unwrap();
    assert!(result.canceled);
    assert!(!result.exit_ok);
    let _ = fs::remove_dir_all(root);
}

fn scaled_progress_command(with_progress: bool) -> (String, Vec<String>) {
    if cfg!(windows) {
        let script = if with_progress {
            "& { param([Parameter(ValueFromRemainingArguments=$true)]$rest) 1..3 | ForEach-Object { Write-Output \"progress-$_\"; Start-Sleep -Seconds 1 } }"
        } else {
            "& { param([Parameter(ValueFromRemainingArguments=$true)]$rest) Start-Sleep -Seconds 5 }"
        };
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                script.to_string(),
                "--json".to_string(),
            ],
        )
    } else {
        let script = if with_progress {
            "for i in 1 2 3; do echo progress-$i; sleep 1; done"
        } else {
            "sleep 5"
        };
        (
            "sh".to_string(),
            vec!["-c".to_string(), script.to_string(), "--json".to_string()],
        )
    }
}

fn scaled_pipe_config(
    root: &std::path::Path,
    registry: &CliSidecarRegistry,
    task_id: &str,
    session_id: &str,
    output_path: PathBuf,
    program: String,
    args: Vec<String>,
) -> CliSidecarLaunchConfig {
    CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: Some("full_access".to_string()),
        env: Vec::new(),
        output_path,
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 8,
        stdin_payload: None,
        runtime_policy: Some(crate::node_agent_cli_runtime_policy::CliRuntimePolicy {
            mode: "progress_aware_test_scaled".to_string(),
            total_timeout_secs: 8,
            idle_timeout_secs: 2,
            heartbeat_secs: 1,
            progress_aware: true,
        }),
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    }
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-node-agent-sidecar-{label}-{}-{}",
        std::process::id(),
        crate::node_agent_cli_sidecar::now_ms()
    ))
}
