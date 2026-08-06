//! CLI 探针：检测本机安装的 AI CLI 工具（codex / copilot / claude / gemini）。
//! 提供探针快照缓存、版本检测、鉴权状态和诊断建议。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use serde::Serialize;

// ── 探针缓存与超时常量 ────────────────────────────────────────────────────────

const CLI_PROBE_STALE_MS: u128 = 30_000;
const CODEX_RUN_CHECK_TIMEOUT: Duration = Duration::from_millis(900);
const GENERIC_CLI_RUN_CHECK_TIMEOUT: Duration = Duration::from_millis(700);

// ── 数据结构 ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub(super) struct LocalCliToolStatus {
    pub(super) name: String,
    pub(super) label: &'static str,
    pub(super) path: Option<String>,
    pub(super) version: Option<String>,
    pub(super) installed: bool,
    pub(super) runnable: bool,
    pub(super) logged_in: Option<bool>,
    pub(super) available: bool,
    pub(super) status: String,
    pub(super) detail: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) diagnosis: Option<String>,
    pub(super) fix_hint: Option<String>,
    pub(super) fix_action: String,
    pub(super) backend: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct LocalCliProbeSnapshot {
    pub(super) refreshed_at_ms: Option<u128>,
    pub(super) tools: Vec<LocalCliToolStatus>,
}

impl Default for LocalCliProbeSnapshot {
    fn default() -> Self {
        Self {
            refreshed_at_ms: None,
            tools: ["codex", "copilot", "claude", "gemini"]
                .into_iter()
                .map(|name| LocalCliToolStatus {
                    name: name.to_string(),
                    label: local_cli_display_label(name),
                    path: None,
                    version: None,
                    installed: false,
                    runnable: false,
                    logged_in: if matches!(name, "codex" | "claude" | "gemini") {
                        Some(false)
                    } else {
                        None
                    },
                    available: false,
                    status: "checking".to_string(),
                    detail: Some("正在后台检测，不阻塞 Win 端启动".to_string()),
                    reason: Some("checking".to_string()),
                    diagnosis: None,
                    fix_hint: None,
                    fix_action: "wait".to_string(),
                    backend: "cli",
                })
                .collect(),
        }
    }
}

impl LocalCliProbeSnapshot {
    pub(super) fn available_pairs(&self) -> Vec<(String, String)> {
        self.tools
            .iter()
            .filter(|tool| tool.available)
            .filter_map(|tool| {
                tool.path
                    .as_ref()
                    .map(|path| (tool.name.clone(), path.clone()))
            })
            .collect()
    }

    pub(super) fn available_names(&self) -> Vec<String> {
        self.available_pairs()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    pub(super) fn codex_status(&self) -> Option<LocalCliToolStatus> {
        self.tools.iter().find(|tool| tool.name == "codex").cloned()
    }

    pub(super) fn is_stale(&self) -> bool {
        self.refreshed_at_ms
            .map(|ms| now_epoch_ms().saturating_sub(ms) > CLI_PROBE_STALE_MS)
            .unwrap_or(true)
    }
}

// ── 探针函数 ──────────────────────────────────────────────────────────────────

fn local_cli_display_label(name: &str) -> &'static str {
    match name.trim().to_ascii_lowercase().as_str() {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => "本机 AI CLI",
    }
}

/// 检测本机可用的 CLI，返回快照。
pub(super) fn probe_local_clis() -> LocalCliProbeSnapshot {
    let tools = ["codex", "copilot", "claude", "gemini"]
        .into_iter()
        .map(probe_local_cli)
        .collect();
    LocalCliProbeSnapshot {
        refreshed_at_ms: Some(now_epoch_ms()),
        tools,
    }
}

fn probe_local_cli(name: &str) -> LocalCliToolStatus {
    let best_path = best_cli_path(name);
    match name {
        "codex" => probe_codex_cli(best_path),
        "claude" => probe_claude_cli(best_path),
        "copilot" => probe_copilot_cli(best_path),
        "gemini" => probe_gemini_cli(best_path),
        _ => probe_generic_cli(name, best_path),
    }
}

fn probe_claude_cli(best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let mut status = probe_generic_cli("claude", best_path);
    if !status.runnable {
        status.logged_in = Some(false);
        return status;
    }
    let Some(path) = status.path.as_deref().map(PathBuf::from) else {
        status.logged_in = Some(false);
        return status;
    };
    let auth = quick_command_status(&path, &["auth", "status"], Duration::from_secs(4));
    status.logged_in = Some(auth.success);
    status.available = auth.success;
    status.status = if auth.success {
        "ready"
    } else {
        "not_logged_in"
    }
    .to_string();
    status.detail = Some(if auth.success {
        "Claude Code 可运行，官方 auth status 已确认登录。".to_string()
    } else {
        "Claude Code 可运行，但官方 auth status 未确认登录。".to_string()
    });
    status.reason = (!auth.success).then(|| "not_logged_in".to_string());
    status.diagnosis = Some(if auth.success {
        "Win 端已找到可运行且已登录的 Claude Code。".to_string()
    } else {
        "Claude Code 本身能启动，但需要通过官方 claude auth login 登录。".to_string()
    });
    status.fix_hint = (!auth.success)
        .then(|| "请在节点页发起 Claude 官方登录；凭据继续由 Claude Code 保管。".to_string());
    status.fix_action = if auth.success { "none" } else { "login" }.to_string();
    status
}

fn probe_copilot_cli(best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let mut status = probe_generic_cli("copilot", best_path);
    if !status.runnable {
        return status;
    }
    let env_auth = ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]
        .into_iter()
        .any(env_key_present);
    status.logged_in = env_auth.then_some(true);
    status.detail = Some(if env_auth {
        "Copilot CLI 可运行，且检测到官方支持的 GitHub token 环境变量。".to_string()
    } else {
        "Copilot CLI 可运行；官方 CLI 未提供非交互 auth status，系统凭据状态将在登录完成后更新。"
            .to_string()
    });
    status
}

fn probe_gemini_cli(best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let mut status = probe_generic_cli("gemini", best_path);
    if !status.runnable {
        status.logged_in = Some(false);
        return status;
    }
    let auth = gemini_auth_configured();
    status.logged_in = Some(auth);
    status.available = auth;
    status.status = if auth { "ready" } else { "not_logged_in" }.to_string();
    status.detail = Some(if auth {
        "Gemini CLI 可运行，且由 Gemini CLI 自己保管的 Google/API 鉴权已就绪".to_string()
    } else {
        "Gemini CLI 可运行，但未检测到 Google 登录缓存、Gemini API Key 或 Vertex AI 鉴权"
            .to_string()
    });
    status.reason = (!auth).then(|| "not_logged_in".to_string());
    status.diagnosis = Some(if auth {
        "Win 端已找到可运行且已鉴权的 Gemini CLI。".to_string()
    } else {
        "Gemini CLI 本身能启动，但需要先通过官方 Gemini CLI 登录 Google 账号。".to_string()
    });
    status.fix_hint = (!auth)
        .then(|| "请在节点页发起 Gemini 官方登录；凭据将继续由 Gemini CLI 保管。".to_string());
    status.fix_action = if auth { "none" } else { "login" }.to_string();
    status.backend = "acp";
    status
}

fn probe_generic_cli(name: &str, best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let label = local_cli_display_label(name);
    let Some(path) = best_path else {
        return LocalCliToolStatus {
            name: name.to_string(),
            label,
            path: None,
            version: None,
            installed: false,
            runnable: false,
            logged_in: None,
            available: false,
            status: "not_installed".to_string(),
            detail: Some(format!("{label} CLI 未安装或不在 PATH 中")),
            reason: Some("not_found".to_string()),
            diagnosis: Some(format!("未在 PATH 和常见安装目录中找到 {label} 命令。")),
            fix_hint: Some("请安装对应 CLI，或把它的 bin 目录加入 PATH 后重新检测。".to_string()),
            fix_action: "install".to_string(),
            backend: "cli",
        };
    };
    let run = quick_command_status(&path, &["--version"], GENERIC_CLI_RUN_CHECK_TIMEOUT);
    let runnable = run.success || run.timed_out;
    LocalCliToolStatus {
        name: name.to_string(),
        label,
        path: Some(path.to_string_lossy().to_string()),
        version: if runnable { run.summary.clone() } else { None },
        installed: true,
        runnable,
        logged_in: None,
        available: runnable,
        status: if runnable { "ready" } else { "not_runnable" }.to_string(),
        detail: if runnable {
            run.summary
                .or_else(|| Some(format!("{label} CLI 已检测到")))
        } else {
            Some(
                run.summary
                    .unwrap_or_else(|| format!("{label} CLI 无法执行")),
            )
        },
        reason: if runnable {
            None
        } else {
            run.reason.or_else(|| Some("run_failed".to_string()))
        },
        diagnosis: if runnable {
            Some(format!("{label} CLI 可由 Win 端启动。"))
        } else {
            Some(format!("检测到 {label} 命令路径，但 Win 端无法启动它。"))
        },
        fix_hint: if runnable {
            None
        } else {
            Some("请修复该 CLI 安装或 PATH 后重新检测。".to_string())
        },
        fix_action: if runnable { "none" } else { "repair_path" }.to_string(),
        backend: "cli",
    }
}

fn probe_codex_cli(best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let label = local_cli_display_label("codex");
    let Some(path) = best_path else {
        return LocalCliToolStatus {
            name: "codex".to_string(),
            label,
            path: None,
            version: None,
            installed: false,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_installed".to_string(),
            detail: Some(
                "未检测到可运行的 Codex CLI；只安装 Codex 桌面端不一定会提供可调用的 codex 命令"
                    .to_string(),
            ),
            reason: Some("not_found".to_string()),
            diagnosis: Some(
                "没有找到可作为命令行工具启动的 Codex CLI。Codex 桌面端和 Codex CLI 是两层能力，桌面端不一定暴露可调用的 codex 命令。"
                    .to_string(),
            ),
            fix_hint: Some(
                "点击安装/修复，让 Win 端运行 OpenAI 官方 Windows 安装器；安装后重新检测。".to_string(),
            ),
            fix_action: "install".to_string(),
            backend: "cli",
        };
    };

    let run = quick_command_status(&path, &["--version"], CODEX_RUN_CHECK_TIMEOUT);
    if !run.success {
        return LocalCliToolStatus {
            name: "codex".to_string(),
            label,
            path: Some(path.to_string_lossy().to_string()),
            version: None,
            installed: true,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_runnable".to_string(),
            detail: Some(run.summary.clone().unwrap_or_else(|| {
                "检测到 codex 命令，但无法非交互执行；请安装 Codex CLI 或修复 PATH".to_string()
            })),
            reason: run
                .reason
                .clone()
                .or_else(|| Some("run_failed".to_string())),
            diagnosis: Some(codex_not_runnable_diagnosis(&path, run.reason.as_deref())),
            fix_hint: Some(codex_not_runnable_fix_hint(&path)),
            fix_action: "repair_path".to_string(),
            backend: "cli",
        };
    }

    let auth = codex_auth_configured();
    LocalCliToolStatus {
        name: "codex".to_string(),
        label,
        path: Some(path.to_string_lossy().to_string()),
        version: run.summary.clone(),
        installed: true,
        runnable: true,
        logged_in: Some(auth),
        available: auth,
        status: if auth { "ready" } else { "not_logged_in" }.to_string(),
        detail: if auth {
            Some("Codex CLI 可运行，且已检测到 API key 或本机 Codex 登录文件".to_string())
        } else {
            Some("Codex CLI 可运行，但未检测到 OPENAI_API_KEY 或本机 Codex 登录文件".to_string())
        },
        reason: if auth {
            None
        } else {
            Some("not_logged_in".to_string())
        },
        diagnosis: if auth {
            Some("Win 端已找到可运行的 Codex CLI，并检测到本机鉴权。".to_string())
        } else {
            Some("Codex CLI 本身能启动，但当前用户还没有可用的 Codex/OpenAI 鉴权。".to_string())
        },
        fix_hint: if auth {
            None
        } else {
            Some("请在此页保存 OpenAI API Key，或先在本机完成 Codex CLI 登录。".to_string())
        },
        fix_action: if auth { "none" } else { "login" }.to_string(),
        backend: "cli",
    }
}

fn best_cli_path(name: &str) -> Option<PathBuf> {
    let candidates_paths: Vec<PathBuf> = elon_pc_dev_runtime::command_candidates(name);
    if candidates_paths.is_empty() {
        return None;
    }

    let not_vscode = |p: &&PathBuf| {
        let lower = p.to_string_lossy().to_ascii_lowercase();
        !lower.contains("globalstorage") && !lower.contains("copilotcli\\copilot")
    };

    #[cfg(windows)]
    let best = candidates_paths
        .iter()
        .find(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".cmd") && not_vscode(p))
        .or_else(|| {
            candidates_paths
                .iter()
                .find(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".cmd"))
        })
        .or_else(|| candidates_paths.iter().find(not_vscode))
        .or_else(|| candidates_paths.first());

    #[cfg(not(windows))]
    let best = candidates_paths
        .iter()
        .find(not_vscode)
        .or_else(|| candidates_paths.first());

    best.cloned()
}

// ── 同步命令检测（带超时）────────────────────────────────────────────────────

struct QuickCommandStatus {
    success: bool,
    timed_out: bool,
    summary: Option<String>,
    reason: Option<String>,
}

fn quick_command_status(program: &Path, args: &[&str], timeout: Duration) -> QuickCommandStatus {
    let mut command = elon_pc_dev_runtime::command_from_path(program);
    command
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let reason = if error.kind() == std::io::ErrorKind::PermissionDenied {
                "permission_denied"
            } else {
                "spawn_failed"
            };
            return QuickCommandStatus {
                success: false,
                timed_out: false,
                summary: Some(format!("无法启动 {}：{error}", program.display())),
                reason: Some(reason.to_string()),
            };
        }
    };
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait_with_output() {
                    Ok(output) => QuickCommandStatus {
                        success: output.status.success(),
                        timed_out: false,
                        summary: first_cli_output_line(&output.stdout, &output.stderr).or_else(
                            || {
                                (!output.status.success()).then(|| {
                                    format!(
                                        "{} 退出码 {:?}",
                                        program.display(),
                                        output.status.code()
                                    )
                                })
                            },
                        ),
                        reason: (!output.status.success()).then(|| "exit_failed".to_string()),
                    },
                    Err(error) => QuickCommandStatus {
                        success: false,
                        timed_out: false,
                        summary: Some(format!("读取 {} 输出失败：{error}", program.display())),
                        reason: Some("output_failed".to_string()),
                    },
                };
            }
            Ok(None) if started_at.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return QuickCommandStatus {
                    success: false,
                    timed_out: true,
                    summary: Some(format!("{} 检测超时", program.display())),
                    reason: Some("timeout".to_string()),
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return QuickCommandStatus {
                    success: false,
                    timed_out: false,
                    summary: Some(format!("检测 {} 失败：{error}", program.display())),
                    reason: Some("run_failed".to_string()),
                };
            }
        }
    }
}

// ── Codex 诊断 ────────────────────────────────────────────────────────────────

fn codex_not_runnable_diagnosis(path: &Path, reason: Option<&str>) -> String {
    if is_codex_desktop_resource_path(path) {
        return "检测到的是 Codex 桌面端安装包里的受保护资源路径。Windows 可以启动桌面 App，但通常不允许一龙 Win 端直接把这个资源文件当命令行 CLI 调用。"
            .to_string();
    }
    match reason {
        Some("permission_denied") => {
            "Windows 拒绝启动这个 codex 路径；常见原因是 PATH 指到了受保护应用包、权限异常或安装残缺。".to_string()
        }
        Some("timeout") => {
            "codex --version 在短时间内没有返回；可能是命令卡在初始化、杀毒拦截或安装损坏。".to_string()
        }
        Some("exit_failed") => {
            "codex 命令能启动，但版本检测返回失败；通常是 CLI 安装损坏或依赖环境异常。".to_string()
        }
        _ => "检测到 codex 命令路径，但 Win 端无法用非交互方式启动它。".to_string(),
    }
}

fn codex_not_runnable_fix_hint(path: &Path) -> String {
    if is_codex_desktop_resource_path(path) {
        return "请点击安装/修复 Codex，让 Win 端运行 OpenAI 官方 Windows 安装器；如果已安装，请确保本地 Codex CLI 的 bin 目录排在 WindowsApps 桌面资源路径之前。"
            .to_string();
    }
    "请点击安装/修复 Codex，或重新安装 Codex CLI 后再检测。".to_string()
}

fn is_codex_desktop_resource_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("\\windowsapps\\")
        && lower.contains("\\openai.codex_")
        && lower.contains("\\app\\resources\\")
}

fn first_cli_output_line(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    [stdout, stderr].into_iter().find_map(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(240).collect())
    })
}

// ── Codex 鉴权检测 ────────────────────────────────────────────────────────────

fn codex_auth_configured() -> bool {
    let api_runtime = super::node_agent_api_runtime_config::status_from_env();
    if api_runtime.key_configured || env_key_present("CODEX_API_KEY") {
        return true;
    }
    codex_home_candidates()
        .into_iter()
        .any(|home| codex_auth_file_present(&home))
}

fn gemini_auth_configured() -> bool {
    if ["GEMINI_API_KEY", "GOOGLE_API_KEY"]
        .into_iter()
        .any(env_key_present)
    {
        return true;
    }
    if std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .map(PathBuf::from)
        .is_some_and(|path| non_empty_file(&path))
    {
        return true;
    }
    user_home_candidates().into_iter().any(|home| {
        [
            home.join(".gemini").join("oauth_creds.json"),
            home.join(".gemini").join("google_accounts.json"),
            home.join(".config")
                .join("gcloud")
                .join("application_default_credentials.json"),
        ]
        .into_iter()
        .any(|path| non_empty_file(&path))
    })
}

fn env_key_present(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn codex_home_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(home) = std::env::var("CODEX_HOME") {
        push_unique_path(&mut candidates, PathBuf::from(home));
    }
    for home in user_home_candidates() {
        push_unique_path(&mut candidates, home.join(".codex"));
    }
    candidates
}

fn user_home_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(key) {
            push_unique_path(&mut candidates, PathBuf::from(home));
        }
    }
    candidates
}

fn codex_auth_file_present(home: &Path) -> bool {
    if ["auth.json", "credentials.json"]
        .into_iter()
        .map(|name| home.join(name))
        .any(|path| non_empty_file(&path))
    {
        return true;
    }

    let config = home.join("config.toml");
    if !non_empty_file(&config) {
        return false;
    }
    std::fs::read_to_string(&config)
        .map(|body| {
            let lower = body.to_ascii_lowercase();
            lower.contains("api_key") || lower.contains("openai_api_key")
        })
        .unwrap_or(false)
}

fn non_empty_file(path: &Path) -> bool {
    path.exists()
        && std::fs::metadata(path)
            .map(|meta| meta.len() > 2)
            .unwrap_or(false)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }
    let key = path.to_string_lossy().to_ascii_lowercase();
    if !paths
        .iter()
        .any(|item| item.to_string_lossy().to_ascii_lowercase() == key)
    {
        paths.push(path);
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

// ── CLI 可用性错误组装 ────────────────────────────────────────────────────────

pub(super) fn cli_unavailable_after_refresh_error(
    name: &str,
    cached_error: anyhow::Error,
    refreshed: &LocalCliProbeSnapshot,
) -> anyhow::Error {
    let clean = name.trim().to_ascii_lowercase();
    let detail = refreshed
        .tools
        .iter()
        .find(|tool| tool.name.eq_ignore_ascii_case(&clean))
        .map(cli_probe_tool_detail)
        .unwrap_or_else(|| format!("刷新后仍未找到 {clean} CLI"));
    anyhow!("此 PC 节点刷新本机 CLI 后仍不能使用 {clean}：{detail}。上一轮缓存错误：{cached_error}")
}

fn cli_probe_tool_detail(tool: &LocalCliToolStatus) -> String {
    let mut parts = vec![format!("状态={}", tool.status)];
    if let Some(path) = tool
        .path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("路径={path}"));
    }
    if let Some(detail) = tool
        .detail
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(detail.to_string());
    }
    if let Some(reason) = tool
        .reason
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("原因={reason}"));
    }
    if let Some(hint) = tool
        .fix_hint
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("建议={hint}"));
    }
    parts.join("；")
}

// ── 单元测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod node_agent_cli_probe_status_tests {
    use super::{cli_probe_tool_detail, LocalCliToolStatus};

    #[test]
    fn cli_probe_tool_detail_keeps_actionable_context() {
        let detail = cli_probe_tool_detail(&LocalCliToolStatus {
            name: "codex".to_string(),
            label: "Codex",
            path: Some(r"C:\Users\me\AppData\Local\OpenAI\Codex\bin\abc\codex.exe".to_string()),
            version: None,
            installed: true,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_runnable".to_string(),
            detail: Some("检测到 codex 命令，但无法非交互执行".to_string()),
            reason: Some("spawn_failed".to_string()),
            diagnosis: None,
            fix_hint: Some("请修复该 CLI 安装或 PATH 后重新检测。".to_string()),
            fix_action: "repair_path".to_string(),
            backend: "cli",
        });

        assert!(detail.contains("状态=not_runnable"));
        assert!(detail.contains("路径=C:\\Users\\me"));
        assert!(detail.contains("原因=spawn_failed"));
        assert!(detail.contains("重新检测"));
    }
}
