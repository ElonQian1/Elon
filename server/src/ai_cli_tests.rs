use super::*;

#[test]
fn codex_exec_args_enable_json_output() {
    let args = vec![
        "exec".to_string(),
        "--sandbox".to_string(),
        "workspace-write".to_string(),
        "--skip-git-repo-check".to_string(),
    ];

    assert_eq!(
        codex_exec_json_args(&args),
        vec![
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "--skip-git-repo-check"
        ]
    );
}

#[test]
fn codex_resume_args_keep_supported_options() {
    let args = vec![
        "-m".to_string(),
        "gpt-5".to_string(),
        "exec".to_string(),
        "--sandbox".to_string(),
        "workspace-write".to_string(),
        "--skip-git-repo-check".to_string(),
    ];

    assert_eq!(
        codex_resume_args(&args, "thread-1").unwrap(),
        vec![
            "-m",
            "gpt-5",
            "exec",
            "resume",
            "--skip-git-repo-check",
            "--json",
            "thread-1"
        ]
    );
}

#[test]
fn extracts_codex_json_thread_and_answer() {
    let stdout = r#"{"type":"thread.started","thread_id":"thread-1"}
{"type":"item.completed","item":{"type":"agent_message","text":"hello"}}"#;

    assert_eq!(extract_thread_id(stdout).as_deref(), Some("thread-1"));
    assert_eq!(extract_json_agent_message(stdout).as_deref(), Some("hello"));
}

fn test_option() -> AiCliOption {
    AiCliOption {
        id: "codex_cli".into(),
        label: "Codex CLI".into(),
        provider: "codex".into(),
        model: None,
        bin: "codex".into(),
        args: vec!["exec".into(), "--skip-git-repo-check".into()],
        prompt_mode: CliPromptMode::Arg,
        timeout_secs: 60,
    }
}

#[test]
fn chat_prompt_uses_lightweight_mode() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "你好，随便聊聊",
        None,
        &test_option(),
        intent_router::CapabilityRoute::ChatAgent,
        false,
    );

    assert!(prompt.contains("轻量聊天模式"));
    assert!(!prompt.contains("通用项目工作流必须始终执行"));
    assert!(!prompt.contains("git pull --rebase"));
}

#[test]
fn prewarm_prompt_does_not_enter_project_workflow() {
    let prompt = build_prewarm_cli_prompt(Path::new("D:/tmp/project"), &test_option());

    assert!(prompt.contains("prewarming a Codex CLI native session"));
    assert!(prompt.contains("Do not inspect files"));
    assert!(!prompt.contains("git pull --rebase"));
    assert!(!prompt.contains("General project workflow"));
}

#[test]
fn development_prompt_keeps_project_workflow() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "帮我修改 APK 并发布新版",
        None,
        &test_option(),
        intent_router::CapabilityRoute::CodeAgent,
        false,
    );

    assert!(prompt.contains("通用项目工作流必须始终执行"));
    assert!(prompt.contains("git pull --rebase"));
    assert!(prompt.contains("scripts/publish-apk.ps1"));
    assert!(prompt.contains("不要 rebase 后继续上传旧 APK"));
    assert!(prompt.contains("服务器为本 APK 会话创建的 worktree/分支"));
    assert!(prompt.contains("服务器会在任务完成后串行合并回项目主分支"));
}

#[test]
fn development_prompt_includes_preflight_note() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "继续完成刚才的修改",
        Some("git pull 未成功（error: cannot pull with rebase: You have unstaged changes.）"),
        &test_option(),
        intent_router::CapabilityRoute::CodeAgent,
        false,
    );

    assert!(prompt.contains("项目预检结果"));
    assert!(prompt.contains("这不是最终失败"));
    assert!(prompt.contains("不要反复盲目执行同一个失败命令"));
}

#[test]
fn resumed_chat_prompt_is_short() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "继续聊这个思路",
        None,
        &test_option(),
        intent_router::CapabilityRoute::ChatAgent,
        true,
    );

    assert!(prompt.contains("Continue the existing Codex CLI native session"));
    assert!(prompt.contains("lightweight chat"));
    assert!(!prompt.contains("git pull --rebase"));
}

#[test]
fn resumed_development_prompt_reuses_bootstrap_rules() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "继续发布新版",
        Some("git status is clean"),
        &test_option(),
        intent_router::CapabilityRoute::CodeAgent,
        true,
    );

    assert!(prompt.contains("full development workflow was already injected"));
    assert!(prompt.contains("git status is clean"));
    assert!(prompt.contains("用户可见："));
    assert!(prompt.contains("new judgment"));
}

#[test]
fn stale_codex_session_output_triggers_fresh_retry() {
    let output = CliOutput {
        success: false,
        stdout: String::new(),
        stderr: "Error: could not resume session thread-1: not found".into(),
    };

    assert!(should_retry_without_native_session(
        &test_option(),
        Some("thread-1"),
        &output
    ));
    assert!(!should_retry_without_native_session(
        &test_option(),
        None,
        &output
    ));
}

#[test]
fn tiny_chat_messages_use_fast_path() {
    assert!(is_tiny_chat_message("你好"));
    assert!(is_tiny_chat_message("你好！"));
    assert!(is_tiny_chat_message("hello"));
    assert!(is_tiny_chat_message("在吗"));
    assert!(!is_tiny_chat_message("你好，帮我发布新版 APK"));
    assert!(!is_tiny_chat_message("继续修复刚才的构建问题"));
}

#[test]
fn timeout_caps_never_expand_cli_timeout() {
    let mut option = test_option();
    option.timeout_secs = 1800;
    cap_option_timeout(&mut option, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);
    assert_eq!(option.timeout_secs, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);

    let mut short_option = test_option();
    short_option.timeout_secs = 3;
    cap_option_timeout(&mut short_option, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS);
    assert_eq!(short_option.timeout_secs, 3);
}

#[test]
fn intent_gate_timeout_defaults_to_chat() {
    let result = intent_gate_timeout_chat_result("你好");

    assert_eq!(result.route, intent_router::CapabilityRoute::ChatAgent);
    assert!(!result.should_enter_development());
    assert!(result.chat_reply.unwrap().contains("普通聊天"));
}

#[test]
fn continuity_note_uses_codex_thread_uri_and_recent_messages() {
    let note = build_native_session_continuity_note(
        "019e55ee-81fb-7c03-98d9-957ba60739ca",
        &[
            ConversationMessage {
                role: "user".into(),
                content: "我们刚才在讨论普通聊天加速".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "已经建议 session 预热和短 prompt".into(),
            },
        ],
    );

    assert!(note.contains("codex://threads/019e55ee-81fb-7c03-98d9-957ba60739ca"));
    assert!(note.contains("普通聊天加速"));
    assert!(note.contains("短 prompt"));
}

#[test]
fn repair_prompt_creates_background_summary_without_project_workflow() {
    let prompt = build_native_session_repair_prompt(
        Path::new("D:/tmp/project"),
        &test_option(),
        "thread-1",
        &[ConversationMessage {
            role: "assistant".into(),
            content: "已经完成轻量聊天限时修复，剩余后台恢复摘要接力。".into(),
        }],
    );

    assert!(prompt.contains("background recovery job"));
    assert!(prompt.contains("codex://threads/thread-1"));
    assert!(prompt.contains("compact continuity summary"));
    assert!(prompt.contains("后台恢复摘要接力"));
    assert!(prompt.contains("Do not inspect files"));
    assert!(!prompt.contains("git pull --rebase"));
    assert!(!prompt.contains("通用项目工作流必须始终执行"));
}

#[test]
fn parses_intent_gate_chat_result() {
    let stdout = r#"{"type":"item.completed","item":{"type":"agent_message","text":"{\"route\":\"chat\",\"confidence\":0.93,\"reason\":\"只是询问流程\",\"chat_reply\":\"先聊清楚也可以。\"}"}}"#;
    let result = parse_intent_gate_result(stdout).unwrap();

    assert_eq!(result.route, intent_router::CapabilityRoute::ChatAgent);
    assert_eq!(result.chat_reply.as_deref(), Some("先聊清楚也可以。"));
    assert!(!result.should_enter_development());
}

#[test]
fn parses_intent_gate_development_result() {
    let stdout =
        r#"{"route":"development","confidence":0.91,"reason":"明确要求修改代码","chat_reply":""}"#;
    let result = parse_intent_gate_result(stdout).unwrap();

    assert_eq!(result.route, intent_router::CapabilityRoute::CodeAgent);
    assert!(result.should_enter_development());
}
