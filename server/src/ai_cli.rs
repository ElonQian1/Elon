use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{
    path::Path,
    path::PathBuf,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::UnboundedSender,
};

use crate::{
    intent_router, tools,
    types::{AiCliOption, AppState, CliPromptMode, WsMessage},
};

#[derive(Debug, Clone)]
pub struct NativeSessionScope {
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntentGateResult {
    pub route: intent_router::CapabilityRoute,
    pub confidence: f64,
    pub reason: String,
    pub chat_reply: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrewarmResult {
    pub reused: bool,
    pub thread_id: Option<String>,
    pub elapsed_ms: u128,
}

impl IntentGateResult {
    pub fn should_enter_development(&self) -> bool {
        self.route == intent_router::CapabilityRoute::CodeAgent && self.confidence >= 0.75
    }
}

pub async fn confirm_project_intent(
    workspace: &Path,
    user_message: &str,
    option_id: Option<&str>,
    native_session_scope: Option<NativeSessionScope>,
    state: &Arc<AppState>,
) -> Result<IntentGateResult> {
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;
    if !supports_codex_sessions(&option) {
        return Err(anyhow!("当前阶段意图确认必须使用 Codex CLI"));
    }

    std::fs::create_dir_all(workspace)?;
    let workspace_key = workspace.display().to_string();
    let native_session_id = native_session_scope.as_ref().and_then(|scope| {
        state
            .store
            .get_native_agent_session(
                &scope.project_id,
                &scope.user_id,
                Some(&scope.conversation_id),
                &option.provider,
                &option.id,
                &workspace_key,
            )
            .ok()
            .flatten()
    });

    let prompt = build_intent_gate_prompt(workspace, user_message, &option);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let output = run_cli_command(
        &option,
        workspace,
        &prompt,
        native_session_id.as_deref(),
        &tx,
    )
    .await?;

    if let (Some(scope), Some(thread_id)) = (
        native_session_scope.as_ref(),
        extract_thread_id(&output.stdout),
    ) {
        let _ = state.store.upsert_native_agent_session(
            &scope.project_id,
            &scope.user_id,
            Some(&scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            &thread_id,
        );
    }

    parse_intent_gate_result(&output.stdout)
}

pub async fn prewarm_codex_session(
    workspace: &Path,
    option_id: Option<&str>,
    native_session_scope: NativeSessionScope,
    state: &Arc<AppState>,
) -> Result<PrewarmResult> {
    let started = Instant::now();
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("no local AI CLI option is available"))?;
    if !supports_codex_sessions(&option) {
        return Err(anyhow!(
            "Codex CLI session prewarm requires a Codex CLI option"
        ));
    }

    std::fs::create_dir_all(workspace)?;
    let workspace_key = workspace.display().to_string();
    let existing_session_id = state.store.get_native_agent_session(
        &native_session_scope.project_id,
        &native_session_scope.user_id,
        Some(&native_session_scope.conversation_id),
        &option.provider,
        &option.id,
        &workspace_key,
    )?;
    if let Some(thread_id) = existing_session_id {
        return Ok(PrewarmResult {
            reused: true,
            thread_id: Some(thread_id),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let mut prewarm_option = option.clone();
    prewarm_option.timeout_secs = prewarm_option.timeout_secs.min(90);
    let prompt = build_prewarm_cli_prompt(workspace, &prewarm_option);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let output = run_cli_command(&prewarm_option, workspace, &prompt, None, &tx).await?;
    let thread_id = extract_thread_id(&output.stdout);
    if let Some(thread_id) = thread_id.as_deref() {
        let _ = state.store.upsert_native_agent_session(
            &native_session_scope.project_id,
            &native_session_scope.user_id,
            Some(&native_session_scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            thread_id,
        );
    }

    Ok(PrewarmResult {
        reused: false,
        thread_id,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

pub async fn run_with_workspace(
    user_id: &str,
    workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    option_id: Option<&str>,
    route: intent_router::CapabilityRoute,
    require_existing_git: bool,
    native_session_scope: Option<NativeSessionScope>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let started = std::time::Instant::now();
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;

    std::fs::create_dir_all(workspace)?;

    let development_task = route != intent_router::CapabilityRoute::ChatAgent
        || intent_router::looks_like_development_request(user_message);
    if development_task {
        ensure_git(workspace, user_id, require_existing_git)?;
    }

    let android_task = development_task && looks_like_android_task(user_message);
    if development_task {
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在准备项目工作区。".into(),
            }
            .to_json(),
        );
        for note in environment_notes(user_message, &option) {
            let _ = tx.send(WsMessage::Progress { message: note }.to_json());
        }
        let _ = tx.send(
            WsMessage::Progress {
                message: "AI 助手正在处理你的请求。".into(),
            }
            .to_json(),
        );
    } else {
        let _ = tx.send(
            WsMessage::Progress {
                message: "正在思考。".into(),
            }
            .to_json(),
        );
    }

    let workspace_key = workspace.display().to_string();
    let native_session_id = if supports_codex_sessions(&option) {
        native_session_scope.as_ref().and_then(|scope| {
            state
                .store
                .get_native_agent_session(
                    &scope.project_id,
                    &scope.user_id,
                    Some(&scope.conversation_id),
                    &option.provider,
                    &option.id,
                    &workspace_key,
                )
                .ok()
                .flatten()
        })
    } else {
        None
    };
    if native_session_id.is_some() {
        let _ = tx.send(
            WsMessage::Progress {
                message: "Restoring Codex CLI context for this conversation.".into(),
            }
            .to_json(),
        );
    }

    let prompt = build_cli_prompt(workspace, user_message, preflight_note, &option, route);
    let output = run_cli_command(
        &option,
        workspace,
        &prompt,
        native_session_id.as_deref(),
        tx,
    )
    .await?;
    if let (Some(scope), Some(thread_id)) = (
        native_session_scope
            .as_ref()
            .filter(|_| supports_codex_sessions(&option)),
        extract_thread_id(&output.stdout),
    ) {
        let _ = state.store.upsert_native_agent_session(
            &scope.project_id,
            &scope.user_id,
            Some(&scope.conversation_id),
            &option.provider,
            &option.id,
            &workspace_key,
            &thread_id,
        );
    }
    let reply = format_cli_reply(&output.stdout, &output.stderr, output.success);
    tracing::info!(
        route = ?route,
        development_task,
        elapsed_ms = started.elapsed().as_millis(),
        "local AI CLI request completed"
    );

    let apk_url = if android_task && output.success {
        let _ = tx.send(
            WsMessage::Progress {
                message: "AI 已完成处理，正在查找 APK 安装包。".into(),
            }
            .to_json(),
        );
        let apk_url =
            tools::find_latest_apk(workspace).map(|_| tools::stable_apk_url(download_base));
        if apk_url.is_none() {
            let _ = tx.send(
                WsMessage::Progress {
                    message: "未找到 APK 安装包；如果刚才是在打包，请检查最终回复里的失败原因。"
                        .into(),
                }
                .to_json(),
            );
        }
        apk_url
    } else {
        None
    };

    let _ = tx.send(
        WsMessage::Done {
            message: reply,
            apk_url,
            image_url: None,
        }
        .to_json(),
    );

    Ok(())
}

struct CliOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn supports_codex_sessions(option: &AiCliOption) -> bool {
    option.provider.eq_ignore_ascii_case("codex")
        || option.id.to_ascii_lowercase().contains("codex")
        || option
            .bin
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .map(|bin| bin.eq_ignore_ascii_case("codex"))
            .unwrap_or(false)
}

fn cli_args_for_run(option: &AiCliOption, native_session_id: Option<&str>) -> Vec<String> {
    if !supports_codex_sessions(option) {
        return option.args.clone();
    }
    if let Some(session_id) = native_session_id {
        if let Some(args) = codex_resume_args(&option.args, session_id) {
            return args;
        }
    }
    codex_exec_json_args(&option.args)
}

fn codex_exec_json_args(raw_args: &[String]) -> Vec<String> {
    let mut args = raw_args.to_vec();
    if args.iter().any(|arg| arg == "--json") {
        return args;
    }
    if let Some(exec_index) = args.iter().position(|arg| arg == "exec" || arg == "e") {
        args.insert(exec_index + 1, "--json".into());
    }
    args
}

fn codex_resume_args(raw_args: &[String], session_id: &str) -> Option<Vec<String>> {
    let exec_index = raw_args
        .iter()
        .position(|arg| arg == "exec" || arg == "e")?;
    let mut args = raw_args[..exec_index].to_vec();
    args.push("exec".into());
    args.push("resume".into());

    let mut has_json = false;
    let mut iter = raw_args[exec_index + 1..].iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => {
                has_json = true;
                args.push(arg.clone());
            }
            "--skip-git-repo-check"
            | "--ignore-user-config"
            | "--ignore-rules"
            | "--strict-config"
            | "--dangerously-bypass-approvals-and-sandbox"
            | "--dangerously-bypass-hook-trust" => args.push(arg.clone()),
            "-m" | "--model" | "-c" | "--config" | "-p" | "--profile" | "--profile-v2"
            | "--output-schema" => {
                args.push(arg.clone());
                if let Some(value) = iter.next() {
                    args.push(value.clone());
                }
            }
            _ => {}
        }
    }
    if !has_json {
        args.push("--json".into());
    }
    args.push(session_id.to_string());
    Some(args)
}

async fn run_cli_command(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
    tx: &UnboundedSender<String>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(&option.bin);
    let args = cli_args_for_run(option, native_session_id);
    cmd.args(&args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    match option.prompt_mode {
        CliPromptMode::Arg => {
            cmd.stdin(Stdio::null());
            cmd.arg(prompt);
        }
        CliPromptMode::Stdin => {
            cmd.stdin(Stdio::piped());
        }
    }

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!(
            "启动本地 AI CLI 失败: {}。请检查选项 '{}' 的 bin/args 配置",
            e,
            option.id
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("无法读取本地 AI CLI stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("无法读取本地 AI CLI stderr"))?;

    let stdout_task = tokio::spawn(read_cli_stream(stdout));
    let stderr_task = tokio::spawn(read_cli_stream(stderr));
    let heartbeat_task = tokio::spawn(send_cli_heartbeat(tx.clone()));

    if option.prompt_mode == CliPromptMode::Stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
        }
    }

    let status =
        match tokio::time::timeout(Duration::from_secs(option.timeout_secs), child.wait()).await {
            Ok(result) => result?,
            Err(_) => {
                heartbeat_task.abort();
                let _ = child.kill().await;
                return Err(anyhow!(
                    "本地 AI CLI 执行超时，请稍后重试或调大对应 TIMEOUT_SECS"
                ));
            }
        };
    heartbeat_task.abort();

    let stdout = stdout_task.await.unwrap_or_default();
    let stderr = stderr_task.await.unwrap_or_default();

    Ok(CliOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

async fn send_cli_heartbeat(tx: UnboundedSender<String>) {
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        if tx
            .send(
                WsMessage::Progress {
                    message: "AI 还在处理，请稍候。".into(),
                }
                .to_json(),
            )
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
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
        );

        assert!(prompt.contains("通用项目工作流必须始终执行"));
        assert!(prompt.contains("git pull --rebase"));
    }

    #[test]
    fn development_prompt_includes_preflight_note() {
        let prompt = build_cli_prompt(
            Path::new("D:/tmp/project"),
            "继续完成刚才的修改",
            Some("git pull 未成功（error: cannot pull with rebase: You have unstaged changes.）"),
            &test_option(),
            intent_router::CapabilityRoute::CodeAgent,
        );

        assert!(prompt.contains("项目预检结果"));
        assert!(prompt.contains("这不是最终失败"));
        assert!(prompt.contains("不要反复盲目执行同一个失败命令"));
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
        let stdout = r#"{"route":"development","confidence":0.91,"reason":"明确要求修改代码","chat_reply":""}"#;
        let result = parse_intent_gate_result(stdout).unwrap();

        assert_eq!(result.route, intent_router::CapabilityRoute::CodeAgent);
        assert!(result.should_enter_development());
    }
}

async fn read_cli_stream<R>(reader: R) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut collected = String::new();

    while let Ok(Some(line)) = lines.next_line().await {
        collected.push_str(&line);
        collected.push('\n');
    }

    collected
}

fn build_cli_prompt(
    workspace: &Path,
    user_message: &str,
    preflight_note: Option<&str>,
    option: &AiCliOption,
    route: intent_router::CapabilityRoute,
) -> String {
    if route == intent_router::CapabilityRoute::ChatAgent {
        return build_chat_cli_prompt(workspace, user_message, option);
    }

    build_development_cli_prompt(workspace, user_message, preflight_note, option)
}

fn build_prewarm_cli_prompt(workspace: &Path, option: &AiCliOption) -> String {
    let model_text = option
        .model
        .as_deref()
        .map(|model| format!(", current model: {}", model))
        .unwrap_or_default();
    format!(
        r#"You are prewarming a Codex CLI native session for one APK project conversation.

Current CLI provider: {provider}{model_text}
Current project workspace: {workspace}

Rules:
- Do not inspect files, run commands, use Git, modify code, build, deploy, publish, or enter the project development workflow.
- Do not analyze the user's project. This call is only meant to create the native Codex CLI session id for future turns in the same conversation.
- Keep any future project rules, memory, or workflow discovery for the first real user request.
- Reply with exactly one line of JSON and nothing else: {{"status":"ready","mode":"prewarm"}}"#,
        provider = option.provider,
        model_text = model_text,
        workspace = workspace.display()
    )
}

fn build_chat_cli_prompt(workspace: &Path, user_message: &str, option: &AiCliOption) -> String {
    let model_text = option
        .model
        .as_deref()
        .map(|model| format!("，当前模型：{}", model))
        .unwrap_or_default();
    format!(
        r#"你是「一龙」平台里的 Codex CLI 对话助手。当前是轻量聊天模式，不是开发执行模式。

当前 CLI：{provider}{model_text}
当前项目目录：{workspace}

请直接回复用户。规则：
- 保持同一个 Codex CLI 原生 session 的上下文连续，但本轮不要主动读取文件、检查 Git、运行命令、修改代码或打包 APK。
- 可以正常聊天、解释概念、帮用户梳理想法、询问下一步，也可以把模糊需求整理成可开发任务。
- 如果用户明确要求修改代码、读取项目、编译、部署或发布 APK，只做简短确认和需求澄清，不要声称已经执行；下一轮会进入开发流程。
- 以后服务端可能把其他 AI 模型的旁路分析结论给你；那些只是参考证据，主上下文仍以当前 Codex CLI session 为准。
- 回复中文，简洁自然，不要输出工具日志，不要使用「用户可见：」前缀。

用户请求：
{user_message}"#,
        provider = option.provider,
        model_text = model_text,
        workspace = workspace.display(),
        user_message = user_message
    )
}

fn build_intent_gate_prompt(workspace: &Path, user_message: &str, option: &AiCliOption) -> String {
    let model_text = option
        .model
        .as_deref()
        .map(|model| format!("，当前模型：{}", model))
        .unwrap_or_default();
    format!(
        r#"你是「一龙」平台里的 Codex CLI 轻量意图确认器。本轮只判断是否需要进入项目开发执行流程。

当前 CLI：{provider}{model_text}
当前项目目录：{workspace}

严格规则：
- 不要读取文件，不要检查 Git，不要运行命令，不要修改代码，不要打包 APK。
- 如果用户明确要求修改/新增/修复代码、编译、打包、发布、部署、提交、推送、操作 Git、替换项目资源，才判定为 development。
- 如果用户是在闲聊、提问、解释概念、讨论流程、问“会不会/是不是/为什么/怎么做最好”、表达想法但没有明确要求立刻执行，判定为 chat。
- 模糊时必须判定为 chat，避免普通聊天误触发重型开发流程。
- 如果 route 是 chat，请用 chat_reply 直接正常回复用户；项目相关普通提问也要尽量基于当前对话和通用经验回答，不要因为提到 APK、项目、服务器或 Git 就要求用户重说。
- 例子：用户问“我们的 APK 是否支持多个手机同时登录/并行修改/会不会冲突？”这属于能力和流程讨论，route 必须是 chat，chat_reply 应该直接解释原则、风险和建议。
- 对“是否支持/会不会/是不是/为什么/怎么做最好”这类问题，chat_reply 不要以“我没看懂/我没看清/你是想问”开头；先给原则性回答，再说明如果要精确核验代码可以进入开发流程。
- 参考回答方向：多手机登录或多端聊天可以并行；同一项目的代码修改需要任务会话、worktree/分支、队列或合并保护，否则会有冲突风险。
- 只有确实无法理解用户在问什么时，chat_reply 才问一个简短澄清问题。
- 如果 route 是 development，chat_reply 置为空字符串。
- 只输出一行 JSON，不要 Markdown，不要代码块，不要额外解释。

JSON 格式：
{{"route":"chat|development","confidence":0.0,"reason":"简短原因","chat_reply":"中文回复或空字符串"}}

用户消息：
{user_message}"#,
        provider = option.provider,
        model_text = model_text,
        workspace = workspace.display(),
        user_message = user_message
    )
}

fn build_development_cli_prompt(
    workspace: &Path,
    user_message: &str,
    preflight_note: Option<&str>,
    option: &AiCliOption,
) -> String {
    let model_text = option
        .model
        .as_deref()
        .map(|model| format!("，当前模型：{}", model))
        .unwrap_or_default();
    let preflight_text = preflight_note
        .map(|note| {
            format!(
                r#"
项目预检结果：
{note}

这不是最终失败。请先把它当作当前任务的一部分处理：进入工作区后查看 git status/diff，保护已有改动，不要丢弃用户或其他 AI 的工作；能安全提交、stash、创建 worktree 或 rebase 时自行处理，再继续用户原始请求。若无法判断这些未提交改动是否该保留，请用用户可见说明讲清楚并暂停等待确认。
"#
            )
        })
        .unwrap_or_default();
    format!(
        r#"你是「一龙」平台服务器上的本地 AI CLI 编程助手。

当前 CLI：{provider}{model_text}
当前工作目录是当前项目工作区：
{workspace}
{preflight_text}

请直接处理用户请求。规则：
- 只在当前项目工作区内读写文件，不要访问其他用户工作区。当前项目可能是模板项目、GitHub 导入项目，也可能是一龙平台自身源码；一律按普通项目处理，不要使用特殊旁路。
- 如果只是普通问答或咨询，不需要改文件，请直接用简洁中文回复。
- 如果需要创建或修改项目代码，先执行项目侦察：查看目录结构和 git 状态；如果存在 AGENTS.md、CODEX.md、.github/copilot-instructions.md、.github/instructions/*.md、README.md、docs/ai-agent-workflow.md、docs/system-architecture.md 或任务相关 docs，必须先阅读这些项目说明，再编辑文件。
- 项目规则和长期记忆以仓库文件为准，CLI 自身没有跨任务魔法记忆；如果本次改变了流程或约定，请同步更新项目内说明文档并提交。
- 如果以后服务端把其他 AI 模型的分类、摘要、图片或特殊分析结果交给你，它们只是旁路证据；你仍然是当前 APK 会话的主执行上下文，必须把这些结论纳入当前 Codex CLI 原生 session 后继续处理，不要另起独立主会话。
- 对已有 Git 项目或 local_path/GitHub 项目：修改前先 fetch 并查看 git 状态；工作区干净才 git pull --rebase origin main。如果上方项目预检结果提示 pull 失败或工作区有未提交改动，不要反复盲目执行同一个失败命令；先查看 git status/diff 处理现场。本任务自己的未提交改动可 stash/rebase/pop；其他任务或来源不明的未提交改动必须从 origin/main 新建 worktree。修改后运行必要检查，git add 仅加入本次任务文件，git commit，并在配置 origin 时 git push origin main。push 被拒绝时先 rebase 再 push，不要 force push。
- 通用项目工作流必须始终执行：确认项目路径和 Git 权限；读取 AGENTS.md、CODEX.md、README.md、.github/instructions、任务相关 docs；按项目自己的规则开发；验证、commit、push；共享动作（merge/main、版本号递增、APK 发布、服务器部署）必须串行。
- 新项目是未知的，不能假装有长期记忆；如果没有项目说明文档，使用平台默认流程并建议用户补充项目说明。不要把一龙自项目当特殊项目，也不要把一龙自项目的发布规则套到无关项目。
- 如果改动影响服务器运行，必须递增 server/Cargo.toml 的 package.version，在 commit/push 后用本地开发机运行 scripts/publish-server.ps1 或 scripts/publish-server.sh 交叉编译并上传 binary；生产服务器性能较弱，只负责接收 binary、重启和健康检查，不要把它当常规编译机。部署后验证 /health 和 /api/server/version。
- 如果改动影响 Android APK 发布给用户，必须递增 android/app/build.gradle 里的 versionCode 和 versionName，在本地开发机构建签名 release APK，上传最新 APK 和 version.json，再验证下载地址。签名文件应来自项目本机配置（例如 android/app/elon-release.jks 或环境变量），不得提交密钥。
- 对普通用户项目，遵循该项目自己的 README/AGENTS/CODEX/文档；不要把一龙自项目的服务器发布规则套到无关项目，除非该项目文档要求。
- 开始执行前，先用 1-2 句自然中文回应用户：说清楚你理解到的具体需求，以及接下来会先检查或修改哪里。为了让客户端识别，这一行必须以「用户可见：」开头。不要使用固定模板，不要提“CLI/后台/工作区”，不要承诺还没有完成的结果。
- 执行过程中，只有当你有新的判断、阻塞、构建失败原因或下一步取舍时，才补充简短中文说明；这类说明也必须以「用户可见：」开头。命令细节和文件列表不需要写给用户。
- 除了真正要展示给用户的自然说明外，不要在其他位置输出「用户可见：」。
- 如果用户要 Android APK，优先复用当前目录已有项目；空目录时可以根据需求新建项目，能构建时请运行构建并在最终回复里写出 APK 路径。
- 修改代码后请在最终回复里说明改了什么、验证了什么；不要编造没有运行过的检查。
- 回复用户使用中文，内容清楚但不要过长。

用户请求：
{user_message}"#,
        provider = option.provider,
        model_text = model_text,
        workspace = workspace.display(),
        preflight_text = preflight_text,
        user_message = user_message
    )
}

fn parse_intent_gate_result(stdout: &str) -> Result<IntentGateResult> {
    let text = extract_json_agent_message(stdout).unwrap_or_else(|| stdout.trim().to_string());
    let value = parse_json_object_from_text(&text)
        .ok_or_else(|| anyhow!("Codex CLI 意图确认没有返回有效 JSON"))?;
    let route_text = value
        .get("route")
        .and_then(Value::as_str)
        .unwrap_or("chat")
        .trim()
        .to_ascii_lowercase();
    let route = match route_text.as_str() {
        "development" | "code" | "codeagent" | "dev" => intent_router::CapabilityRoute::CodeAgent,
        _ => intent_router::CapabilityRoute::ChatAgent,
    };
    let confidence = value
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let chat_reply = value
        .get("chat_reply")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|reply| !reply.is_empty())
        .map(ToOwned::to_owned);

    Ok(IntentGateResult {
        route,
        confidence,
        reason,
        chat_reply,
    })
}

fn parse_json_object_from_text(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    })
}

fn ensure_git(workspace: &Path, user_id: &str, require_existing_git: bool) -> Result<()> {
    if workspace.join(".git").exists() && has_origin_remote(workspace) {
        return Ok(());
    }

    if require_existing_git {
        return Err(anyhow!(
            "当前项目被标记为 Git/local_path 项目，但工作目录 {} 不是带 origin 远端的 Git 仓库。请先把它设置成真实 git clone（包含 .git 和 origin/main），再让 AI 修改。",
            workspace.display()
        ));
    }

    let _ = std::process::Command::new("git")
        .args(["init"])
        .current_dir(workspace)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", &format!("{}@elon.app", user_id)])
        .current_dir(workspace)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", user_id])
        .current_dir(workspace)
        .output();

    Ok(())
}

fn has_origin_remote(workspace: &Path) -> bool {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

fn environment_notes(user_message: &str, option: &AiCliOption) -> Vec<String> {
    let mut notes = Vec::new();
    if looks_like_android_task(user_message) {
        if option.bin.contains("codex") && !codex_auth_configured() {
            notes.push("环境提醒：AI CLI 登录状态异常，可能会自动切换备用代理。".into());
        }
        if !command_available("git") {
            notes.push("环境提醒：服务器未检测到 git，项目保存可能失败。".into());
        }
        if !command_available("java") {
            notes.push("环境提醒：服务器未检测到 java，Android Gradle 构建会失败。".into());
        }
        if !android_sdk_configured() {
            notes.push(
                "环境提醒：服务器未检测到 Android SDK，请先安装 SDK 后再稳定打包 APK。".into(),
            );
        }
    }
    notes
}

fn codex_auth_configured() -> bool {
    if std::env::var("OPENAI_API_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    let codex_home = std::env::var("CODEX_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".codex"))
        });

    codex_home
        .map(|home| home.join("auth.json").exists())
        .unwrap_or(false)
}

fn command_available(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn android_sdk_configured() -> bool {
    let candidates = [
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        Some("/root/android-sdk".into()),
        Some("/opt/android-sdk".into()),
    ];

    candidates
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .any(|path| path.join("platforms").exists() || path.join("cmdline-tools").exists())
}

fn looks_like_android_task(user_message: &str) -> bool {
    let lower = user_message.to_ascii_lowercase();
    lower.contains("apk")
        || lower.contains("android")
        || user_message.contains("安卓")
        || user_message.contains("应用")
        || user_message.contains("打包")
        || user_message.contains("编译")
}

fn format_cli_reply(stdout: &str, stderr: &str, success: bool) -> String {
    let extracted;
    let primary = if stdout.trim().is_empty() {
        extracted = extract_codex_answer(stderr);
        extracted.as_deref().unwrap_or(stderr)
    } else if let Some(answer) = extract_json_agent_message(stdout) {
        extracted = Some(answer);
        extracted.as_deref().unwrap_or(stdout)
    } else {
        stdout
    };
    let clean = truncate_chars(strip_ansi(primary).trim(), 8000);

    if clean.is_empty() {
        if success {
            "本地 AI CLI 已完成处理。".into()
        } else {
            "AI 助手尝试处理这个流程，但没有返回可读的失败原因。请稍后重试；如果问题持续出现，需要人工确认当前 Git 工作区状态后再继续。".into()
        }
    } else if success {
        clean
    } else {
        format!(
            "{}\n\n这次 AI 助手已经尝试自行处理，但流程没有正常完成。请根据上面的原因确认下一步，或稍后重试。",
            clean
        )
    }
}

fn extract_thread_id(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value.get("type").and_then(Value::as_str) == Some("thread.started") {
            value
                .get("thread_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

fn extract_json_agent_message(stdout: &str) -> Option<String> {
    let mut latest = None;
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("agent_message") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            latest = Some(text.to_string());
        }
    }
    latest
}

fn extract_codex_answer(stderr: &str) -> Option<String> {
    let clean = strip_ansi(stderr);
    let mut answers = Vec::<String>::new();
    let mut collecting = false;
    let mut current = Vec::<String>::new();

    for raw in clean.lines() {
        let line = raw.trim();
        if line == "codex" {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = true;
            continue;
        }

        if collecting && is_codex_block_boundary(line) {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = false;
            continue;
        }

        if collecting && !is_noisy_codex_answer_line(line) {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        answers.push(current.join("\n").trim().to_string());
    }

    answers
        .into_iter()
        .rev()
        .find(|answer| !answer.trim().is_empty())
}

fn is_codex_block_boundary(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "user" | "exec" | "tokens used" | "tool" | "system" | "assistant" | "output:"
    ) || lower.starts_with("openai codex")
        || lower.starts_with("workdir:")
        || lower.starts_with("model:")
        || lower.starts_with("provider:")
        || lower.starts_with("approval:")
        || lower.starts_with("sandbox:")
        || lower.starts_with("reasoning")
        || lower.starts_with("session id:")
        || lower.starts_with("wall time:")
        || lower.starts_with("process exited")
        || lower.starts_with("original token count:")
        || lower.starts_with("/bin/")
        || lower.starts_with("succeeded in")
        || lower.starts_with("failed in")
        || lower.starts_with("error:")
        || lower.starts_with("warn")
        || lower.contains(" event.timestamp=")
        || lower.contains("mcp_server=")
}

fn is_noisy_codex_answer_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.is_empty()
        || lower.contains("feedback_tags")
        || lower.contains("model_client.")
        || lower.contains("responses_websocket")
        || lower.contains("event.timestamp=")
        || lower.contains("mcp_server=")
        || lower.contains("auth_header")
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }

    out
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{}...\n\n（输出过长，已截断）", truncated)
    } else {
        truncated
    }
}
