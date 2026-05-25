//! Git 相关工具（commit/push、pull --rebase、版本号递增、初始化仓库）。
//! 从 `tools.rs` 中抽出，调用者通过 `tools::git_commit` 等 re-export 使用。

use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

/// 执行 git commit
pub fn git_commit(project_root: &Path, message: &str) -> Result<String> {
    info!("[工具] git commit: {}", message);
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(project_root)
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "git add 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
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
    // 尝试推送到远程（用户隔离工作区可能无远程，失败时非致命）。
    // 会话 worktree 会运行在自己的分支上，这里必须推当前分支而不是硬编码 main。
    let branch = current_branch(project_root).unwrap_or_else(|| "main".into());
    if !has_origin_remote(project_root) {
        return Ok(format!("git commit 成功: {}", stdout.trim()));
    }
    let push_output = Command::new("git")
        .args(["push", "origin", &branch])
        .current_dir(project_root)
        .output();
    let push_note = match push_output {
        Ok(out) if out.status.success() => format!(" 并已推送到 origin/{}", branch),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            warn!("[工具] git push 失败（非致命）: {}", err.trim());
            " (无远程或 push 失败，仅本地提交)".to_string()
        }
        Err(e) => {
            warn!("[工具] git push 命令出错: {}", e);
            " (push 命令出错，仅本地提交)".to_string()
        }
    };
    Ok(format!("git commit 成功: {}{}", stdout.trim(), push_note))
}

fn has_origin_remote(project_root: &Path) -> bool {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_root)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn current_branch(project_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

/// git pull --rebase origin main（同步最新代码，非致命）
pub fn git_pull_rebase(project_root: &Path) -> Result<String> {
    info!("[工具] git pull --rebase origin main");
    let output = Command::new("git")
        .args(["pull", "--rebase", "origin", "main"])
        .current_dir(project_root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        warn!("[工具] git pull --rebase 失败（非致命）: {}", stderr.trim());
        return Ok(format!(
            "git pull 未成功（{}），在本地代码继续操作",
            stderr.trim()
        ));
    }
    Ok(format!("已同步最新代码: {}", stdout.trim()))
}

/// Android build.gradle 版本号自动递增（versionCode +1，versionName PATCH +1）
pub(crate) fn bump_android_version(work_dir: &Path) -> String {
    let gradle_path = work_dir.join("app").join("build.gradle");
    if !gradle_path.exists() {
        return "（未找到 app/build.gradle，跳过版本号递增）".into();
    }
    // 幂等性守卫：5 分钟内如果已递增过版本号，跳过（防止服务器重启后重复递增）
    let bump_marker = work_dir.join(".version_bumped_at");
    if let Ok(meta) = std::fs::metadata(&bump_marker) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().unwrap_or_default() < std::time::Duration::from_secs(300) {
                return "(5分钟内已递增过版本号，跳过重复递增)".into();
            }
        }
    }
    let content = match std::fs::read_to_string(&gradle_path) {
        Ok(c) => c,
        Err(e) => return format!("（读取 build.gradle 失败: {}）", e),
    };
    let mut new_lines: Vec<String> = Vec::new();
    let mut version_code_old = 0u32;
    let mut version_code_new = 0u32;
    let mut version_name_old = String::new();
    let mut version_name_new = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("versionCode ") {
            if let Some(num_str) = trimmed.split_whitespace().nth(1) {
                if let Ok(n) = num_str.parse::<u32>() {
                    version_code_old = n;
                    version_code_new = n + 1;
                    new_lines.push(line.replacen(
                        &format!("versionCode {}", n),
                        &format!("versionCode {}", version_code_new),
                        1,
                    ));
                    continue;
                }
            }
        } else if trimmed.starts_with("versionName ") {
            if let Some(s) = trimmed.find('"') {
                if let Some(e) = trimmed[s + 1..].find('"') {
                    let ver = &trimmed[s + 1..s + 1 + e];
                    let parts: Vec<&str> = ver.split('.').collect();
                    if parts.len() == 3 {
                        if let (Ok(maj), Ok(min), Ok(pat)) = (
                            parts[0].parse::<u32>(),
                            parts[1].parse::<u32>(),
                            parts[2].parse::<u32>(),
                        ) {
                            version_name_old = ver.to_string();
                            version_name_new = format!("{}.{}.{}", maj, min, pat + 1);
                            new_lines.push(line.replacen(
                                &format!("versionName \"{}\"", version_name_old),
                                &format!("versionName \"{}\"", version_name_new),
                                1,
                            ));
                            continue;
                        }
                    }
                }
            }
        }
        new_lines.push(line.to_string());
    }

    let mut new_content = new_lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }
    if version_code_new == 0 {
        return "（未找到 versionCode，跳过版本号递增）".into();
    }
    match std::fs::write(&gradle_path, &new_content) {
        Ok(_) => {
            // 写入成功后更新幂等性标记文件
            let _ = std::fs::write(&bump_marker, "");
            if version_name_new.is_empty() {
                format!(
                    "版本号已递增: versionCode {} → {}",
                    version_code_old, version_code_new
                )
            } else {
                format!(
                    "版本号已递增: versionCode {} → {}，versionName {} → {}",
                    version_code_old, version_code_new, version_name_old, version_name_new
                )
            }
        }
        Err(e) => format!("（写入 build.gradle 失败: {}）", e),
    }
}

pub(crate) fn ensure_git_repo(project_root: &Path, user_id: &str) -> Result<()> {
    if !project_root.join(".git").exists() {
        let output = Command::new("git")
            .args(["init"])
            .current_dir(project_root)
            .output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "git init 失败: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    let _ = Command::new("git")
        .args(["config", "user.email", &format!("{}@elon.app", user_id)])
        .current_dir(project_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", user_id])
        .current_dir(project_root)
        .output();
    Ok(())
}
