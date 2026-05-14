use anyhow::{anyhow, Result};
use std::{path::Path, process::Command};
use tracing::{info, warn};

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

/// 执行 git commit
pub fn git_commit(project_root: &Path, message: &str) -> Result<String> {
    info!("[工具] git commit: {}", message);
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(project_root)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("git add 失败: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project_root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        // 如果是 nothing to commit，不算错误
        if stderr.contains("nothing to commit") || stdout.contains("nothing to commit") {
            return Ok("无变更，跳过 commit".into());
        }
        return Err(anyhow!("git commit 失败: {}", stderr));
    }
    Ok(format!("git commit 成功: {}", stdout.trim()))
}

/// 构建项目
/// target: "rust" | "android" | "frontend"
pub fn build_project(project_root: &Path, target: &str) -> Result<String> {
    info!("[工具] 构建项目: {}", target);
    let (cmd, args, work_dir) = match target {
        "rust" => (
            "cargo",
            vec!["build", "--release"],
            project_root.join("server"),
        ),
        "android" => (
            "bash",
            vec!["./gradlew", "assembleRelease"],
            project_root.join("android"),
        ),
        "frontend" => (
            "bash",
            vec!["-c", "npm run build"],
            project_root.join("frontend"),
        ),
        _ => return Err(anyhow!("未知构建目标: {}，支持: rust/android/frontend", target)),
    };

    if !work_dir.exists() {
        return Err(anyhow!("目录不存在: {}，该模块尚未创建", work_dir.display()));
    }

    let output = Command::new(cmd)
        .args(&args)
        .current_dir(&work_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(anyhow!("构建失败:\nstdout: {}\nstderr: {}", stdout, stderr));
    }
    Ok(format!("{} 构建成功\n{}", target, stdout.trim()))
}

/// 执行受限 shell 命令（只允许白名单命令）
pub fn run_shell(project_root: &Path, command: &str) -> Result<String> {
    // 白名单：只允许安全的只读或构建相关命令
    const ALLOWED_PREFIXES: &[&str] = &[
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
    ];

    let is_allowed = ALLOWED_PREFIXES.iter().any(|prefix| command.starts_with(prefix));
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
    Ok(format!("{}\n{}", stdout.trim(), stderr.trim()).trim().to_string())
}

/// 安全路径：确保路径不会逃出 project_root
fn safe_path(project_root: &Path, relative_path: &str) -> Result<std::path::PathBuf> {
    // 拒绝包含 .. 的路径
    if relative_path.contains("..") {
        return Err(anyhow!("路径不允许包含 '..': {}", relative_path));
    }
    let full = project_root.join(relative_path);
    // 规范化后再检查是否仍在 project_root 内
    let canonical_root = project_root.canonicalize().unwrap_or(project_root.to_path_buf());
    // 注意：文件可能还不存在，无法 canonicalize，用前缀检查
    let full_str = full.to_string_lossy();
    let root_str = canonical_root.to_string_lossy();
    if !full_str.starts_with(root_str.as_ref()) {
        return Err(anyhow!("路径越界: {}", relative_path));
    }
    Ok(full)
}
