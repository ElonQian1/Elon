use super::node_agent_cli_prompt_runner::{
    cli_prompt_delivery, cli_prompt_timeout_secs_with_config, codex_exec_args,
    write_and_close_cli_stdin, DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS,
};

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
