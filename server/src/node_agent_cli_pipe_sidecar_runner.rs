// server/src/node_agent_cli_pipe_sidecar_runner.rs

use anyhow::Result;
use std::{
    collections::HashSet,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Child,
    sync::mpsc,
};

use crate::{
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry, CliSidecarSessionRecord},
    node_agent_cli_sidecar_io::{append_output, read_new_commands, CliSidecarOutputRecord},
    node_agent_cli_sidecar_runner::{
        hide_tokio_command_window,
        terminal_hint::{CodexTerminalHint, CodexTerminalOutcome},
        CliSidecarLaunchConfig, SIDECAR_POLL_MS,
    },
    node_agent_codex_session,
    node_agent_task_journal::TaskJournal,
};

const CODEX_TERMINAL_DRAIN_GRACE: Duration = Duration::from_millis(750);
const CODEX_FINAL_MESSAGE_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug)]
enum PipeReadEvent {
    Chunk(&'static str, String),
    Closed(&'static str),
}

pub(crate) async fn run_pipe_json_sidecar(config: CliSidecarLaunchConfig) -> Result<()> {
    let registry = CliSidecarRegistry::new(config.registry_dir.clone());
    let task_journal = config
        .task_journal_dir
        .clone()
        .map(TaskJournal::new)
        .unwrap_or_else(TaskJournal::default);
    let started_at = now_ms();
    let mut session = CliSidecarSessionRecord::managed_pipe_json(
        &config.session_id,
        &config.task_id,
        &config.cli_name,
        &config.route,
        config.cwd.clone(),
        Some(config.output_path.to_string_lossy().to_string()),
        Some(std::process::id()),
        None,
        started_at,
    );
    session.worker_path = config
        .worker_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    session.worker_release = config.worker_release.clone();
    session.worker_sha256 = config.worker_sha256.clone();
    registry.upsert_session(session)?;

    let mut command = tokio::process::Command::new(&config.program);
    command
        .args(&config.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(
            if config.stdin_payload.is_some() || config.stdin_piped_empty {
                Stdio::piped()
            } else {
                Stdio::null()
            },
        );
    if let Some(cwd) = config.cwd.as_deref() {
        command.current_dir(cwd);
    }
    for (key, value) in &config.env {
        command.env(key, value);
    }
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    hide_tokio_command_window(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let message = format!("无法启动 pipe sidecar CLI {}: {error}", config.program);
            append_output(
                &config.output_path,
                CliSidecarOutputRecord::error(message.clone()),
            )?;
            let _ = registry.mark_task_terminal(&config.task_id, "failed");
            return Err(anyhow::anyhow!(message));
        }
    };
    if let Err(error) = crate::node_agent_cli_prompt_runner::write_and_close_cli_stdin(
        &mut child,
        config.stdin_payload.as_deref(),
    )
    .await
    {
        let message = format!(
            "无法写入并关闭 pipe sidecar CLI {} stdin: {error}",
            config.cli_name
        );
        let _ = child.start_kill();
        let _ = registry.mark_task_terminal(&config.task_id, "failed");
        append_output(&config.output_path, CliSidecarOutputRecord::error(message))?;
        return Ok(());
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
        .ok_or_else(|| anyhow::anyhow!("无法读取 pipe sidecar stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("无法读取 pipe sidecar stderr"))?;
    let (pipe_tx, mut pipe_rx) = mpsc::unbounded_channel();
    spawn_pipe_reader(stdout, "stdout", pipe_tx.clone());
    spawn_pipe_reader(stderr, "stderr", pipe_tx);

    let mut mailbox_offset = 0_u64;
    let mut processed_commands = HashSet::new();
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    let mut child_exit = None;
    let mut child_exit_observed_at = None;
    let mut canceled = false;
    let mut timed_out = false;
    let mut protocol_outcome = None;
    let mut terminal_hint = CodexTerminalHint::default();
    let runtime_policy = config.runtime_policy.clone().unwrap_or_else(|| {
        crate::node_agent_cli_runtime_policy::CliRuntimePolicy::fixed(config.timeout_secs.max(1))
    });
    let mut heartbeat =
        tokio::time::interval(Duration::from_secs(runtime_policy.heartbeat_secs.max(1)));
    let mut mailbox = tokio::time::interval(Duration::from_millis(SIDECAR_POLL_MS));
    let total_deadline =
        tokio::time::Instant::now() + Duration::from_secs(runtime_policy.total_timeout_secs.max(1));
    let mut idle_deadline =
        tokio::time::Instant::now() + Duration::from_secs(runtime_policy.idle_timeout_secs.max(1));

    let success = loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let state = if canceled { "cancel_requested" } else { "running" };
                let _ = registry.touch_session(&config.session_id, Some(state), child_pid);
                append_output(&config.output_path, CliSidecarOutputRecord::heartbeat())?;
            }
            _ = mailbox.tick() => {
                consume_pipe_mailbox(
                    &registry,
                    &config,
                    &mut mailbox_offset,
                    &mut processed_commands,
                    &mut child,
                    &mut canceled,
                )?;
                if child_exit.is_none() {
                    child_exit = child.try_wait()?;
                    if child_exit.is_some() {
                        child_exit_observed_at = Some(tokio::time::Instant::now());
                    }
                }
            }
            _ = tokio::time::sleep_until(total_deadline), if !timed_out && protocol_outcome.is_none() => {
                timed_out = true;
                crate::node_agent_cli_runtime_policy::terminate_process_tree(child_pid);
                let _ = child.start_kill();
                append_output(
                    &config.output_path,
                    CliSidecarOutputRecord::error(format!(
                        "{} pipe sidecar 达到总时限（{} 秒）",
                        config.cli_name, runtime_policy.total_timeout_secs
                    )),
                )?;
            }
            _ = tokio::time::sleep_until(idle_deadline), if runtime_policy.progress_aware && !timed_out && protocol_outcome.is_none() => {
                timed_out = true;
                crate::node_agent_cli_runtime_policy::terminate_process_tree(child_pid);
                let _ = child.start_kill();
                append_output(
                    &config.output_path,
                    CliSidecarOutputRecord::error(format!(
                        "{} pipe sidecar 空闲超时（连续 {} 秒没有输出、命令或文件进展）",
                        config.cli_name, runtime_policy.idle_timeout_secs
                    )),
                )?;
            }
            event = pipe_rx.recv() => {
                match event {
                    Some(PipeReadEvent::Chunk(stream, text)) => {
                        if config.cli_name == "codex" && stream == "stdout" {
                            terminal_hint.observe(&text, Instant::now());
                        }
                        let observation = crate::node_agent_cli_output_aggregate::progress_observation(&text);
                        if observation.progress {
                            idle_deadline = tokio::time::Instant::now()
                                + Duration::from_secs(runtime_policy.idle_timeout_secs.max(1));
                        }
                        write_pipe_chunk(&config, &task_journal, stream, &text)?;
                    }
                    Some(PipeReadEvent::Closed("stdout")) => stdout_closed = true,
                    Some(PipeReadEvent::Closed("stderr")) => stderr_closed = true,
                    Some(PipeReadEvent::Closed(_)) | None => {}
                }
            }
        }

        if protocol_outcome.is_none() && config.cli_name == "codex" {
            if let Some(outcome) = terminal_hint.outcome(
                Instant::now(),
                CODEX_TERMINAL_DRAIN_GRACE,
                CODEX_FINAL_MESSAGE_GRACE,
            ) {
                protocol_outcome = Some(outcome == CodexTerminalOutcome::Success);
                crate::node_agent_cli_runtime_policy::terminate_process_tree(child_pid);
                let _ = child.start_kill();
            }
        }

        if child_exit.is_none() {
            child_exit = child.try_wait()?;
            if child_exit.is_some() {
                child_exit_observed_at = Some(tokio::time::Instant::now());
            }
        }
        if let Some(status) = child_exit {
            let drained_after_exit = child_exit_observed_at
                .is_some_and(|instant| instant.elapsed() >= Duration::from_millis(500));
            // Cancellation/timeout must still drain both pipes (or the bounded
            // post-exit grace) so an already-emitted usage event is preserved.
            if (stdout_closed && stderr_closed) || drained_after_exit {
                break protocol_outcome.unwrap_or_else(|| status.success());
            }
        }
    };

    let state = if timed_out {
        "failed"
    } else if canceled {
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

fn spawn_pipe_reader<R>(reader: R, stream: &'static str, tx: mpsc::UnboundedSender<PipeReadEvent>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf).await {
                Ok(0) | Err(_) => {
                    let _ = tx.send(PipeReadEvent::Closed(stream));
                    break;
                }
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf).into_owned();
                    let _ = tx.send(PipeReadEvent::Chunk(stream, text));
                }
            }
        }
    });
}

fn write_pipe_chunk(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    stream: &str,
    text: &str,
) -> Result<()> {
    let (session_from_line, visible_text) = if config.cli_name == "codex" {
        node_agent_codex_session::strip_session_id_lines(text)
    } else {
        (None, text.to_string())
    };
    if let (Some(scope_key), Some(session_id)) = (
        config.codex_session_scope_key.as_deref(),
        session_from_line.as_deref(),
    ) {
        node_agent_codex_session::persist_session_compat(
            task_journal,
            config.legacy_codex_sessions_file.as_deref(),
            &config.task_id,
            scope_key,
            session_id,
        );
    }
    if visible_text.is_empty() {
        return Ok(());
    }
    append_output(
        &config.output_path,
        CliSidecarOutputRecord::chunk(stream, &visible_text),
    )?;
    Ok(())
}

fn consume_pipe_mailbox(
    registry: &CliSidecarRegistry,
    config: &CliSidecarLaunchConfig,
    offset: &mut u64,
    processed: &mut HashSet<String>,
    child: &mut Child,
    canceled: &mut bool,
) -> Result<()> {
    let path = registry.command_mailbox_path(&config.task_id);
    for command in read_new_commands(&path, offset)? {
        if command
            .target_session_id
            .as_deref()
            .is_some_and(|session_id| session_id != config.session_id)
        {
            continue;
        }
        let key = command.command_id.clone().unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                command.command,
                command.text.as_deref().unwrap_or(""),
                command.at_ms
            )
        });
        if !processed.insert(key) {
            continue;
        }
        if command.command.as_str() == "cancel" {
            *canceled = true;
            crate::node_agent_cli_runtime_policy::terminate_process_tree(child.id());
            let _ = child.start_kill();
            let _ = registry.touch_session(&config.session_id, Some("cancel_requested"), None);
        }
    }
    Ok(())
}
