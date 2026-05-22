use anyhow::{anyhow, Result};
use std::{path::Path, process::Stdio, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command, sync::mpsc::UnboundedSender};

use crate::{
    tools,
    types::{AiCliOption, AppState, CliPromptMode, WsMessage},
};

pub async fn run(
    user_id: &str,
    workspace_user_id: &str,
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

    let workspace = state.get_user_workspace(workspace_user_id);
    std::fs::create_dir_all(&workspace)?;
    ensure_git(&workspace, user_id)?;

    let _ = tx.send(
        WsMessage::Progress {
            message: format!("正在调用本地 {} 处理请求...", option.label),
        }
        .to_json(),
    );

    let prompt = build_cli_prompt(&workspace, user_message, &option);
    let output = run_cli_command(&option, &workspace, &prompt).await?;
    let reply = format_cli_reply(&output.stdout, &output.stderr, output.success);

    let apk_url = tools::find_latest_apk(&workspace).map(|apk| {
        let apk_name = apk
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        format!(
            "{}/download/{}/{}",
            state.public_url, workspace_user_id, apk_name
        )
    });

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

    if option.prompt_mode == CliPromptMode::Stdin {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
        }
    }

    let output = tokio::time::timeout(
        Duration::from_secs(option.timeout_secs),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| anyhow!("本地 AI CLI 执行超时，请稍后重试或调大对应 TIMEOUT_SECS"))??;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(anyhow!(
            "本地 AI CLI 执行失败: {}",
            compact_failure(&stdout, &stderr)
        ));
    }

    Ok(CliOutput {
        success: output.status.success(),
        stdout,
        stderr,
    })
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

fn format_cli_reply(stdout: &str, stderr: &str, success: bool) -> String {
    let primary = if stdout.trim().is_empty() {
        stderr
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
