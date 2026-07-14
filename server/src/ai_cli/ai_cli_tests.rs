use super::ai_cli_chat::{intent_gate_timeout_chat_result, DEFAULT_TINY_CHAT_TIMEOUT_CAP_SECS};
use super::ai_cli_chat_policy::{
    project_lightweight_chat_split_enabled_from, prompt_route_for_project_chat,
    should_use_project_lightweight_chat, PROJECT_LIGHTWEIGHT_CHAT_ENABLED_ENV,
};
use super::ai_cli_native_session::build_native_session_continuity_note;
use super::ai_cli_output::parse_intent_gate_result;
use super::ai_cli_prompts::{build_native_session_repair_prompt, build_prewarm_cli_prompt};
use super::*;
use crate::store::ConversationMessage;
use crate::types::{AiCliOption, CliPromptMode};

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
            "--sandbox",
            "workspace-write",
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
        reasoning_effort: None,
        reasoning_summary: None,
        verbosity: None,
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
        AiCliRequestMode::Execute,
    );

    assert!(prompt.contains("轻量聊天模式"));
    assert!(!prompt.contains("轻量项目工作流必须执行"));
    assert!(!prompt.contains("git pull --rebase"));
}

#[test]
fn project_lightweight_chat_split_is_enabled_by_default() {
    let split_enabled = project_lightweight_chat_split_enabled_from(|_| None);

    assert!(split_enabled);
    assert!(should_use_project_lightweight_chat(
        split_enabled,
        false,
        intent_router::CapabilityRoute::ChatAgent,
        "你好"
    ));
    assert_eq!(
        prompt_route_for_project_chat(split_enabled, intent_router::CapabilityRoute::ChatAgent),
        intent_router::CapabilityRoute::ChatAgent
    );
}

#[test]
fn codex_reconnect_attempt_ignores_backoff_delay() {
    assert_eq!(
        extract_codex_reconnect_attempt(
            "stream disconnected - retrying sampling request (1/5 in 1199ms)"
        )
        .as_deref(),
        Some("1/5")
    );
    assert_eq!(
        extract_codex_reconnect_attempt("reconnecting... 3/5 in 7728ms").as_deref(),
        Some("3/5")
    );
}

#[test]
fn project_lightweight_chat_split_can_be_enabled_explicitly() {
    let split_enabled = project_lightweight_chat_split_enabled_from(|name| {
        (name == PROJECT_LIGHTWEIGHT_CHAT_ENABLED_ENV).then(|| "true".to_string())
    });

    assert!(split_enabled);
    assert!(should_use_project_lightweight_chat(
        split_enabled,
        false,
        intent_router::CapabilityRoute::ChatAgent,
        "你好"
    ));
    assert_eq!(
        prompt_route_for_project_chat(split_enabled, intent_router::CapabilityRoute::ChatAgent),
        intent_router::CapabilityRoute::ChatAgent
    );
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
        AiCliRequestMode::Execute,
    );

    assert!(prompt.contains("轻量项目工作流"));
    assert!(prompt.contains("先阅读这些轻量入口"));
    assert!(prompt.contains("按入口里的任务路由读取细则"));
    assert!(!prompt.contains("必须先阅读这些项目说明，再编辑文件"));
    assert!(!prompt.contains(".github/copilot-instructions.md、.github/instructions/*.md、README.md、docs/ai-agent-workflow.md"));

    assert!(prompt.contains("scripts/publish-apk.ps1"));
    assert!(prompt.contains("finish-ai-task.*` 的 `AndroidFeature` Kind"));
    assert!(prompt.contains("只同步时用 `CodePushed`"));
    assert!(prompt.contains("scripts/publish-server.ps1"));
    assert!(prompt.contains("pc-frontend"));
    assert!(prompt.contains("pc-next-dist"));
    assert!(prompt.contains("/api/server/version"));
    assert!(prompt.contains("DOM/坐标/层级检查"));
    assert!(prompt.contains("统一收尾的 `PcFrontend` Kind"));
    assert!(prompt.contains("不要把 CodePushed 当成用户问题已解决"));
    assert!(prompt.contains("不能只凭 `npm run build` 宣称解决"));
    assert!(prompt.contains("finish-ai-task.*"));
    assert!(prompt.contains("FINALIZABLE=true"));
    assert!(prompt.contains("脚本输出 `NEXT=`、`EDIT_ROOT=`、`FINISH_COMMAND_*=`、`FINALIZABLE=`"));
    assert!(prompt.contains("不要手动改 `server/Cargo.toml` 版本"));
    assert!(prompt.contains("不要提交 `build.gradle` 版本"));
    assert!(!prompt.contains("/api/release/claim"));
    assert!(!prompt.contains("ELON_BUILD_VERSION"));
    assert!(!prompt.contains("临时写入 build.gradle"));
    assert!(prompt.contains("新建源文件默认目标 <=500 行"));
    assert!(prompt.contains("501-800 行可容忍"));
    assert!(prompt.contains(">800 行必须拆分"));
    assert!(prompt.contains("已有 >1500 行文件"));
    assert!(prompt.contains("5-15 行文件计划"));
    assert!(!prompt.contains("必须递增 server/Cargo.toml 的 package.version"));
    assert!(!prompt.contains("递增 versionCode/versionName"));
    assert!(prompt.contains("只有 push 被拒绝才 rebase"));
    assert!(prompt.contains("无 origin 项目本地 commit 即可"));
}

#[test]
fn plan_prompt_is_read_only_even_for_code_route() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "给这个功能做一个计划",
        Some("source-size guardrail"),
        &test_option(),
        intent_router::CapabilityRoute::CodeAgent,
        false,
        AiCliRequestMode::Plan,
    );

    assert!(prompt.contains("当前是 Plan 模式"));
    assert!(prompt.contains("绝对不要创建、修改、删除文件"));
    assert!(prompt.contains("按这个计划开始实现"));
    assert!(prompt.contains("source-size guardrail"));
}

#[test]
fn resumed_development_prompt_keeps_source_size_guardrail() {
    let prompt = build_cli_prompt(
        Path::new("D:/tmp/project"),
        "继续刚才的代码修改",
        None,
        &test_option(),
        intent_router::CapabilityRoute::CodeAgent,
        true,
        AiCliRequestMode::Execute,
    );

    assert!(prompt.contains("source-size guardrail"));
    assert!(prompt.contains("new source files target <=500 lines"));
    assert!(prompt.contains("501-800 lines are tolerated"));
    assert!(prompt.contains(">800 lines must be split"));
    assert!(prompt.contains("existing >1500-line files"));
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
        AiCliRequestMode::Execute,
    );

    assert!(prompt.contains("项目预检与约束摘要"));
    assert!(prompt.contains("这不是最终失败"));
    assert!(prompt.contains("保护已有改动"));
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
        AiCliRequestMode::Execute,
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
        AiCliRequestMode::Execute,
    );

    assert!(prompt.contains("full development workflow was already injected"));
    assert!(prompt.contains("use the publish scripts after commit + push"));
    assert!(prompt.contains("do not manually bump or commit"));
    assert!(prompt.contains("If scripts print NEXT=, ERROR_CODE=, DOC="));
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
fn pc_cli_passthrough_keeps_tool_events_structured() {
    let raw = r#"{"type":"tool_call","tool":"run_command","args":{"program":"git"}}"#;
    let event = pc_cli_passthrough_event(raw).unwrap();
    let value: serde_json::Value = serde_json::from_str(&event).unwrap();
    assert_eq!(value["type"], "tool_call");
    assert_eq!(value["tool"], "run_command");
    assert_eq!(value["args"]["program"], "git");

    assert!(pc_cli_passthrough_event("plain text").is_none());
    assert!(pc_cli_passthrough_event(r#"{"type":"assistant_message","text":"hi"}"#).is_none());
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
    assert!(!prompt.contains("轻量项目工作流必须执行"));
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

#[test]
fn app_keyword_counts_as_android_task_for_pc_artifact_sync() {
    assert!(super::ai_cli_environment::looks_like_android_task(
        "帮我开发一个 APP"
    ));
}

#[test]
fn project_dev_profile_counts_as_android_task_for_pc_artifact_sync() {
    assert!(super::ai_cli_environment::looks_like_android_task(
        "把按钮改成绿色\n\n<project_dev_profile>\nproject_type: android\nbuild_command: ./gradlew assembleDebug\n</project_dev_profile>"
    ));
}

#[test]
fn pc_apk_probe_since_tracks_non_plan_workspace_tasks() {
    assert!(pc_apk_probe_since(AiCliRequestMode::Execute, Some("C:/repo")).is_some());
    assert!(pc_apk_probe_since(AiCliRequestMode::Execute, None).is_none());
    assert!(pc_apk_probe_since(AiCliRequestMode::Plan, Some("C:/repo")).is_none());
    assert!(pc_apk_probe_since(AiCliRequestMode::Passthrough, Some("C:/repo")).is_some());
}

#[test]
fn pc_apk_sync_prefers_active_conversation_workspace() {
    assert_eq!(
        super::ai_cli_apk_sync::pc_apk_sync_workspace(
            Some("C:/project/repo"),
            Some("C:/project/conversation-worktree")
        ),
        Some("C:/project/conversation-worktree")
    );
    assert_eq!(
        super::ai_cli_apk_sync::pc_apk_sync_workspace(Some("C:/project/repo"), Some("  ")),
        Some("C:/project/repo")
    );
}

#[test]
fn pc_apk_filename_is_sanitized() {
    assert_eq!(
        safe_pc_apk_filename(r"C:\tmp\outputs\app-debug.apk"),
        "app-debug.apk"
    );
    assert_eq!(
        safe_pc_apk_filename("not-an-apk.txt"),
        "ElonSpeed-latest.apk"
    );
}
