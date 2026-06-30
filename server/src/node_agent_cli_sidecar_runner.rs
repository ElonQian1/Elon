// server/src/node_agent_cli_sidecar_runner.rs

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
        append_output, read_new_commands, read_new_output_records, CliSidecarOutputRecord,
    },
    node_agent_task_journal::TaskJournal,
};

const SIDECAR_HEARTBEAT_SECS: u64 = 5;
const SIDECAR_POLL_MS: u64 = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CliSidecarLaunchConfig {
    pub session_id: String,
    pub task_id: String,
    pub cli_name: String,
    pub route: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    pub output_path: PathBuf,
    pub registry_dir: PathBuf,
    #[serde(default)]
    pub task_journal_dir: Option<PathBuf>,
    pub timeout_secs: u64,
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
}

#[derive(Debug, Clone)]
pub(crate) struct CliSidecarRunResult {
    pub exit_ok: bool,
    pub stdout_text: String,
    pub stderr_text: String,
    pub canceled: bool,
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

pub(crate) fn session_id_for_task(task_id: &str) -> String {
    format!("sidecar-{}-{}", safe_id_fragment(task_id), now_ms())
}

pub(crate) async fn spawn_sidecar(config: CliSidecarLaunchConfig) -> Result<CliSidecarLaunch> {
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

    let exe = std::env::current_exe().context("读取当前 node-agent exe 路径")?;
    let mut cmd = tokio::process::Command::new(exe);
    cmd.arg("--cli-sidecar")
        .arg(&config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_tokio_command_window(&mut cmd);
    let child = cmd.spawn().context("启动 CLI sidecar 进程")?;
    Ok(CliSidecarLaunch {
        sidecar_pid: child.id(),
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
    mut on_event: impl FnMut(CliSidecarOutputEvent),
) -> Result<CliSidecarRunResult> {
    let mut offset = 0_u64;
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut exit_ok = None;
    let mut canceled = false;

    loop {
        for record in read_new_output_records(output_path, &mut offset)? {
            match record.record_type.as_str() {
                "chunk" => {
                    let text = record.text.unwrap_or_default();
                    match record.stream.as_deref() {
                        Some("stdout") | Some("pty") => {
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
                        on_event(CliSidecarOutputEvent::ChildStarted(pid));
                    }
                }
                "exit" => {
                    exit_ok = Some(record.success.unwrap_or(false));
                    canceled = record.canceled.unwrap_or(false);
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

pub(crate) async fn run_sidecar(config: CliSidecarLaunchConfig) -> Result<()> {
    let registry = CliSidecarRegistry::new(config.registry_dir.clone());
    let task_journal = config
        .task_journal_dir
        .clone()
        .map(TaskJournal::new)
        .unwrap_or_else(TaskJournal::default);
    let started_at = now_ms();
    registry.upsert_session(CliSidecarSessionRecord::managed_conpty(
        &config.session_id,
        &config.task_id,
        &config.cli_name,
        &config.route,
        config.cwd.clone(),
        Some(config.output_path.to_string_lossy().to_string()),
        Some(std::process::id()),
        None,
        started_at,
    ))?;

    let mut pty = match CliPtyProcess::spawn(CliPtySpawnConfig {
        program: &config.program,
        args: &config.args,
        cwd: config.cwd.as_deref(),
        env: &config.env,
        cols: config.initial_cols,
        rows: config.initial_rows,
    }) {
        Ok(pty) => pty,
        Err(error) => {
            let message = format!("无法启动 sidecar CLI {}: {error}", config.program);
            append_output(
                &config.output_path,
                CliSidecarOutputRecord::error(message.clone()),
            )?;
            let _ = registry.mark_task_terminal(&config.task_id, "failed");
            return Err(anyhow::anyhow!(message));
        }
    };

    let child_pid = pty.child_pid();
    if let Some(pid) = child_pid {
        let _ = registry.touch_session(&config.session_id, Some("running"), Some(pid));
        let _ = task_journal.record_process_started(&config.task_id, pid);
        append_output(
            &config.output_path,
            CliSidecarOutputRecord::child_started(pid),
        )?;
    }
    let mut pty_output_rx = pty.take_output_rx();

    let mut mailbox_offset = 0_u64;
    let mut processed_commands = HashSet::new();
    let mut reader_closed = false;
    let mut child_exit = None;
    let mut child_exit_observed_at = None;
    let mut canceled = false;
    let mut timed_out = false;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(SIDECAR_HEARTBEAT_SECS));
    let mut mailbox = tokio::time::interval(Duration::from_millis(SIDECAR_POLL_MS));
    let timeout = tokio::time::sleep(Duration::from_secs(config.timeout_secs.max(1)));
    tokio::pin!(timeout);

    let success = loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let state = if canceled { "cancel_requested" } else { "running" };
                let _ = registry.touch_session(&config.session_id, Some(state), child_pid);
            }
            _ = mailbox.tick() => {
                consume_mailbox(
                    &registry,
                    &config,
                    &mut mailbox_offset,
                    &mut processed_commands,
                    &mut pty,
                    &mut canceled,
                )?;
                if child_exit.is_none() {
                    child_exit = pty.try_wait()?;
                    if child_exit.is_some() {
                        child_exit_observed_at = Some(tokio::time::Instant::now());
                    }
                }
            }
            _ = &mut timeout, if !timed_out => {
                timed_out = true;
                canceled = true;
                let _ = pty.kill();
                append_output(
                    &config.output_path,
                    CliSidecarOutputRecord::error(format!(
                        "{} sidecar 执行超时（超过 {} 秒）",
                        config.cli_name, config.timeout_secs
                    )),
                )?;
            }
            event = pty_output_rx.recv() => {
                match event {
                    Some(CliPtyEvent::Output(text)) => {
                        write_pty_chunk(&config, &task_journal, &mut pty, &text)?;
                    }
                    Some(CliPtyEvent::ReaderError(error)) => {
                        append_output(
                            &config.output_path,
                            CliSidecarOutputRecord::chunk("pty_error", &format!("{error}\n")),
                        )?;
                        reader_closed = true;
                    }
                    Some(CliPtyEvent::ReaderClosed) | None => reader_closed = true,
                }
            }
        }

        if child_exit.is_none() {
            child_exit = pty.try_wait()?;
            if child_exit.is_some() {
                child_exit_observed_at = Some(tokio::time::Instant::now());
            }
        }
        if let Some(success) = child_exit {
            let drained_after_exit = child_exit_observed_at
                .is_some_and(|instant| instant.elapsed() >= Duration::from_millis(500));
            if reader_closed || canceled || drained_after_exit {
                break success;
            }
        }
    };

    let state = if canceled {
        "canceled"
    } else if success {
        "finished"
    } else {
        "failed"
    };
    let _ = registry.mark_task_terminal(&config.task_id, state);
    append_output(
        &config.output_path,
        CliSidecarOutputRecord::exit(success, canceled),
    )?;
    Ok(())
}

fn write_pty_chunk(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    pty: &mut CliPtyProcess,
    text: &str,
) -> Result<()> {
    if text.contains("\x1b[6n") {
        pty.write_input("\x1b[1;1R")?;
    }
    let visible = text.replace("\x1b[6n", "");
    if visible.is_empty() {
        return Ok(());
    }
    write_chunk(config, task_journal, "pty", &visible)
}

fn write_chunk(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    stream: &str,
    text: &str,
) -> Result<()> {
    append_output(
        &config.output_path,
        CliSidecarOutputRecord::chunk(stream, text),
    )?;
    let journal_stream = if stream == "pty" { "stdout" } else { stream };
    let is_codex_session_id =
        config.cli_name == "codex" && text.trim_start().starts_with("session id: ");
    if !is_codex_session_id {
        let _ = task_journal.record_cli_chunk(&config.task_id, journal_stream, text);
    }
    Ok(())
}

fn consume_mailbox(
    registry: &CliSidecarRegistry,
    config: &CliSidecarLaunchConfig,
    offset: &mut u64,
    processed: &mut HashSet<String>,
    pty: &mut CliPtyProcess,
    canceled: &mut bool,
) -> Result<()> {
    let path = registry.command_mailbox_path(&config.task_id);
    for command in read_new_commands(&path, offset)? {
        let key = command.command_id.clone().unwrap_or_else(|| {
            format!(
                "{}:{}:{}:{}:{}:{}",
                command.command,
                command.approval_id.as_deref().unwrap_or(""),
                command.text.as_deref().unwrap_or(""),
                command.cols.unwrap_or_default(),
                command.rows.unwrap_or_default(),
                command.at_ms
            )
        });
        if !processed.insert(key) {
            continue;
        }
        match command.command.as_str() {
            "cancel" => {
                *canceled = true;
                let _ = pty.kill();
                let _ = registry.touch_session(&config.session_id, Some("cancel_requested"), None);
            }
            "terminal_input" => {
                if let Some(text) = command.text.as_deref() {
                    pty.write_input(text)?;
                    let _ = registry.touch_session(&config.session_id, Some("running"), None);
                }
            }
            "terminal_resize" => {
                if let (Some(cols), Some(rows)) = (command.cols, command.rows) {
                    pty.resize(cols, rows)?;
                    let _ = registry.touch_session(&config.session_id, Some("running"), None);
                }
            }
            "tool_approval_decision" => {
                if let Some(input) = command
                    .decision
                    .as_deref()
                    .and_then(approval_decision_input)
                {
                    pty.write_input(input)?;
                }
                let _ = registry.touch_session(&config.session_id, Some("running"), None);
            }
            _ => {}
        }
    }
    Ok(())
}

fn approval_decision_input(decision: &str) -> Option<&'static str> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Some("y\r"),
        "deny" | "denied" | "reject" | "rejected" => Some("n\r"),
        _ => None,
    }
}

fn config_path_for(session_id: &str) -> PathBuf {
    super::state_path()
        .with_file_name("cli-sidecars")
        .join("configs")
        .join(format!("{}.json", safe_id_fragment(session_id)))
}

fn safe_id_fragment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(windows)]
fn hide_tokio_command_window(command: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_tokio_command_window(_command: &mut tokio::process::Command) {}
