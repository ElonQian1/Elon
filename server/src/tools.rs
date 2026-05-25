use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{info, warn};

pub use crate::tools_git::{git_commit, git_pull_rebase};

/// AI 代理可以调用的所有工具

/// 读取文件内容（限制在 project_root 内）
pub fn read_file(project_root: &Path, relative_path: &str) -> Result<String> {
    let full_path = safe_path(project_root, relative_path)?;
    info!("[工具] 读取文件: {}", full_path.display());
    Ok(std::fs::read_to_string(&full_path)?)
}

/// 写入文件内容（限制在 project_root 内）
pub fn write_file(project_root: &Path, relative_path: &str, content: &str) -> Result<String> {
    let full_path = safe_path(project_root, relative_path)?;
    info!("[工具] 写入文件: {}", full_path.display());
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, content)?;
    Ok(format!("已写入 {} ({} 字节)", relative_path, content.len()))
}

/// 列出目录内容
pub fn list_dir(project_root: &Path, relative_path: &str) -> Result<String> {
    let full_path = safe_path(project_root, relative_path)?;
    info!("[工具] 列出目录: {}", full_path.display());
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&full_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type()?.is_dir();
        entries.push(if is_dir { format!("{}/", name) } else { name });
    }
    entries.sort();
    Ok(entries.join("\n"))
}

/// 创建产品级项目工作区：初始化模板、Git 仓库和首个提交。
pub fn create_project_workspace(
    project_root: &Path,
    project_type: &str,
    project_name: &str,
    user_id: &str,
) -> Result<String> {
    info!("[工具] 创建项目工作区: {}", project_root.display());
    std::fs::create_dir_all(project_root)?;

    if is_dir_empty(project_root)? {
        match project_type {
            "android" => {
                if let Some(template_dir) = template_dir("android") {
                    copy_dir_all(&template_dir, project_root)?;
                } else {
                    std::fs::write(
                        project_root.join("README.md"),
                        format!(
                            "# {}\n\nAndroid 项目工作区已创建。模板目录尚未配置，AI 可以在后续任务中补齐项目代码。\n",
                            project_name
                        ),
                    )?;
                }
            }
            _ => return Err(anyhow!("未知模板类型: {}，目前支持: android", project_type)),
        }
    }

    ensure_git_repo(project_root, user_id)?;
    let _ = git_commit(project_root, "chore: initialize project")?;
    Ok(format!(
        "项目工作区已创建: {}",
        project_root.to_string_lossy()
    ))
}

/// 构建项目（自动检测项目类型，从工作区根目录开始）
/// target: "android" | "rust" | "frontend"
/// 成功时返回值中若包含 ##APK_FILE:<name>，表示有 APK 可供下载
pub fn build_project(project_root: &Path, target: &str) -> Result<String> {
    info!("[工具] 构建项目: {}", target);

    let (work_dir, cmd) = match target {
        "android" => {
            // 优先检查工作区根目录，再检查 android/ 子目录
            let work_dir = if project_root.join("gradlew").exists() {
                project_root.to_path_buf()
            } else if project_root.join("android").join("gradlew").exists() {
                project_root.join("android")
            } else {
                return Err(anyhow!(
                    "未找到 gradlew。请先调用 init_project 工具初始化 Android 项目模板"
                ));
            };
            (work_dir, "chmod +x gradlew && ./gradlew assembleDebug 2>&1")
        }
        "rust" => {
            let work_dir = if project_root.join("Cargo.toml").exists() {
                project_root.to_path_buf()
            } else if project_root.join("server").join("Cargo.toml").exists() {
                project_root.join("server")
            } else {
                return Err(anyhow!("未找到 Cargo.toml"));
            };
            (work_dir, "cargo build --release 2>&1")
        }
        "frontend" => {
            let work_dir = if project_root.join("package.json").exists() {
                project_root.to_path_buf()
            } else {
                project_root.join("frontend")
            };
            if !work_dir.join("package.json").exists() {
                return Err(anyhow!("未找到 package.json"));
            }
            (work_dir, "npm run build 2>&1")
        }
        _ => {
            return Err(anyhow!(
                "未知构建目标: {}，支持: android/rust/frontend",
                target
            ))
        }
    };

    // Android 构建前自动递增版本号（versionCode +1，versionName PATCH +1）
    let version_note = if target == "android" {
        crate::tools_git::bump_android_version(&work_dir)
    } else {
        String::new()
    };
    if !version_note.is_empty() {
        info!("[工具] {}", version_note);
    }

    let output = Command::new("bash")
        .args(["-c", cmd])
        .current_dir(&work_dir)
        .output()?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        return Err(anyhow!(
            "构建失败:\n{}",
            &combined[..combined.len().min(2000)]
        ));
    }

    // Android 构建成功后找到 APK 文件，嵌入特殊标记
    if target == "android" {
        if let Some(apk_path) = find_latest_apk(&work_dir) {
            let apk_name = apk_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            return Ok(format!(
                "android 构建成功\n{}\n##APK_FILE:{}\n\n构建日志:\n{}",
                version_note,
                apk_name,
                &combined[..combined.len().min(800)]
            ));
        }
    }

    Ok(format!(
        "{} 构建成功\n{}",
        target,
        &combined[..combined.len().min(500)]
    ))
}

/// 在用户工作区初始化项目模板（复制服务器上预置的模板）
pub fn init_project(project_root: &Path, project_type: &str) -> Result<String> {
    info!("[工具] 初始化项目模板: {}", project_type);
    match project_type {
        "android" => {
            let template_dir = template_dir("android");
            let Some(template_dir) = template_dir else {
                return Err(anyhow!(
                    "服务器上尚未设置 Android 模板，请配置 ANDROID_TEMPLATE_DIR 或 TEMPLATE_ROOT"
                ));
            };
            copy_dir_all(&template_dir, project_root)?;
            Ok("Android 项目模板已初始化。\n\
                 现在请用 write_file 修改以下文件实现具体功能:\n\
                 - app/src/main/kotlin/com/template/app/MainActivity.kt\n\
                 - app/src/main/res/layout/activity_main.xml\n\
                 - app/src/main/AndroidManifest.xml\n\
                 - settings.gradle（修改应用名）\n\
                 - app/build.gradle（修改包名 applicationId）"
                .into())
        }
        _ => Err(anyhow!("未知模板类型: {}，目前支持: android", project_type)),
    }
}

fn template_dir(project_type: &str) -> Option<PathBuf> {
    let specific_key = format!("{}_TEMPLATE_DIR", project_type.to_ascii_uppercase());
    if let Ok(path) = std::env::var(specific_key) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let root = std::env::var("TEMPLATE_ROOT").unwrap_or_else(|_| "/root/templates".into());
    let path = PathBuf::from(root).join(project_type);
    path.exists().then_some(path)
}

fn ensure_git_repo(project_root: &Path, user_id: &str) -> Result<()> {
    crate::tools_git::ensure_git_repo(project_root, user_id)
}

fn is_dir_empty(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

pub use crate::tools_apk::{
    find_apk_by_filename, find_download_apk, find_latest_apk, stable_apk_url, STABLE_APK_FILENAME,
};

/// 执行受限 shell 命令（只允许白名单命令）
pub fn run_shell(project_root: &Path, command: &str) -> Result<String> {
    // 白名单：只允许安全的只读或构建相关命令
    const ALLOWED_PREFIXES: &[&str] = &[
        "echo",
        "cargo check",
        "cargo test",
        "cargo clippy",
        "git log",
        "git diff",
        "git status",
        "ls",
        "cat",
        "find",
        "grep",
        "./gradlew",
    ];

    let is_allowed = ALLOWED_PREFIXES
        .iter()
        .any(|prefix| command.starts_with(prefix));
    if !is_allowed {
        warn!("[工具] 拒绝执行命令: {}", command);
        return Err(anyhow!(
            "命令 '{}' 不在允许列表中。允许的命令前缀: {}",
            command,
            ALLOWED_PREFIXES.join(", ")
        ));
    }

    info!("[工具] 执行命令: {}", command);
    let output = Command::new("bash")
        .args(["-c", command])
        .current_dir(project_root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("{}\n{}", stdout.trim(), stderr.trim())
        .trim()
        .to_string())
}

/// PC 端 elon 项目路径（ELON_SELF_PC_PATH 环境变量，默认 Windows 路径）
pub fn elon_pc_project_path() -> String {
    std::env::var("ELON_SELF_PC_PATH")
        .unwrap_or_else(|_| r"D:\rust\active-projects\elon cli".into())
}

/// 通过 homecli PC 代理执行命令，收集完整输出并将进度实时推送给 APK 客户端
pub async fn exec_via_agent(
    state: &Arc<crate::types::AppState>,
    cli: &str,
    args: Vec<String>,
    cwd: &str,
    progress_tx: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String> {
    use crate::types::WsMessage;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use homecli_proto::AgentToServer;

    let agents = state.agent_manager.list().await;
    let agent_id = agents
        .first()
        .ok_or_else(|| anyhow!("没有可用的 PC agent，请确认 homecli 已启动并连接到服务器"))?
        .agent_id
        .clone();

    let (_task_id, mut rx) = state
        .agent_manager
        .dispatch(&agent_id, cli.to_string(), args, cwd.to_string(), vec![])
        .await?;

    info!(%agent_id, %cli, %cwd, "exec_via_agent: dispatched");

    let mut output_bytes = Vec::<u8>::new();
    let mut exit_code: Option<i32> = None;

    while let Some(msg) = rx.recv().await {
        match msg {
            AgentToServer::TaskStarted { pid, .. } => {
                info!(%agent_id, %pid, "exec_via_agent: task started");
                if let Some(tx) = progress_tx {
                    let _ = tx.send(
                        WsMessage::progress(format!("[PC agent] 任务启动 pid={}", pid))
                        .to_json(),
                    );
                }
            }
            AgentToServer::TaskStdout { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    if let Some(tx) = progress_tx {
                        let s = String::from_utf8_lossy(&bytes);
                        for line in s.lines().filter(|l| !l.trim().is_empty()) {
                            let _ = tx.send(
                                WsMessage::progress(line.to_string())
                                .to_json(),
                            );
                        }
                    }
                    output_bytes.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskStderr { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    if let Some(tx) = progress_tx {
                        let s = String::from_utf8_lossy(&bytes);
                        for line in s.lines().filter(|l| !l.trim().is_empty()) {
                            let _ = tx.send(
                                WsMessage::progress(format!("[stderr] {}", line))
                                .to_json(),
                            );
                        }
                    }
                    output_bytes.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskExit { code, .. } => {
                exit_code = code;
                break;
            }
            AgentToServer::TaskError { message, .. } => {
                return Err(anyhow!("PC agent 任务失败: {}", message));
            }
            _ => {}
        }
    }

    let output = String::from_utf8_lossy(&output_bytes).to_string();
    match exit_code {
        Some(0) | None => Ok(output),
        Some(code) => Err(anyhow!(
            "PC agent 退出码 {}\n{}",
            code,
            &output[..output.len().min(2000)]
        )),
    }
}

/// 通过 homecli PC 代理触发项目构建脚本（android/rust），失败时调用者可回退到本地构建
pub async fn build_project_via_agent(
    state: &Arc<crate::types::AppState>,
    target: &str,
    changelog: &str,
    progress_tx: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String> {
    let pc_cwd = elon_pc_project_path();
    let (cli, args): (&str, Vec<String>) = match target {
        "android" => (
            "pwsh",
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                r"scripts\publish-apk.ps1".into(),
                "-Changelog".into(),
                changelog.to_string(),
            ],
        ),
        "rust" => (
            "pwsh",
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                r"scripts\publish-server.ps1".into(),
            ],
        ),
        _ => {
            return Err(anyhow!(
                "PC agent 不支持构建目标: {}（支持: android / rust）",
                target
            ))
        }
    };
    let output = exec_via_agent(state, cli, args, &pc_cwd, progress_tx).await?;
    if target == "android" {
        return Ok(format!(
            "android 构建成功（PC agent）\n##APK_FILE:ElonSpeed-latest.apk\n\n{}",
            &output[..output.len().min(500)]
        ));
    }
    Ok(format!(
        "{} 构建成功（PC agent）\n{}",
        target,
        &output[..output.len().min(500)]
    ))
}

/// 安全路径：确保路径不会逃出 project_root
fn safe_path(project_root: &Path, relative_path: &str) -> Result<std::path::PathBuf> {
    // 拒绝包含 .. 的路径
    if relative_path.contains("..") {
        return Err(anyhow!("路径不允许包含 '..': {}", relative_path));
    }
    let full = project_root.join(relative_path);
    // 规范化后再检查是否仍在 project_root 内
    let canonical_root = project_root
        .canonicalize()
        .unwrap_or(project_root.to_path_buf());
    // 注意：文件可能还不存在，无法 canonicalize，用前缀检查
    let full_str = full.to_string_lossy();
    let root_str = canonical_root.to_string_lossy();
    if !full_str.starts_with(root_str.as_ref()) {
        return Err(anyhow!("路径越界: {}", relative_path));
    }
    Ok(full)
}
