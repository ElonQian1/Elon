use super::super::state_path;
use super::*;

pub(super) async fn run_pty_sidecar(config: CliSidecarLaunchConfig) -> Result<()> {
    let registry = CliSidecarRegistry::new(config.registry_dir.clone());
    let task_journal = config
        .task_journal_dir
        .clone()
        .map(TaskJournal::new)
        .unwrap_or_else(TaskJournal::default);
    let started_at = now_ms();
    let mut session = CliSidecarSessionRecord::managed_conpty(
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

    let mut codex_approval_tracker = CodexApprovalTracker::default();
    let mut codex_session_capture = CodexSessionCapture::default();
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
                let state = if canceled {
                    "cancel_requested"
                } else if codex_approval_tracker.has_pending() {
                    "waiting_approval"
                } else {
                    "running"
                };
                let _ = registry.touch_session(&config.session_id, Some(state), child_pid);
            }
            _ = mailbox.tick() => {
                consume_mailbox(
                    &registry,
                    &config,
                    &task_journal,
                    &mut codex_approval_tracker,
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
                        write_pty_chunk(
                            &config,
                            &task_journal,
                            &registry,
                            &mut codex_approval_tracker,
                            &mut codex_session_capture,
                            &mut pty,
                            &text,
                        )?;
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
            // Cancellation/timeout must not skip the final PTY drain: Codex may
            // have emitted its token-count event immediately before termination.
            if reader_closed || drained_after_exit {
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

pub(super) fn write_pty_chunk(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    registry: &CliSidecarRegistry,
    codex_approval_tracker: &mut CodexApprovalTracker,
    codex_session_capture: &mut CodexSessionCapture,
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
    write_chunk(
        config,
        task_journal,
        registry,
        codex_approval_tracker,
        codex_session_capture,
        "pty",
        &visible,
    )
}

pub(super) fn write_chunk(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    registry: &CliSidecarRegistry,
    codex_approval_tracker: &mut CodexApprovalTracker,
    codex_session_capture: &mut CodexSessionCapture,
    stream: &str,
    text: &str,
) -> Result<()> {
    let (session_from_line, visible_text) = if config.cli_name == "codex" {
        node_agent_codex_session::strip_session_id_lines(text)
    } else {
        (None, text.to_string())
    };
    let session_from_capture = if config.cli_name == "codex" {
        codex_session_capture.observe(text)
    } else {
        None
    };
    let session_id = session_from_line.or(session_from_capture);
    if let (Some(scope_key), Some(session_id)) = (
        config.codex_session_scope_key.as_deref(),
        session_id.as_deref(),
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
    let journal_stream = if stream == "pty" { "stdout" } else { stream };
    let _ = task_journal.record_cli_chunk(&config.task_id, journal_stream, &visible_text);
    if config.cli_name == "codex" && !route_a_full_access(config.runtime_permission.as_deref()) {
        if let Some(event) = codex_approval_tracker.observe_output(
            &config.task_id,
            &config.session_id,
            &visible_text,
            now_ms(),
        ) {
            write_tool_event(config, task_journal, &event)?;
            let _ = registry.touch_session(&config.session_id, Some("waiting_approval"), None);
        }
    }
    Ok(())
}

pub(super) fn route_a_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("full_access" | "danger_full_access")
    )
}

pub(super) fn write_tool_event(
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    event: &serde_json::Value,
) -> Result<()> {
    let text = format!("{}\n", serde_json::to_string(event)?);
    append_output(
        &config.output_path,
        CliSidecarOutputRecord::chunk("runtime", &text),
    )?;
    let _ = task_journal.record_cli_chunk(&config.task_id, "runtime", &text);
    Ok(())
}

pub(super) fn consume_mailbox(
    registry: &CliSidecarRegistry,
    config: &CliSidecarLaunchConfig,
    task_journal: &TaskJournal,
    codex_approval_tracker: &mut CodexApprovalTracker,
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
                if let Some(event) = codex_approval_tracker.observe_decision(
                    &config.task_id,
                    command.approval_id.as_deref(),
                    command.decision.as_deref(),
                    now_ms(),
                ) {
                    write_tool_event(config, task_journal, &event)?;
                }
                if let Some(input) = command
                    .decision
                    .as_deref()
                    .and_then(approval_decision_input)
                {
                    pty.write_input(input)?;
                }
                let state = if codex_approval_tracker.has_pending() {
                    "waiting_approval"
                } else {
                    "running"
                };
                let _ = registry.touch_session(&config.session_id, Some(state), None);
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn approval_decision_input(decision: &str) -> Option<&'static str> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Some("y\r"),
        "deny" | "denied" | "reject" | "rejected" => Some("n\r"),
        _ => None,
    }
}

pub(super) fn config_path_for(session_id: &str) -> PathBuf {
    state_path()
        .with_file_name("cli-sidecars")
        .join("configs")
        .join(format!("{}.json", safe_id_fragment(session_id)))
}

pub(super) fn safe_id_fragment(value: &str) -> String {
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
