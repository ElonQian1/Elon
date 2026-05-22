use anyhow::{anyhow, Result};
use std::{path::Path, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::UnboundedSender,
};

use crate::{
    tools,
    types::{AiCliOption, AppState, CliPromptMode, WsMessage},
};

pub async fn run_with_workspace(
    user_id: &str,
    workspace: &Path,
    download_base: &str,
    user_message: &str,
    option_id: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let option = state
        .ai_cli
        .find_option(option_id)
        .cloned()
        .ok_or_else(|| anyhow!("未找到可用本地 AI CLI 选项"))?;

    std::fs::create_dir_all(workspace)?;
    ensure_git(workspace, user_id)?;

    let _ = tx.send(
        WsMessage::Progress {
            message: format!("CLI 工作区已准备：{}", workspace.display()),
        }
        .to_json(),
    );
    for note in environment_notes(user_message, &option) {
        let _ = tx.send(WsMessage::Progress { message: note }.to_json());
    }
    let _ = tx.send(
        WsMessage::Progress {
            message: format!("正在启动本地 CLI：{}", option.command_preview()),
        }
        .to_json(),
    );

    let prompt = build_cli_prompt(workspace, user_message, &option);
    let output = run_cli_command(&option, workspace, &prompt, tx).await?;
    let reply = format_cli_reply(&output.stdout, &output.stderr, output.success);

    let _ = tx.send(
        WsMessage::Progress {
            message: "CLI 已结束，正在查找 APK 构建产物。".into(),
        }
        .to_json(),
    );
    let apk_url = tools::find_latest_apk(workspace).map(|apk| {
        let apk_name = apk
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        format!("{}/{}", download_base.trim_end_matches('/'), apk_name)
    });
    if apk_url.is_none() && looks_like_android_task(user_message) {
        let _ = tx.send(
            WsMessage::Progress {
                message: "未找到 APK 产物；请查看上面的 CLI 输出确认是否缺少 Java、Android SDK、Gradle 或构建步骤。".into(),
            }
            .to_json(),
        );
    }

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

async fn run_cli_command(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    tx: &UnboundedSender<String>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(&option.bin);
    cmd.args(&option.args)
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

    let stdout_task = tokio::spawn(read_cli_stream(stdout, "stdout", tx.clone()));
    let stderr_task = tokio::spawn(read_cli_stream(stderr, "stderr", tx.clone()));
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

    if !status.success() {
        return Err(anyhow!(
            "本地 AI CLI 执行失败: {}",
            compact_failure(&stdout, &stderr)
        ));
    }

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
                    message: "CLI 仍在运行，正在等待模型或构建结果。".into(),
                }
                .to_json(),
            )
            .is_err()
        {
            break;
        }
    }
}

async fn read_cli_stream<R>(reader: R, label: &'static str, tx: UnboundedSender<String>) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut collected = String::new();
    let mut forwarded = 0usize;

    while let Ok(Some(line)) = lines.next_line().await {
        collected.push_str(&line);
        collected.push('\n');

        let clean = strip_ansi(&line);
        let trimmed = clean.trim();
        if trimmed.is_empty() || !should_forward_cli_line(trimmed) {
            continue;
        }
        if forwarded < 160 {
            let _ = tx.send(
                WsMessage::Progress {
                    message: format!("CLI 输出({}): {}", label, truncate_chars(trimmed, 500)),
                }
                .to_json(),
            );
            forwarded += 1;
        } else if forwarded == 160 {
            let _ = tx.send(
                WsMessage::Progress {
                    message: format!("CLI 输出({})较多，后续只保留在最终摘要中。", label),
                }
                .to_json(),
            );
            forwarded += 1;
        }
    }

    collected
}

fn should_forward_cli_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("debug ") || lower.starts_with("trace ") {
        return false;
    }
    let noisy = [
        "using system root certificates",
        "models cache:",
        "otel.",
        "rpc.",
        "app_server.",
    ];
    !noisy.iter().any(|item| lower.contains(item))
}

fn build_cli_prompt(workspace: &Path, user_message: &str, option: &AiCliOption) -> String {
    let model_text = option
        .model
        .as_deref()
        .map(|model| format!("，当前模型：{}", model))
        .unwrap_or_default();
    format!(
        r#"你是「一龙」平台服务器上的本地 AI CLI 编程助手。

当前 CLI：{provider}{model_text}
当前工作目录是用户隔离工作区：
{workspace}

请直接处理用户请求。规则：
- 只在当前工作目录内读写用户项目文件，不要修改平台服务端源码，也不要访问其他用户工作区。
- 如果只是普通问答或咨询，不需要改文件，请直接用简洁中文回复。
- 如果需要创建或修改项目代码，请自主阅读目录、编辑文件、运行必要检查或构建。
- 如果用户要 Android APK，优先复用当前目录已有项目；空目录时可以根据需求新建项目，能构建时请运行构建并在最终回复里写出 APK 路径。
- 修改代码后请在最终回复里说明改了什么、验证了什么；不要编造没有运行过的检查。
- 回复用户使用中文，内容清楚但不要过长。

用户请求：
{user_message}"#,
        provider = option.provider,
        model_text = model_text,
        workspace = workspace.display(),
        user_message = user_message
    )
}

fn ensure_git(workspace: &Path, user_id: &str) -> Result<()> {
    if workspace.join(".git").exists() {
        return Ok(());
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

fn environment_notes(user_message: &str, option: &AiCliOption) -> Vec<String> {
    let mut notes = Vec::new();
    if option.bin.contains("codex") && !codex_auth_configured() {
        notes.push("环境提醒：服务器未检测到 Codex CLI 登录凭据或 OPENAI_API_KEY，本地 Codex CLI 可能会失败并回退到 API 代理。".into());
    }
    if !command_available("git") {
        notes.push("环境提醒：服务器未检测到 git，版本保存和部分 CLI 插件初始化可能失败。".into());
    }
    if looks_like_android_task(user_message) {
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
    } else {
        stdout
    };
    let clean = truncate_chars(strip_ansi(primary).trim(), 8000);

    if clean.is_empty() {
        if success {
            "本地 AI CLI 已完成处理。".into()
        } else {
            "本地 AI CLI 执行失败，但没有返回详细错误。".into()
        }
    } else {
        clean
    }
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

fn compact_failure(stdout: &str, stderr: &str) -> String {
    let combined = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let clean = strip_ansi(combined);
    truncate_chars(clean.trim(), 2000)
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
