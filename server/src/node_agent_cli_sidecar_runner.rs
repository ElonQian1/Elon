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
                        Some("stdout") => {
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

    let mut cmd = tokio::process::Command::new(&config.program);
    cmd.args(&config.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = config
        .cwd
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        cmd.current_dir(cwd);
    }
    for (key, value) in &config.env {
        cmd.env(key, value);
    }
    if config.stdin_piped_empty {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    hide_tokio_command_window(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
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
    if config.stdin_piped_empty {
        let _ = child.stdin.take();
    }

    let child_pid = child.id();
    if let Some(pid) = child_pid {
        let _ = registry.touch_session(&config.session_id, Some("running"), Some(pid));
        let _ = task_journal.record_process_started(&config.task_id, pid);
        append_output(
            &config.output_path,
            CliSidecarOutputRecord::child_started(pid),
        )?;
    }

    let stdout = child
        .stdout
        .take()
        .context("sidecar child stdout missing")?;
    let stderr = child
        .stderr
        .take()
        .context("sidecar child stderr missing")?;
    let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<SidecarLine>();
    spawn_stdout_reader(stdout, line_tx.clone());
    spawn_stderr_reader(stderr, line_tx);

    let mut mailbox_offset = 0_u64;
    let mut processed_commands = HashSet::new();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut canceled = false;
    let mut interval = tokio::time::interval(Duration::from_secs(SIDECAR_HEARTBEAT_SECS));
    let timeout = tokio::time::sleep(Duration::from_secs(config.timeout_secs.max(1)));
    tokio::pin!(timeout);

    let success = loop {
        tokio::select! {
            _ = interval.tick() => {
                let state = if canceled { "cancel_requested" } else { "running" };
                let _ = registry.touch_session(&config.session_id, Some(state), child_pid);
                consume_mailbox(
                    &registry,
                    &config,
                    &mut mailbox_offset,
                    &mut processed_commands,
                    &mut child,
                    &mut canceled,
                )?;
            }
            _ = &mut timeout => {
                canceled = true;
                let _ = child.kill().await;
                append_output(
                    &config.output_path,
                    CliSidecarOutputRecord::error(format!(
                        "{} sidecar 执行超时（超过 {} 秒）",
                        config.cli_name, config.timeout_secs
                    )),
                )?;
            }
            line = line_rx.recv() => {
                match line {
                    Some(SidecarLine::Stdout(text)) => {
                        write_chunk(&config, &task_journal, "stdout", &text)?;
                    }
                    Some(SidecarLine::Stderr(text)) => {
                        write_chunk(&config, &task_journal, "stderr", &text)?;
                    }
                    Some(SidecarLine::StdoutDone) => stdout_done = true,
                    Some(SidecarLine::StderrDone) => stderr_done = true,
                    None => {
                        stdout_done = true;
                        stderr_done = true;
                    }
                }
            }
            status = child.wait(), if stdout_done && stderr_done => {
                break status.map(|status| status.success()).unwrap_or(false);
            }
        }

        if let Some(status) = child.try_wait()? {
            if stdout_done && stderr_done {
                break status.success();
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
    let is_codex_session_id =
        config.cli_name == "codex" && text.trim_start().starts_with("session id: ");
    if !is_codex_session_id {
        let _ = task_journal.record_cli_chunk(&config.task_id, stream, text);
    }
    Ok(())
}

fn consume_mailbox(
    registry: &CliSidecarRegistry,
    config: &CliSidecarLaunchConfig,
    offset: &mut u64,
    processed: &mut HashSet<String>,
    child: &mut tokio::process::Child,
    canceled: &mut bool,
) -> Result<()> {
    let path = registry.command_mailbox_path(&config.task_id);
    for command in read_new_commands(&path, offset)? {
        let key = format!(
            "{}:{}:{}",
            command.command,
            command.approval_id.as_deref().unwrap_or(""),
            command.at_ms
        );
        if !processed.insert(key) {
            continue;
        }
        match command.command.as_str() {
            "cancel" => {
                *canceled = true;
                let _ = child.start_kill();
                let _ = registry.touch_session(&config.session_id, Some("cancel_requested"), None);
            }
            "tool_approval_decision" => {
                let _ = registry.touch_session(&config.session_id, Some("running"), None);
            }
            _ => {}
        }
    }
    Ok(())
}

fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    tx: tokio::sync::mpsc::UnboundedSender<SidecarLine>,
) {
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = tx.send(SidecarLine::Stdout(format!("{line}\n")));
                }
                Ok(None) | Err(_) => {
                    let _ = tx.send(SidecarLine::StdoutDone);
                    break;
                }
            }
        }
    });
}

fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    tx: tokio::sync::mpsc::UnboundedSender<SidecarLine>,
) {
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => {
                    let _ = tx.send(SidecarLine::StderrDone);
                    break;
                }
                Ok(_) => {
                    while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
                        buf.pop();
                    }
                    let text = format!("{}\n", String::from_utf8_lossy(&buf));
                    let _ = tx.send(SidecarLine::Stderr(text));
                }
            }
        }
    });
}

#[derive(Debug)]
enum SidecarLine {
    Stdout(String),
    Stderr(String),
    StdoutDone,
    StderrDone,
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
