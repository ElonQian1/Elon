use anyhow::{anyhow, Result};
use std::{
    path::Path,
    process::Stdio,
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, process::Command, sync::mpsc::UnboundedSender};

use super::{
    ai_cli_streaming::{current_unix_millis, read_cli_stream, send_cli_heartbeat},
    ai_cli_trace::{
        record_cli_done, record_cli_error, record_cli_start, record_codex_network_gate,
        CliTraceContext,
    },
};
use crate::types::{AiCliOption, CliPromptMode};

pub(crate) struct CliOutput {
    pub(crate) success: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn supports_codex_sessions(option: &AiCliOption) -> bool {
    option.provider.eq_ignore_ascii_case("codex")
        || option.id.to_ascii_lowercase().contains("codex")
        || option
            .bin
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .map(|bin| bin.eq_ignore_ascii_case("codex"))
            .unwrap_or(false)
}

/// CopilotCLI 原生会话检测。
///
/// 会话续接策略：已有 `native_session_id` → 在 args 前插入 `--continue`；
/// 首次运行成功后自动写入 sentinel key，下次请求即可续接。
pub(crate) fn supports_copilot_sessions(option: &AiCliOption) -> bool {
    option.provider.eq_ignore_ascii_case("copilot")
        || option.id.to_ascii_lowercase().contains("copilot")
        || option
            .bin
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .map(|bin| bin.eq_ignore_ascii_case("copilot"))
            .unwrap_or(false)
}

/// 任意原生会话支持（Codex 或 CopilotCLI）。
pub(crate) fn supports_any_native_sessions(option: &AiCliOption) -> bool {
    supports_codex_sessions(option) || supports_copilot_sessions(option)
}

pub(crate) fn configured_timeout_cap(env_name: &str, default_secs: u64) -> u64 {
    std::env::var(env_name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| (1..=3600).contains(secs))
        .unwrap_or(default_secs)
}

pub(crate) fn cap_option_timeout(option: &mut AiCliOption, cap_secs: u64) {
    let cap_secs = cap_secs.max(1);
    if option.timeout_secs == 0 || option.timeout_secs > cap_secs {
        option.timeout_secs = cap_secs;
    }
}

pub(crate) fn is_cli_timeout_error(error: &anyhow::Error) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("timeout") || text.contains("timed out") || text.contains("执行超时")
}

pub(crate) async fn run_cli_command_traced(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
    tx: &UnboundedSender<String>,
    trace: Option<CliTraceContext<'_>>,
) -> Result<CliOutput> {
    let trace_started = Instant::now();
    if supports_codex_sessions(option) {
        if let Some(trace) = trace {
            if let Err(error) = trace
                .state
                .codex_network
                .ensure_ready(trace.operation)
                .await
            {
                record_codex_network_gate(trace, option, "blocked", &error);
                return Err(anyhow!(error));
            }
        }
    }
    if let Some(trace) = trace {
        record_cli_start(trace, option, workspace, prompt, native_session_id);
    }
    let result = run_cli_command(option, workspace, prompt, native_session_id, tx).await;
    if let Some(trace) = trace {
        match &result {
            Ok(output) => record_cli_done(
                trace,
                option,
                native_session_id,
                output,
                trace_started.elapsed().as_millis(),
            ),
            Err(error) => record_cli_error(
                trace,
                option,
                native_session_id,
                error,
                trace_started.elapsed().as_millis(),
            ),
        }
        if supports_codex_sessions(option) {
            match &result {
                Ok(output) if output.success => {
                    trace
                        .state
                        .codex_network
                        .mark_cli_success("codex_cli_success")
                        .await;
                }
                Ok(output) => {
                    let combined = format!("{}\n{}", output.stdout, output.stderr);
                    if crate::codex_health::is_codex_network_error_text(&combined) {
                        trace
                            .state
                            .codex_network
                            .mark_cli_failure("codex_cli_output", &combined)
                            .await;
                    }
                }
                Err(error) => {
                    let text = error.to_string();
                    if is_cli_timeout_error(error)
                        || crate::codex_health::is_codex_network_error_text(&text)
                    {
                        trace
                            .state
                            .codex_network
                            .mark_cli_failure("codex_cli_error", &text)
                            .await;
                    }
                }
            }
        }
    }
    result
}

async fn run_cli_command(
    option: &AiCliOption,
    workspace: &Path,
    prompt: &str,
    native_session_id: Option<&str>,
    tx: &UnboundedSender<String>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(&option.bin);
    let args = super::ai_cli_runner::cli_args_for_run(option, native_session_id);
    cmd.args(&args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    // 为每个 worktree 隔离 Gradle 守护进程，防止同一项目多个并发编译任务争抢
    // ~/.gradle/daemon 文件锁导致 CLI 卡死。
    // - GRADLE_USER_HOME 按 worktree 隔离：/opt/elon/gradle-homes/<workspace_key>/
    // - wrapper/dists（发行版 zip，约 100MB）符号链接到共享目录，只下载一次
    // - daemon/ 和 caches/ 各自独立，无锁竞争
    // - worktree 合并完成后，project_conversation_workspace 会异步删除对应的 gradle-home
    {
        let workspace_key = workspace
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");
        let gradle_home = std::path::PathBuf::from("/opt/elon/gradle-homes").join(workspace_key);
        // 共享 Gradle 发行版目录：所有 worktree 复用已下载的 zip，节省带宽和时间
        #[cfg(unix)]
        {
            let shared_dists =
                std::path::PathBuf::from("/opt/elon/gradle-distributions/wrapper/dists");
            let _ = std::fs::create_dir_all(&shared_dists);
            let wrapper_dir = gradle_home.join("wrapper");
            let _ = std::fs::create_dir_all(&wrapper_dir);
            let dists_link = wrapper_dir.join("dists");
            if !dists_link.exists() {
                let _ = std::os::unix::fs::symlink(&shared_dists, &dists_link);
            }
        }
        cmd.env("GRADLE_USER_HOME", &gradle_home);
    }

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

    // last_activity_ms 记录 CLI 最近一次 stdout/stderr 出行的时间戳（毫秒）。
    // 心跳任务依赖这个反馈判断是否 CLI 静默，避免在 CLI 还在
    // 正常输出时发废话。
    let now_ms = current_unix_millis();
    let last_activity_ms = Arc::new(AtomicU64::new(now_ms));

    let stdout_task = tokio::spawn(read_cli_stream(
        stdout,
        Some(last_activity_ms.clone()),
        Some(tx.clone()),
        option.model.clone(),
    ));
    let stderr_task = tokio::spawn(read_cli_stream(
        stderr,
        Some(last_activity_ms.clone()),
        None,
        None,
    ));
    let heartbeat_task = tokio::spawn(send_cli_heartbeat(tx.clone(), last_activity_ms.clone()));

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
                stdout_task.abort();
                stderr_task.abort();
                super::ai_cli_runner::kill_timed_out_child(&mut child).await;
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
