use super::ai_cli_pc_config::pc_route_a_extra_args;
use super::codex_reply_is_complete;
use super::{
    clean_codex_stream_chunk, extract_codex_reply, extract_lightweight_pc_chat_reply,
    extract_lightweight_pc_chat_timeout_reply, lightweight_pc_reply_delta, native_session_uuid,
    pc_codex_progress_hint, pc_dispatch_started_event, pc_display_model_label,
    pc_lightweight_chat_reasoning_effort, pc_lightweight_no_node_event_diagnostic,
    pc_lightweight_no_readable_diagnostic, pc_passthrough_empty_reply_diagnostic,
    pc_project_reasoning_effort, sanitize_pc_development_reply, should_skip_pc_chat_native_session,
    strip_terminal_control_sequences, AiCliRequestMode, NativeSessionScope,
};
use serde_json::Value;

fn test_scope(conversation_id: &str) -> NativeSessionScope {
    NativeSessionScope {
        project_id: "project-1".to_string(),
        user_id: "user-1".to_string(),
        conversation_id: conversation_id.to_string(),
        runtime_permission: "project_write".to_string(),
    }
}

#[test]
fn pc_codex_stream_chunk_filters_terminal_noise() {
    let raw = "\u{1b}[m\u{1b}]0;C:\\Windows\\system32\\cmd.EXE\u{7}\u{1b}[?25h\
\u{1b}]0;C:\\WINDOWS\\system32\\cmd.exe \u{7}\
\u{1b}[2m2026-07-01T07:02:50.938044Z\u{1b}[22m  \u{1b}[33mWARN \u{1b}[m\
\u{1b}[2mcodex_core_plugins::manifest:\u{1b}[22m ignoring interface.defaultPrompt[0]\n\
memories startup: error returned from database: (code: 1) no such table: stage1_outputs\n\
mcp_native_chat_ok\n";

    let clean = clean_codex_stream_chunk(raw);

    assert_eq!(clean, "mcp_native_chat_ok\n");
}

#[test]
fn pc_dispatch_started_event_exposes_local_req_id_without_prompt() {
    let event = pc_dispatch_started_event(
        "req-1",
        "agent-1",
        "一龙4060（agent-1）",
        "codex",
        Some("D:/workspace"),
        None,
        AiCliRequestMode::Execute,
    );
    let value: Value = serde_json::from_str(&event).unwrap();
    assert_eq!(value["type"], "pc_dispatch_started");
    assert_eq!(value["pc_req_id"], "req-1");
    assert_eq!(value["req_id"], "req-1");
    assert_eq!(value["node_display_name"], "一龙4060（agent-1）");
    assert!(value.get("prompt").is_none());
    assert!(value.get("api_key").is_none());
}

#[test]
fn native_session_uuid_is_stable_and_cli_scoped() {
    let scope = test_scope("conversation-1");
    let codex = native_session_uuid("codex", &scope);
    let same_codex = native_session_uuid("codex", &scope);
    let copilot = native_session_uuid("copilot", &scope);
    let other_conversation = native_session_uuid("codex", &test_scope("conversation-2"));

    assert_eq!(codex, same_codex);
    assert_ne!(codex, copilot);
    assert_ne!(codex, other_conversation);
    assert!(codex.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-'));
}

#[test]
fn pc_codex_extra_args_include_stable_session_and_runtime_options() {
    let session_id = native_session_uuid("codex", &test_scope("conversation-1"));
    let args = pc_route_a_extra_args(
        "codex",
        Some(&session_id),
        Some("gpt-5.3-codex"),
        Some("medium"),
    );

    assert_eq!(args[0], format!("--session-id={session_id}"));
    assert!(args.contains(&"--codex-model=gpt-5.3-codex".to_string()));
    assert!(args.contains(&"--codex-effort=medium".to_string()));
}

#[test]
fn pc_lightweight_chat_downgrades_heavy_codex_effort() {
    assert_eq!(
        pc_lightweight_chat_reasoning_effort("codex", Some("xhigh")).as_deref(),
        Some("low")
    );
    assert_eq!(
        pc_lightweight_chat_reasoning_effort("codex", Some("high")).as_deref(),
        Some("low")
    );
    assert_eq!(
        pc_lightweight_chat_reasoning_effort("codex", Some("medium")).as_deref(),
        Some("medium")
    );
    assert_eq!(
        pc_lightweight_chat_reasoning_effort("copilot", Some("xhigh")),
        None
    );
}

#[test]
fn pc_project_codex_defaults_to_medium_effort() {
    assert_eq!(
        pc_project_reasoning_effort("codex", None, AiCliRequestMode::Execute).as_deref(),
        Some("medium")
    );
    assert_eq!(
        pc_project_reasoning_effort("codex", None, AiCliRequestMode::Plan).as_deref(),
        Some("low")
    );
    assert_eq!(
        pc_project_reasoning_effort("codex", Some("high"), AiCliRequestMode::Execute).as_deref(),
        Some("high")
    );
    assert_eq!(
        pc_project_reasoning_effort("codex", Some("ultra"), AiCliRequestMode::Execute).as_deref(),
        Some("xhigh")
    );
    assert_eq!(
        pc_project_reasoning_effort("codex", Some("unexpected"), AiCliRequestMode::Execute)
            .as_deref(),
        Some("medium")
    );
    assert_eq!(
        pc_project_reasoning_effort("copilot", None, AiCliRequestMode::Execute),
        None
    );
}

#[test]
fn pc_direct_passthrough_does_not_force_default_effort() {
    assert_eq!(
        pc_project_reasoning_effort("codex", None, AiCliRequestMode::Passthrough),
        None
    );
    assert_eq!(
        pc_project_reasoning_effort("codex", Some("high"), AiCliRequestMode::Passthrough)
            .as_deref(),
        Some("high")
    );
}

#[test]
fn pc_codex_progress_hint_reports_network_fallbacks() {
    let reconnect = "\u{1b}[31mERROR:\u{1b}[m Reconnecting... 3/5";
    let (_, message) =
        pc_codex_progress_hint(reconnect, "Codex · 推理 medium").expect("reconnect hint");
    assert!(message.contains("自动重连"));
    assert!(message.contains("第 3/5 次"));

    let (_, message) = pc_codex_progress_hint("codex_core::client: falling back to HTTP", "Codex")
        .expect("fallback hint");
    assert!(message.contains("HTTP fallback"));
}

#[test]
fn pc_lightweight_chat_skips_native_session_for_short_starters() {
    assert!(should_skip_pc_chat_native_session("你好"));
    assert!(should_skip_pc_chat_native_session("我有一个想法"));
    assert!(should_skip_pc_chat_native_session("有个需求"));
    assert!(!should_skip_pc_chat_native_session(
        "我有一个想法，想做一个可以扫描商品并自动比价的 App"
    ));
}

#[test]
fn pc_lightweight_display_label_reports_effective_low_effort() {
    assert_eq!(
        pc_display_model_label(
            "codex",
            Some("GPT-5.5 · 推理 xhigh"),
            Some("low"),
            true,
            "node-a",
        ),
        "GPT-5.5 · 轻量 low"
    );
    assert_eq!(
        pc_display_model_label("codex", Some("GPT-5.5"), Some("low"), false, "node-a"),
        "GPT-5.5"
    );
}

#[test]
fn pc_copilot_extra_args_keep_session_and_model_flags() {
    let session_id = native_session_uuid("copilot", &test_scope("conversation-1"));
    let args = pc_route_a_extra_args("copilot", Some(&session_id), Some("gpt-5"), None);

    assert_eq!(
        args,
        vec![
            format!("--session-id={session_id}"),
            "--model".to_string(),
            "gpt-5".to_string()
        ]
    );
}

#[test]
fn pc_lightweight_chat_reply_ignores_terminal_noise() {
    let output = "\u{1b}[m\\\\?\\C:\\Users\\ELon\n\
用作为当前目录的以上路径启动了 CMD.EXE。\n\
UNC 路径不受支持。默认值设为 Windows 目录。\n\
]0;C:\\WINDOWS\\system32\\cmd.exe\u{1b}[?25h]0;C:\\WINDOWS\\system32\\cmd.exe\n\
2026-06-30T12:14:19.451149Z WARN sqlx::query: slow statement: execution time exceeded alert threshold db.statement=\"DELETE FROM logs WHERE ts < ?\" rows_affected=10449 rows_returned=0 elapsed=1.54s\n\
codex\n\
你好，我在。\n\
tokens used\n\
1\n";

    assert_eq!(
        extract_lightweight_pc_chat_reply(output, true),
        "你好，我在。"
    );
}

#[test]
fn pc_lightweight_chat_strips_orphan_ansi_fragments() {
    assert_eq!(strip_terminal_control_sequences("[m你好[?25h[22m"), "你好");
}

#[test]
fn pc_codex_reply_extracts_last_summary_block() {
    let output = "\u{1b}[35mcodex\u{1b}[m\n\
规则文件已成功读取。\n\
exec\n\
git status\n\
\u{1b}[35mcodex\u{1b}[m\n\
完成。本次只新增了记录文件。\n\
\n\
结果汇总：\n\
- 读取代码成功。\n\
- Git 可用。\n\
tokens used\n\
44,443\n";

    let reply = extract_codex_reply(output);

    assert!(reply.contains("完成。本次只新增了记录文件。"));
    assert!(reply.contains("结果汇总"));
    assert!(!reply.contains("规则文件已成功读取"));
}

#[test]
fn pc_codex_reply_ignores_false_unparseable_diagnostic() {
    let output = "codex\n\
完成。真实最终回复。\n\
tokens used\n\
1\n\
codex\n\
Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。\n";

    assert_eq!(extract_codex_reply(output), "完成。真实最终回复。");
}

#[test]
fn pc_codex_reply_reads_json_agent_message() {
    let output = concat!(
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：第一段过程"}}"#,
        "\n",
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"最终回复。已完成授权选择器。"}}"#
    );

    assert_eq!(extract_codex_reply(output), "最终回复。已完成授权选择器。");
    assert!(codex_reply_is_complete(output));
}

#[test]
fn pc_codex_reply_requires_a_summary_after_the_last_tool() {
    let incomplete = concat!(
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"我先读取规则。"}}"#,
        "\n",
        r#"{"type":"item.started","item":{"type":"command_execution","command":"Get-Content CODEX.md"}}"#,
        "\n",
        r#"{"type":"item.completed","item":{"type":"command_execution","exit_code":0}}"#,
    );
    let complete = format!(
        "{}\n{}",
        incomplete,
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"只读检查完成，规则已确认。"}}"#,
    );

    assert!(!codex_reply_is_complete(incomplete));
    assert!(codex_reply_is_complete(&complete));
}

#[test]
fn pc_passthrough_empty_reply_diagnostic_does_not_claim_success() {
    let diagnostic = pc_passthrough_empty_reply_diagnostic("", "codex", "GPT-5.5 · 推理 xhigh");

    assert!(diagnostic.contains("没有返回可展示的正文"));
    assert!(diagnostic.contains("无法确认完成"));
    assert!(!diagnostic.contains("已完成"));
    assert!(!diagnostic.contains("任务已完成"));
}

#[test]
fn pc_development_reply_hides_paths_commands_and_diff() {
    let reply = "已改好，“开始”按钮现在是绿色，文字是白色，并且 APK 已重新构建成功。\n\
新的安装包仍在这里:\n\
[app-debug.apk](C:/Users/Administrator/Elon/workspaces/conversation-worktrees/prj/app/build/outputs/apk/debug/app-debug.apk)\n\
安装命令不变:\n\
```powershell\n\
C:\\Users\\Administrator\\AppData\\Local\\Android\\Sdk\\platform-tools\\adb.exe install -r app\\build\\outputs\\apk\\debug\\app-debug.apk\n\
```\n\
diff --git a/app/src/main/java/com/dadapao/app/MainActivity.java b/app/src/main/java/com/dadapao/app/MainActivity.java\n";

    let sanitized = sanitize_pc_development_reply(reply, Some("https://example.test/latest.apk"));

    assert!(sanitized.contains("已改好"));
    assert!(sanitized.contains("项目空间"));
    assert!(!sanitized.contains("C:/Users"));
    assert!(!sanitized.contains("adb.exe"));
    assert!(!sanitized.contains("diff --git"));
}

#[test]
fn pc_development_reply_keeps_device_and_runtime_summary() {
    let reply = "已确认当前项目可用的 Android 渲染设备。\n\
物理设备：小米 23116PN5BC（shennong）。\n\
USB：e0d909c3，状态 device。\n\
无线：192.168.31.171:5555，状态 device。\n\
两个连接对应同一台测试手机。\n\
推荐优先使用 USB；需要脱线测试时切换无线连接。\n\
Renderer：BOOTSTRAP。\n\
Live Runtime：尚未连接，nodeCount=0。";

    let sanitized = sanitize_pc_development_reply(reply, None);

    assert!(sanitized.contains("192.168.31.171:5555"));
    assert!(sanitized.contains("Renderer：BOOTSTRAP"));
    assert!(sanitized.contains("nodeCount=0"));
    assert!(sanitized.chars().count() <= 1600);
}

#[test]
fn pc_development_empty_reply_does_not_claim_code_was_changed() {
    let sanitized = sanitize_pc_development_reply("", None);

    assert!(sanitized.contains("没有返回可展示的总结"));
    assert!(!sanitized.contains("已改好"));
    assert!(!sanitized.contains("本轮开发任务已完成"));
}

#[test]
fn pc_lightweight_chat_empty_output_stays_empty_for_upstream_fallback() {
    let reply = extract_lightweight_pc_chat_reply("", true);

    assert!(reply.trim().is_empty());
}

#[test]
fn pc_lightweight_chat_timeout_keeps_partial_readable_reply() {
    let output = "OpenAI Codex\nmodel: test\ncodex\nhello, tell me your idea.";

    assert_eq!(
        extract_lightweight_pc_chat_timeout_reply(output, true).as_deref(),
        Some("hello, tell me your idea.")
    );
}

#[test]
fn pc_lightweight_no_readable_diagnostic_exposes_codex_network_timeout() {
    let output = "2026-07-02 WARN stream disconnected - retrying sampling request (5/5)\n\
{\"type\":\"error\",\"message\":\"Reconnecting... 5/5 (request timed out)\"}\n\
2026-07-02 WARN falling back to HTTP";

    let diagnostic = pc_lightweight_no_readable_diagnostic(output, "codex").unwrap();

    assert!(diagnostic.contains("Codex"));
    assert!(diagnostic.contains("网络请求超时"));
    assert!(diagnostic.contains("request timed out"));
    assert!(diagnostic.contains("fallback HTTP"));
}

#[test]
fn pc_lightweight_first_event_timeout_names_node_ack_gap() {
    let diagnostic = pc_lightweight_no_node_event_diagnostic("codex", "一龙4060（node-a）", 15);

    assert!(diagnostic.contains("Codex"));
    assert!(diagnostic.contains("一龙4060（node-a）"));
    assert!(diagnostic.contains("15 秒内没有返回任何 CLI 输出或完成事件"));
    assert!(diagnostic.contains("本轮已停止"));
}

#[test]
fn pc_lightweight_chat_reply_delta_streams_growth_only() {
    let mut streamed = String::new();
    assert_eq!(
        lightweight_pc_reply_delta("OpenAI Codex\nmodel: test\ncodex\n说", true, &mut streamed)
            .as_deref(),
        Some("说")
    );
    assert_eq!(
        lightweight_pc_reply_delta(
            "OpenAI Codex\nmodel: test\ncodex\n说说看。",
            true,
            &mut streamed,
        )
        .as_deref(),
        Some("说看。")
    );
    assert_eq!(
        lightweight_pc_reply_delta(
            "OpenAI Codex\nmodel: test\ncodex\n说说看。",
            true,
            &mut streamed,
        ),
        None
    );
}
