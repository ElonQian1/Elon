use super::node_agent_cli_prompt_runner::{
    cli_prompt_delivery, cli_prompt_timeout_secs_with_config, codex_exec_args,
    codex_session_scope_key_for_task, write_and_close_cli_stdin,
    DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS,
};
use super::{
    node_agent_cli_pty::{default_cols, default_rows},
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_runner::{follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig},
};
use tokio::sync::watch;

#[test]
fn codex_full_access_prompt_gets_development_timeout() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("danger_full_access"), false, None),
        1200
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), false, None),
        1200
    );
}

#[test]
fn ordinary_prompt_timeouts_stay_short() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("read_only"), false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", None, false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config(" Codex ", None, false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config(
            "copilot",
            Some("danger_full_access"),
            true,
            Some("7200"),
        ),
        180
    );
}

#[test]
fn supervised_codex_is_not_hard_killed_at_twenty_minutes() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, None),
        DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS
    );
    assert!(DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS > 20 * 60);
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, Some("7200")),
        7200
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, Some("1200")),
        DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS
    );
}

#[cfg(windows)]
#[test]
fn codex_cmd_wrapper_keeps_supervision_prompt_out_of_argv() {
    let prompt = "<elon-pc-executor>\r\n修复 prompt & 保持 | JSONL <clean> %PATH%!";
    let (program, mut args) = super::node_agent_cli_security::windows_batch_wrapper(
        r"C:\Users\tester\AppData\Roaming\npm\codex.cmd",
    )
    .expect("Windows codex.cmd should use cmd wrapper");
    args.extend(["exec".to_string(), "--json".to_string()]);
    let delivery = cli_prompt_delivery("codex", prompt);
    args.extend(delivery.args.clone());

    assert_eq!(program, "cmd");
    assert_eq!(args.last().map(String::as_str), Some("-"));
    assert!(!args.iter().any(|arg| arg == prompt));
    assert_eq!(delivery.stdin_payload.as_deref(), Some(prompt));
}

#[test]
fn codex_multiline_supervision_prompt_is_preserved_for_stdin() {
    let prompt = "<elon-pc-executor>\n验收一：读取 AGENTS.md\n验收二：FINALIZABLE=true\n";
    let delivery = cli_prompt_delivery("codex", prompt);

    assert_eq!(delivery.args, vec!["-"]);
    assert_eq!(delivery.stdin_payload.as_deref(), Some(prompt));
    assert!(!delivery.stdin_piped_empty);
}

#[test]
fn local_codex_task_always_gets_a_persisted_session_scope() {
    assert_eq!(
        codex_session_scope_key_for_task(
            "codex",
            "local-task-a",
            &[],
            Some("danger_full_access"),
            Some("D:\\repo"),
        )
        .as_deref(),
        Some("task:local-task-a")
    );
    assert_eq!(
        codex_session_scope_key_for_task(
            "copilot",
            "local-task-a",
            &[],
            Some("danger_full_access"),
            Some("D:\\repo"),
        ),
        None
    );
}

#[test]
fn codex_resume_keeps_thread_before_stdin_marker_in_exact_argument_sequence() {
    let last_message = std::path::Path::new("codex-last-message.txt");
    let delivery = cli_prompt_delivery("codex", "继续完成真实多行任务");
    let args = codex_exec_args(
        Some("019f0000-1111-7222-8333-444444444444"),
        Some(last_message),
        false,
        false,
        &[],
        &delivery.args,
    );

    assert_eq!(
        args,
        vec![
            "exec",
            "resume",
            "--json",
            "--output-last-message",
            "codex-last-message.txt",
            "--skip-git-repo-check",
            "019f0000-1111-7222-8333-444444444444",
            "-",
        ]
    );
    assert_eq!(
        delivery.stdin_payload.as_deref(),
        Some("继续完成真实多行任务")
    );
}

#[test]
fn codex_special_characters_are_not_interpreted_as_arguments() {
    let prompt =
        r#"中文 "quotes" 'single' & | < > ^ %PATH% !bang! (group) $env:TEMP `tick` \ slash"#;
    let delivery = cli_prompt_delivery("codex", prompt);

    assert_eq!(delivery.args, vec!["-"]);
    assert_eq!(delivery.stdin_payload.as_deref(), Some(prompt));
    assert!(!delivery.args.iter().any(|arg| arg.contains("quotes")));
}

#[tokio::test]
async fn codex_prompt_over_eight_thousand_characters_reaches_closed_stdin_intact() {
    let prompt = "0123456789abcdef\n".repeat(600);
    assert!(prompt.chars().count() > 8_000);
    let delivery = cli_prompt_delivery("codex", &prompt);
    let mut command = if cfg!(windows) {
        let mut command = tokio::process::Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            "[Console]::Out.Write([Console]::In.ReadToEnd())",
        ]);
        command
    } else {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "cat"]);
        command
    };
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn stdin echo child");
    write_and_close_cli_stdin(&mut child, delivery.stdin_payload.as_deref())
        .await
        .expect("write and close full prompt");
    let output = child.wait_with_output().await.expect("wait for stdin echo");

    assert!(output.status.success());
    assert_eq!(output.stdout, prompt.as_bytes());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn managed_pipe_replays_real_multiline_stdin_echo_from_child_stdout() {
    let root = std::env::temp_dir().join(format!(
        "elon-cli-managed-pipe-stdin-echo-{}-{}",
        std::process::id(),
        now_ms()
    ));
    std::fs::create_dir_all(&root).expect("temp dir should be created");
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = "task-managed-pipe-stdin-echo";
    let session_id = "managed-pipe-stdin-echo";
    let output_path = registry.output_path(task_id, session_id);
    let prompt = "第一行：npm node / codex stdin\n第二行：& | < > %PATH%\n第三行：真实回显完成\n";

    run_sidecar(CliSidecarLaunchConfig {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program: "node".to_string(),
        args: vec![
            "-e".to_string(),
            "process.stdin.pipe(process.stdout)".to_string(),
            "--".to_string(),
            "--json".to_string(),
        ],
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
        runtime_policy: None,
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
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn copilot_claude_and_gemini_keep_existing_prompt_arguments_and_empty_pipe() {
    let prompt = "line one\nline two & special";
    for cli_name in ["copilot", "claude", "gemini"] {
        let delivery = cli_prompt_delivery(cli_name, prompt);
        assert_eq!(delivery.args, vec!["-p", prompt]);
        assert!(delivery.stdin_payload.is_none());
        assert!(delivery.stdin_piped_empty);
    }
}
