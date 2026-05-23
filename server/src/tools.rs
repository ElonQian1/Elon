use anyhow::{anyhow, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
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
    Ok(format!("git commit 成功: {}", stdout.trim()))
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
                "android 构建成功\n##APK_FILE:{}\n\n构建日志:\n{}",
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

pub const STABLE_APK_FILENAME: &str = "latest.apk";

pub fn stable_apk_url(download_base: &str) -> String {
    format!(
        "{}/{}",
        download_base.trim_end_matches('/'),
        STABLE_APK_FILENAME
    )
}

pub fn find_latest_apk(work_dir: &Path) -> Option<std::path::PathBuf> {
    let matches = collect_apks(work_dir);
    matches.into_iter().max_by_key(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    })
}

pub fn find_apk_by_filename(work_dir: &Path, filename: &str) -> Option<std::path::PathBuf> {
    collect_apks(work_dir)
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(filename))
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
}

pub fn find_download_apk(work_dir: &Path, filename: &str) -> Option<std::path::PathBuf> {
    if filename == STABLE_APK_FILENAME {
        find_latest_apk(work_dir)
    } else {
        find_apk_by_filename(work_dir, filename)
    }
}

fn collect_apks(work_dir: &Path) -> Vec<std::path::PathBuf> {
    let dirs = [
        "app/build/outputs/apk",
        "android/app/build/outputs/apk",
        "build",
        "artifacts",
    ];
    let mut matches = Vec::new();
    for rel in &dirs {
        collect_apks_from_dir(&work_dir.join(rel), &mut matches);
    }
    matches
}

fn collect_apks_from_dir(dir: &Path, matches: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_apks_from_dir(&path, matches);
        } else if path.extension().and_then(|e| e.to_str()) == Some("apk") {
            matches.push(path);
        }
    }
}

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
