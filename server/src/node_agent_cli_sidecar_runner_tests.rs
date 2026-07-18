// server/src/node_agent_cli_sidecar_runner_tests.rs

use crate::{
    node_agent_cli_pty::{default_cols, default_rows},
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_io::{append_output, read_new_output_records, CliSidecarOutputRecord},
    node_agent_cli_sidecar_runner::{
        follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig, CliSidecarOutputEvent,
    },
    node_agent_task_journal::{TaskJournal, TaskJournalStart},
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
        runtime_permission: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
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
async fn pipe_json_sidecar_registers_child_and_keeps_streams_clean() {
    let root = temp_dir("runner-pipe-json");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-sidecar-pipe-json";
    let session_id = "sidecar-pipe-json";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = pipe_json_echo_command();

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    })
    .await
    .expect("pipe json sidecar should run shell child");

    let sessions = registry
        .latest_sessions(5)
        .expect("sidecar sessions should load");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, session_id);
    assert_eq!(sessions[0].state, "finished");
    assert_eq!(sessions[0].transport, "managed_pipe_json_sidecar");
    assert!(!sessions[0].capabilities.terminal_attach);
    assert!(sessions[0].capabilities.output_stream_replay);
    assert!(sessions[0].capabilities.cancel);
    assert!(sessions[0].child_pid.is_some());

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let mut events = Vec::new();
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |event| {
        events.push(event)
    })
    .await
    .expect("sidecar output should replay");

    assert!(result.exit_ok);
    assert!(result.stdout_text.contains("pipe-out"));
    assert!(result.stderr_text.contains("pipe-err"));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stdout(text) if text.contains("pipe-out")
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        CliSidecarOutputEvent::Stderr(text) if text.contains("pipe-err")
    )));
    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    assert!(records
        .iter()
        .any(|record| record.stream.as_deref() == Some("stdout")));
    assert!(records
        .iter()
        .any(|record| record.stream.as_deref() == Some("stderr")));
    assert!(!records
        .iter()
        .any(|record| record.stream.as_deref() == Some("pty")));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn managed_pipe_replays_real_multiline_stdin_echo_from_child_stdout() {
    let root = temp_dir("managed-pipe-stdin-echo");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-managed-pipe-stdin-echo";
    let session_id = "managed-pipe-stdin-echo";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = pipe_json_stdin_echo_command();
    let prompt = "第一行：npm node / codex stdin\n第二行：& | < > %PATH%\n第三行：真实回显完成\n";

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: Some(prompt.to_string()),
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    })
    .await
    .expect("managed pipe should write stdin to the real child");

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |_| {})
        .await
        .expect("managed pipe should replay echoed stdout");

    assert!(result.exit_ok);
    assert_eq!(result.stdout_text, prompt);
    assert!(result.stderr_text.is_empty());
    let session = registry
        .session_for_task(task_id)
        .expect("managed pipe session lookup should work")
        .expect("managed pipe session should exist");
    assert_eq!(session.transport, "managed_pipe_json_sidecar");
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn timeout_follower_waits_for_buffered_usage_and_real_exit() {
    let root = temp_dir("timeout-follower-drains-usage");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-timeout-usage";
    let output_path = root.join("timeout-output.jsonl");
    append_output(&output_path, CliSidecarOutputRecord::child_started(12_345)).unwrap();
    append_output(
        &output_path,
        CliSidecarOutputRecord::error("codex pipe sidecar 执行超时（超过 10 秒）".to_string()),
    )
    .unwrap();

    let delayed_output_path = output_path.clone();
    let writer = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        append_output(
            &delayed_output_path,
            CliSidecarOutputRecord::chunk(
                "stdout",
                concat!(
                    r#"{"type":"token_count","usage":{"input_tokens":40,"output_tokens":2,"total_tokens":42}}"#,
                    "\n",
                ),
            ),
        )
        .unwrap();
        append_output(
            &delayed_output_path,
            CliSidecarOutputRecord::exit(false, true),
        )
        .unwrap();
    });

    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        follow_sidecar_output(&registry, task_id, &output_path, &mut cancel_rx, |_| {}),
    )
    .await
    .expect("follower should wait for the real exit")
    .expect("sidecar output should parse");
    writer.await.unwrap();

    assert!(!result.exit_ok);
    assert!(result.canceled);
    assert!(result
        .terminal_error
        .as_deref()
        .is_some_and(|error| error.contains("执行超时")));
    let usage = crate::cli_usage::parse_cli_usage(&result.stdout_text).unwrap();
    assert_eq!(usage.input_tokens, 40);
    assert_eq!(usage.output_tokens, 2);
    assert_eq!(usage.total_tokens, 42);
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
        runtime_permission: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
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
        runtime_permission: None,
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
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

#[tokio::test]
async fn sidecar_runner_persists_codex_session_from_pty_output() {
    let root = temp_dir("runner-codex-session");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let journal_dir = root.join("journal");
    let journal = TaskJournal::new(&journal_dir);
    let task_id = "task-sidecar-codex-session";
    let session_id = "sidecar-codex-session";
    let codex_session = "019f172c-2d52-7e33-8ce5-5af73dada2bf";
    let scope_key = "scope-codex-session";
    let legacy_file = root.join("legacy-codex-sessions.json");
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = codex_session_echo_command(codex_session);

    journal
        .record_started(TaskJournalStart {
            req_id: task_id,
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some(task_id),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("task should be registered before sidecar output");

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: Some("project_write".to_string()),
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(journal_dir.clone()),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: Some(scope_key.to_string()),
        legacy_codex_sessions_file: Some(legacy_file.clone()),
        timeout_secs: 10,
        stdin_payload: None,
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    })
    .await
    .expect("sidecar should persist codex session");

    let snapshot = journal
        .snapshot(task_id, 0, 50)
        .expect("task journal snapshot should load");
    let record = snapshot
        .record
        .expect("task record should remain available");
    assert_eq!(record.codex_session_id.as_deref(), Some(codex_session));
    assert_eq!(record.codex_session_scope_key.as_deref(), Some(scope_key));
    assert_eq!(
        journal
            .load_codex_session(scope_key)
            .expect("codex session cache should load")
            .as_deref(),
        Some(codex_session)
    );
    assert!(fs::read_to_string(&legacy_file)
        .expect("legacy session cache should be written")
        .contains(codex_session));

    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    let visible = records
        .iter()
        .filter_map(|record| record.text.as_deref())
        .collect::<String>();
    assert!(!visible.contains(codex_session));
    assert!(visible.contains("codex-ready"));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn sidecar_runner_records_codex_approval_prompt_and_decision() {
    let root = temp_dir("runner-codex-approval");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let journal_dir = root.join("journal");
    let journal = TaskJournal::new(&journal_dir);
    let task_id = "task-sidecar-codex-approval";
    let session_id = "sidecar-codex-approval";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = codex_approval_prompt_command();

    journal
        .record_started(TaskJournalStart {
            req_id: task_id,
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some(task_id),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("task should be registered before sidecar output");

    let run = tokio::spawn(run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: Some("project_write".to_string()),
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(journal_dir.clone()),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    }));

    wait_for_pending_approval(&journal, task_id, "sidecar_tap_1").await;
    let waiting = registry
        .session_for_task(task_id)
        .expect("sidecar session lookup should work")
        .expect("sidecar session should exist");
    assert_eq!(waiting.state, "waiting_approval");
    assert!(registry
        .record_tool_approval_decision(task_id, "sidecar_tap_1", "approve")
        .expect("approval decision should be queued"));

    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .expect("sidecar should finish")
        .expect("join should succeed")
        .expect("sidecar run should succeed");

    let snapshot = journal
        .snapshot(task_id, 0, 100)
        .expect("task journal snapshot should load");
    assert_eq!(snapshot.approvals.pending_count, 0);
    assert_eq!(snapshot.approvals.decided_count, 1);
    assert_eq!(snapshot.approvals.approvals[0].approval_id, "sidecar_tap_1");
    assert_eq!(snapshot.approvals.approvals[0].status, "approved");
    assert_eq!(
        snapshot.approvals.approvals[0]
            .checkpoint
            .as_ref()
            .expect("sidecar checkpoint should be preserved")["restart_recovery"]["next_action"],
        "approve_or_deny_sidecar_waiter"
    );

    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    let runtime_text = records
        .iter()
        .filter(|record| record.stream.as_deref() == Some("runtime"))
        .filter_map(|record| record.text.as_deref())
        .collect::<String>();
    assert!(runtime_text.contains(r#""type":"tool_approval_required""#));
    assert!(runtime_text.contains(r#""type":"tool_approval_decision""#));
    assert!(runtime_text.contains(r#""decision":"approve""#));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn sidecar_runner_skips_codex_approval_prompt_in_danger_full_access() {
    let root = temp_dir("runner-codex-approval-danger");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let journal_dir = root.join("journal");
    let journal = TaskJournal::new(&journal_dir);
    let task_id = "task-sidecar-codex-approval-danger";
    let session_id = "sidecar-codex-approval-danger";
    let output_path = registry.output_path(task_id, session_id);
    let (program, args) = codex_approval_text_command();

    journal
        .record_started(TaskJournalStart {
            req_id: task_id,
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some(task_id),
            cwd: Some("D:/demo"),
            runtime_permission: Some("danger_full_access"),
        })
        .expect("task should be registered before sidecar output");

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program,
        args,
        cwd: None,
        runtime_permission: Some("danger_full_access".to_string()),
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(journal_dir.clone()),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    })
    .await
    .expect("sidecar should run approval-like output");

    let snapshot = journal
        .snapshot(task_id, 0, 100)
        .expect("task journal snapshot should load");
    assert_eq!(snapshot.approvals.approvals.len(), 0);

    let mut offset = 0;
    let records = read_new_output_records(&output_path, &mut offset)
        .expect("sidecar output records should load");
    let runtime_text = records
        .iter()
        .filter(|record| record.stream.as_deref() == Some("runtime"))
        .filter_map(|record| record.text.as_deref())
        .collect::<String>();
    assert!(!runtime_text.contains(r#""type":"tool_approval_required""#));

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

fn pipe_json_echo_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "echo pipe-out && echo pipe-err 1>&2".to_string(),
                "--json".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "echo pipe-out; echo pipe-err >&2".to_string(),
                "--json".to_string(),
            ],
        )
    }
}

fn codex_approval_prompt_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "$e=[char]27; Write-Output \"$e[33mAllow command?$e[0m\"; Write-Output '$ echo sidecar-approved'; Write-Output '[y/N]'; $x=[Console]::In.ReadLine(); Write-Output \"decision:$x\"".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "printf '\\033[33mAllow command?\\033[0m\\n$ echo sidecar-approved\\n[y/N]\\n'; read x; printf 'decision:%s\\n' \"$x\"".to_string(),
            ],
        )
    }
}

fn codex_approval_text_command() -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "$e=[char]27; Write-Output \"$e[33mAllow command?$e[0m\"; Write-Output '$ echo should-not-request-approval'; Write-Output '[y/N]'; Write-Output 'done'".to_string(),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "printf '\\033[33mAllow command?\\033[0m\\n$ echo should-not-request-approval\\n[y/N]\\ndone\\n'".to_string(),
            ],
        )
    }
}

fn codex_session_echo_command(session_id: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!(
                    "$e=[char]27; Write-Output \"$e[36mSession ID: {session_id}$e[0m\"; Write-Output \"codex-ready\""
                ),
            ],
        )
    } else {
        (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                format!("printf '\\033[36mSession ID: {session_id}\\033[0m\\ncodex-ready\\n'"),
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

fn pipe_json_stdin_echo_command() -> (String, Vec<String>) {
    (
        "node".to_string(),
        vec![
            "-e".to_string(),
            "process.stdin.pipe(process.stdout)".to_string(),
            "--".to_string(),
            "--json".to_string(),
        ],
    )
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

async fn wait_for_pending_approval(journal: &TaskJournal, task_id: &str, approval_id: &str) {
    for _ in 0..80 {
        if let Ok(snapshot) = journal.snapshot(task_id, 0, 50) {
            if snapshot
                .approvals
                .pending_approval_ids()
                .iter()
                .any(|id| id == approval_id)
            {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("sidecar approval {approval_id} did not become pending");
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
