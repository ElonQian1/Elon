// server/src/node_agent_cli_sidecar_runner.rs

#[path = "node_agent_cli_pipe_sidecar_runner.rs"]
mod pipe_sidecar_runner;
#[path = "node_agent_cli_sidecar_worker_monitor.rs"]
mod worker_monitor;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::sync::watch;

use crate::{
    node_agent_cli_pty::{
        default_cols, default_rows, CliPtyEvent, CliPtyProcess, CliPtySpawnConfig,
    },
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord},
    node_agent_cli_sidecar_io::{
        append_output, read_new_commands, read_new_output_records, read_output_records_from,
        CliSidecarOutputRecord,
    },
    node_agent_codex_approval::CodexApprovalTracker,
    node_agent_codex_session::{self, CodexSessionCapture},
    node_agent_task_journal::TaskJournal,
};
use pipe_sidecar_runner::run_pipe_json_sidecar;

pub(crate) const SIDECAR_POLL_MS: u64 = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CliSidecarLaunchConfig {
    pub session_id: String,
    pub task_id: String,
    pub cli_name: String,
    pub route: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub runtime_permission: Option<String>,
    pub env: Vec<(String, String)>,
    pub output_path: PathBuf,
    pub registry_dir: PathBuf,
    #[serde(default)]
    pub task_journal_dir: Option<PathBuf>,
    #[serde(default)]
    pub worker_path: Option<PathBuf>,
    #[serde(default)]
    pub worker_release: Option<String>,
    #[serde(default)]
    pub worker_sha256: Option<String>,
    #[serde(default)]
    pub codex_session_scope_key: Option<String>,
    #[serde(default)]
    pub legacy_codex_sessions_file: Option<PathBuf>,
    pub timeout_secs: u64,
    #[serde(default)]
    pub stdin_payload: Option<String>,
    #[serde(default)]
    pub runtime_policy: Option<crate::node_agent_cli_runtime_policy::CliRuntimePolicy>,
    #[serde(default)]
    pub stdin_piped_empty: bool,
    #[serde(default = "default_cols")]
    pub initial_cols: u16,
    #[serde(default = "default_rows")]
    pub initial_rows: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct CliSidecarLaunch {
    pub sidecar_pid: Option<u32>,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliSidecarOutputEvent {
    Stdout(String),
    Stderr(String),
    ChildStarted(u32),
    Heartbeat,
}

#[derive(Debug, Clone)]
pub(crate) struct CliSidecarRunResult {
    pub exit_ok: bool,
    pub stdout_text: String,
    pub stderr_text: String,
    pub canceled: bool,
    pub terminal_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CliSidecarReplayCursor {
    pub(crate) offset: u64,
    pub(crate) sequence: u64,
}

pub(crate) fn sidecar_enabled() -> bool {
    !matches!(
        std::env::var("ELON_CLI_SIDECAR_DISABLED")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

pub(crate) fn sidecar_enabled_for_cli(cli_name: &str) -> bool {
    sidecar_enabled()
        && sidecar_enabled_for_cli_name(
            cli_name,
            codex_json_direct_stdout_enabled(),
            codex_pipe_sidecar_enabled(),
        )
}

fn sidecar_enabled_for_cli_name(
    cli_name: &str,
    codex_json_direct_stdout: bool,
    codex_pipe_sidecar: bool,
) -> bool {
    if !cli_name.trim().eq_ignore_ascii_case("codex") {
        return true;
    }
    !codex_json_direct_stdout || codex_pipe_sidecar
}

fn codex_json_direct_stdout_enabled() -> bool {
    codex_json_direct_stdout_enabled_from(std::env::var("ELON_CODEX_JSON_DIRECT_STDOUT").ok())
}

fn codex_json_direct_stdout_enabled_from(value: Option<String>) -> bool {
    let Some(value) = value else {
        return true;
    };
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off" | "disabled"
    )
}

fn codex_pipe_sidecar_enabled() -> bool {
    codex_pipe_sidecar_enabled_from(std::env::var("ELON_CODEX_PIPE_SIDECAR").ok())
}

fn codex_pipe_sidecar_enabled_from(value: Option<String>) -> bool {
    let Some(value) = value else {
        return true;
    };
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off" | "disabled"
    )
}

fn should_use_pipe_json_sidecar(config: &CliSidecarLaunchConfig) -> bool {
    should_use_pipe_json_sidecar_from(
        config,
        codex_json_direct_stdout_enabled(),
        codex_pipe_sidecar_enabled(),
    )
}

fn should_use_pipe_json_sidecar_from(
    config: &CliSidecarLaunchConfig,
    codex_json_direct_stdout: bool,
    codex_pipe_sidecar: bool,
) -> bool {
    config.cli_name.trim().eq_ignore_ascii_case("codex")
        && codex_json_direct_stdout
        && codex_pipe_sidecar
        && args_request_json_stream(&config.args)
}

fn args_request_json_stream(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--json")
}

pub(crate) fn session_id_for_task(task_id: &str) -> String {
    format!("sidecar-{}-{}", safe_id_fragment(task_id), now_ms())
}

pub(crate) async fn spawn_sidecar(mut config: CliSidecarLaunchConfig) -> Result<CliSidecarLaunch> {
    let current_exe = std::env::current_exe().context("读取当前 node-agent exe 路径")?;
    let worker =
        crate::node_agent_cli_worker::prepare_versioned_worker(&current_exe, &config.registry_dir)?;
    config.worker_path = Some(worker.path.clone());
    config.worker_release = Some(worker.release);
    config.worker_sha256 = Some(worker.sha256);
    let config_path = config_path_for(&config.session_id);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 sidecar config 目录 {:?}", parent))?;
    }
    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 sidecar output 目录 {:?}", parent))?;
    }
    fs::write(&config.output_path, b"")
        .with_context(|| format!("初始化 sidecar output {:?}", config.output_path))?;
    fs::write(&config_path, serde_json::to_vec_pretty(&config)?)
        .with_context(|| format!("写入 sidecar config {:?}", config_path))?;

    let mut cmd = tokio::process::Command::new(&worker.path);
    cmd.arg("--cli-sidecar")
        .arg(&config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_tokio_command_window(&mut cmd);
    let child = cmd.spawn().context("启动 CLI sidecar 进程")?;
    let sidecar_pid = child.id();
    worker_monitor::spawn(
        child,
        CliSidecarRegistry::new(config.registry_dir.clone()),
        config.task_id.clone(),
        config.output_path.clone(),
    );
    Ok(CliSidecarLaunch {
        sidecar_pid,
        output_path: config.output_path,
    })
}

pub(crate) async fn run_sidecar_from_config_path(path: impl AsRef<Path>) -> Result<()> {
    let text = fs::read_to_string(path.as_ref())
        .with_context(|| format!("读取 CLI sidecar config {:?}", path.as_ref()))?;
    let config: CliSidecarLaunchConfig = serde_json::from_str(&text)
        .with_context(|| format!("解析 CLI sidecar config {:?}", path.as_ref()))?;
    run_sidecar(config).await
}

pub(crate) async fn follow_sidecar_output(
    registry: &CliSidecarRegistry,
    task_id: &str,
    output_path: &Path,
    cancel_rx: &mut watch::Receiver<bool>,
    on_event: impl FnMut(CliSidecarOutputEvent),
) -> Result<CliSidecarRunResult> {
    follow_sidecar_output_from(
        registry,
        task_id,
        output_path,
        CliSidecarReplayCursor::default(),
        cancel_rx,
        on_event,
        |cursor| {
            if let Some(session) = registry.session_for_task(task_id)? {
                registry.record_output_cursor(
                    task_id,
                    &session.session_id,
                    cursor.offset,
                    cursor.sequence,
                )?;
            }
            Ok(())
        },
    )
    .await
}

pub(crate) async fn follow_sidecar_output_from(
    registry: &CliSidecarRegistry,
    task_id: &str,
    output_path: &Path,
    initial_cursor: CliSidecarReplayCursor,
    cancel_rx: &mut watch::Receiver<bool>,
    mut on_event: impl FnMut(CliSidecarOutputEvent),
    mut on_cursor: impl FnMut(CliSidecarReplayCursor) -> Result<()>,
) -> Result<CliSidecarRunResult> {
    follow_sidecar_output_from_with_batch(
        registry,
        task_id,
        output_path,
        initial_cursor,
        cancel_rx,
        on_event,
        |_, cursor| on_cursor(cursor),
    )
    .await
}

pub(crate) async fn follow_sidecar_output_from_with_batch(
    registry: &CliSidecarRegistry,
    task_id: &str,
    output_path: &Path,
    initial_cursor: CliSidecarReplayCursor,
    cancel_rx: &mut watch::Receiver<bool>,
    mut on_event: impl FnMut(CliSidecarOutputEvent),
    mut persist_batch: impl FnMut(
        &[crate::node_agent_cli_sidecar_io::CliSidecarOutputRecord],
        CliSidecarReplayCursor,
    ) -> Result<()>,
) -> Result<CliSidecarRunResult> {
    let mut cursor = initial_cursor;
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut exit_ok = None;
    let mut canceled = false;
    let mut child_started = false;
    let mut terminal_error = None;

    loop {
        let batch_start = cursor;
        let records = read_new_output_records(output_path, &mut cursor.offset)?;
        cursor.sequence = cursor.sequence.saturating_add(records.len() as u64);
        let read_records = !records.is_empty();
        if read_records {
            let durable_count = records
                .iter()
                .position(|record| record.record_type == "exit")
                .unwrap_or(records.len());
            if durable_count == records.len() {
                persist_batch(&records, cursor)?;
            } else if durable_count > 0 {
                let mut durable_offset = batch_start.offset;
                let durable_records =
                    read_output_records_from(output_path, &mut durable_offset, durable_count)?;
                let durable_cursor = CliSidecarReplayCursor {
                    offset: durable_offset,
                    sequence: batch_start
                        .sequence
                        .saturating_add(durable_records.len() as u64),
                };
                persist_batch(&durable_records, durable_cursor)?;
            }
        }
        for record in records {
            match record.record_type.as_str() {
                "chunk" => {
                    let text = record.text.unwrap_or_default();
                    match record.stream.as_deref() {
                        Some("stdout") | Some("pty") | Some("runtime") => {
                            stdout_text.push_str(&text);
                            on_event(CliSidecarOutputEvent::Stdout(text));
                        }
                        Some("stderr") => {
                            stderr_text.push_str(&text);
                            on_event(CliSidecarOutputEvent::Stderr(text));
                        }
                        _ => {}
                    }
                }
                "child_started" => {
                    if let Some(pid) = record.child_pid {
                        child_started = true;
                        on_event(CliSidecarOutputEvent::ChildStarted(pid));
                    }
                }
                "exit" => {
                    if let Some(error) = record.error {
                        terminal_error = Some(error);
                        // A spawn failure has no later child exit record. A timeout
                        // after `child_started` does, so keep following to drain any
                        // token event already buffered ahead of the real exit.
                        if !child_started {
                            exit_ok = Some(false);
                        }
                    } else {
                        exit_ok = Some(record.success.unwrap_or(false));
                        canceled = record.canceled.unwrap_or(false);
                    }
                }
                "runtime" => {
                    if record
                        .runtime
                        .as_ref()
                        .and_then(|value| value.get("heartbeat"))
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                    {
                        on_event(CliSidecarOutputEvent::Heartbeat);
                    }
                }
                _ => {}
            }
        }
        if let Some(exit_ok) = exit_ok {
            return Ok(CliSidecarRunResult {
                exit_ok,
                stdout_text,
                stderr_text,
                canceled,
                terminal_error,
            });
        }
        tokio::select! {
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    registry.record_cancel_command(task_id)?;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(SIDECAR_POLL_MS)) => {}
        }
    }
}

pub(crate) async fn run_sidecar(mut config: CliSidecarLaunchConfig) -> Result<()> {
    if should_use_pipe_json_sidecar(&config) {
        return run_pipe_json_sidecar(config).await;
    }
    restore_prompt_arg_for_pty(&mut config);
    run_pty_sidecar(config).await
}

fn restore_prompt_arg_for_pty(config: &mut CliSidecarLaunchConfig) {
    if config.cli_name.trim().eq_ignore_ascii_case("codex")
        && config.args.last().is_some_and(|arg| arg == "-")
    {
        if let (Some(prompt), Some(last)) = (config.stdin_payload.take(), config.args.last_mut()) {
            *last = prompt;
        }
    }
}

#[path = "node_agent_cli_sidecar_runner_impl.rs"]
mod sidecar_impl;
use self::sidecar_impl::*;

#[cfg(windows)]
pub(crate) fn hide_tokio_command_window(command: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_tokio_command_window(_command: &mut tokio::process::Command) {}

#[cfg(test)]
mod tests {
    use crate::node_agent_cli_sidecar_io::{append_output, CliSidecarOutputRecord};

    #[test]
    fn sidecar_uses_pipe_sidecar_for_codex_when_json_direct_stdout_is_enabled() {
        assert!(super::sidecar_enabled_for_cli_name("codex", true, true));
        assert!(super::sidecar_enabled_for_cli_name(" CODEX ", true, true));
        assert!(super::sidecar_enabled_for_cli_name("copilot", true, true));
        assert!(!super::sidecar_enabled_for_cli_name("codex", true, false));
    }

    #[test]
    fn sidecar_can_be_reenabled_for_codex_fallback() {
        assert!(super::sidecar_enabled_for_cli_name("codex", false, false));
        assert!(super::codex_json_direct_stdout_enabled_from(None));
        assert!(super::codex_json_direct_stdout_enabled_from(Some(
            "true".to_string()
        )));
        assert!(!super::codex_json_direct_stdout_enabled_from(Some(
            "0".to_string()
        )));
        assert!(!super::codex_json_direct_stdout_enabled_from(Some(
            "OFF".to_string()
        )));
        assert!(super::codex_pipe_sidecar_enabled_from(None));
        assert!(super::codex_pipe_sidecar_enabled_from(Some(
            "true".to_string()
        )));
        assert!(!super::codex_pipe_sidecar_enabled_from(Some(
            "0".to_string()
        )));
    }

    #[test]
    fn pipe_json_sidecar_requires_codex_json_args() {
        let mut config = super::CliSidecarLaunchConfig {
            session_id: "sidecar-test".to_string(),
            task_id: "task-test".to_string(),
            cli_name: "codex".to_string(),
            route: "route_a_external_cli".to_string(),
            program: "codex".to_string(),
            args: vec!["exec".to_string(), "--json".to_string()],
            cwd: None,
            runtime_permission: None,
            env: Vec::new(),
            output_path: std::path::PathBuf::from("output.jsonl"),
            registry_dir: std::path::PathBuf::from("registry"),
            task_journal_dir: None,
            worker_path: None,
            worker_release: None,
            worker_sha256: None,
            codex_session_scope_key: None,
            legacy_codex_sessions_file: None,
            timeout_secs: 10,
            stdin_payload: None,
            runtime_policy: None,
            stdin_piped_empty: false,
            initial_cols: crate::node_agent_cli_pty::default_cols(),
            initial_rows: crate::node_agent_cli_pty::default_rows(),
        };
        assert!(super::should_use_pipe_json_sidecar_from(
            &config, true, true
        ));
        config.args = vec!["exec".to_string()];
        assert!(!super::should_use_pipe_json_sidecar_from(
            &config, true, true
        ));
        config.cli_name = "copilot".to_string();
        config.args = vec!["--json".to_string()];
        assert!(!super::should_use_pipe_json_sidecar_from(
            &config, true, true
        ));
        config.cli_name = "codex".to_string();
        assert!(!super::should_use_pipe_json_sidecar_from(
            &config, true, false
        ));
    }

    #[test]
    fn pty_fallback_keeps_legacy_codex_prompt_argument() {
        let prompt = "监督第一行\n第二行 & | < >".to_string();
        let mut config = super::CliSidecarLaunchConfig {
            session_id: "sidecar-test".to_string(),
            task_id: "task-test".to_string(),
            cli_name: "codex".to_string(),
            route: "route_a_external_cli".to_string(),
            program: "codex".to_string(),
            args: vec!["exec".to_string(), "--json".to_string(), "-".to_string()],
            cwd: None,
            runtime_permission: None,
            env: Vec::new(),
            output_path: std::path::PathBuf::from("output.jsonl"),
            registry_dir: std::path::PathBuf::from("registry"),
            task_journal_dir: None,
            worker_path: None,
            worker_release: None,
            worker_sha256: None,
            codex_session_scope_key: None,
            legacy_codex_sessions_file: None,
            timeout_secs: 10,
            stdin_payload: Some(prompt.clone()),
            runtime_policy: None,
            stdin_piped_empty: false,
            initial_cols: crate::node_agent_cli_pty::default_cols(),
            initial_rows: crate::node_agent_cli_pty::default_rows(),
        };
        super::restore_prompt_arg_for_pty(&mut config);
        assert_eq!(config.args.last(), Some(&prompt));
        assert!(config.stdin_payload.is_none());
    }

    #[tokio::test]
    async fn replay_persists_nonterminal_cursor_but_replays_terminal_until_completion_commits() {
        let root = std::env::temp_dir().join(format!(
            "elon-sidecar-cursor-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("output.jsonl");
        append_output(
            &output,
            CliSidecarOutputRecord::chunk("stdout", "checkpointed\n"),
        )
        .unwrap();
        let exit_output = output.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            append_output(&exit_output, CliSidecarOutputRecord::exit(true, false)).unwrap();
        });
        let registry =
            crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("registry"));
        let (_cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
        let mut cursors = Vec::new();
        let result = super::follow_sidecar_output_from(
            &registry,
            "task",
            &output,
            super::CliSidecarReplayCursor::default(),
            &mut cancel_rx,
            |_| {},
            |cursor| {
                cursors.push(cursor);
                Ok(())
            },
        )
        .await
        .unwrap();
        assert!(result.exit_ok);
        assert_eq!(result.stdout_text, "checkpointed\n");
        assert_eq!(cursors.len(), 1);
        assert_eq!(cursors[0].sequence, 1);
        let _ = std::fs::remove_dir_all(root);
    }
}
