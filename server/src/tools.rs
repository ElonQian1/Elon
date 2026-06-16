use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{info, warn};

use crate::project_default_docs::ensure_default_docs_in_workspace;

pub use crate::tools_git::{git_commit, git_fetch_status};

/// 每用户并发构建槽（同一用户同时只允许一个本地构建）
static BUILD_SLOTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn build_slots() -> &'static Mutex<HashSet<String>> {
    BUILD_SLOTS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// RAII 守卫：Drop 时自动释放构建槽
struct BuildSlotGuard(String);

impl Drop for BuildSlotGuard {
    fn drop(&mut self) {
        if let Ok(mut slots) = build_slots().lock() {
            slots.remove(&self.0);
        }
    }
}

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
    if should_normalize_written_file(&full_path, content) {
        normalize_text_file_line_endings_if_needed(&full_path)?;
    }
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
                    let pkg = derive_android_package_id(project_root);
                    patch_android_application_id(project_root, &pkg)?;
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

    ensure_default_docs_in_workspace(project_root)?;
    ensure_git_repo(project_root, user_id)?;
    let _ = git_commit(project_root, "chore: initialize project")?;
    Ok(format!(
        "项目工作区已创建: {}",
        project_root.to_string_lossy()
    ))
}

/// 构建项目（自动检测项目类型，从工作区根目录开始）
/// target: "android" | "rust" | "frontend"
/// user_id: 调用用户 ID，用于并发构建保护
/// 成功时返回值中若包含 ##APK_FILE:<name>，表示有 APK 可供下载
pub fn build_project(project_root: &Path, target: &str, user_id: &str) -> Result<String> {
    info!("[工具] 构建项目: {}", target);

    // 每用户并发构建保护：同一用户同时只允许一个本地构建
    {
        let mut slots = build_slots().lock().unwrap_or_else(|e| e.into_inner());
        if slots.contains(user_id) {
            return Err(anyhow!("您已有一个构建任务正在进行，请等待完成后再试"));
        }
        slots.insert(user_id.to_string());
    }
    let _slot_guard = BuildSlotGuard(user_id.to_string());

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

    // 构建前顺手归一化可执行脚本的行尾，避免 shebang 读取到 \r。
    normalize_project_scripts_for_shell(&work_dir)?;

    // 启动子进程并设置 10 分钟超时防护
    let mut child = Command::new("bash")
        .args(["-c", cmd])
        .current_dir(&work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    const BUILD_TIMEOUT_SECS: u64 = 600;
    let start = std::time::Instant::now();
    let output = loop {
        if start.elapsed().as_secs() > BUILD_TIMEOUT_SECS {
            let _ = child.kill();
            return Err(anyhow!(
                "构建超时（超过 {} 秒），已终止进程",
                BUILD_TIMEOUT_SECS
            ));
        }
        match child.try_wait()? {
            Some(_) => break child.wait_with_output()?,
            None => std::thread::sleep(std::time::Duration::from_millis(500)),
        }
    };

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        let elapsed = start.elapsed().as_secs();
        tracing::info!(
            target: "build_metrics",
            user_id = user_id,
            target = target,
            duration_secs = elapsed,
            success = false,
            "build_failed"
        );
        return Err(anyhow!(
            "构建失败:\n{}",
            &combined[..combined.len().min(2000)]
        ));
    }

    let elapsed = start.elapsed().as_secs();

    // Android 构建成功后找到 APK 文件，嵌入特殊标记
    if target == "android" {
        if let Some(apk_path) = find_latest_apk(&work_dir) {
            let apk_name = apk_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            tracing::info!(
                target: "build_metrics",
                user_id = user_id,
                target = target,
                duration_secs = elapsed,
                success = true,
                "build_complete"
            );
            return Ok(format!(
                "android 构建成功\n{}\n##APK_FILE:{}\n\n构建日志:\n{}",
                version_note,
                apk_name,
                &combined[..combined.len().min(800)]
            ));
        }
    }

    tracing::info!(
        target: "build_metrics",
        user_id = user_id,
        target = target,
        duration_secs = elapsed,
        success = true,
        "build_complete"
    );

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
            let pkg = derive_android_package_id(project_root);
            patch_android_application_id(project_root, &pkg)?;
            ensure_default_docs_in_workspace(project_root)?;
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

/// 从项目工作区路径派生唯一的 Android applicationId。
/// 例：project_root 最后一段 "prj_8f3a92c1" → "com.elon.prj8f3a92c1"
fn derive_android_package_id(project_root: &Path) -> String {
    let raw = project_root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    // Android 包名每段必须以字母开头
    let segment = if raw.starts_with(|c: char| c.is_ascii_alphabetic()) {
        raw
    } else {
        format!("p{}", raw)
    };
    format!("com.elon.{}", segment)
}

/// 把 Android 模板里的占位 applicationId/namespace 替换为项目专属包名。
/// 只修改 app/build.gradle，源码目录结构保持不变（applicationId 与源码包名可以不同）。
fn patch_android_application_id(project_root: &Path, package_id: &str) -> Result<()> {
    let build_gradle = project_root.join("app").join("build.gradle");
    if !build_gradle.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&build_gradle)?;
    let patched = content
        .replace(
            "applicationId \"com.template.app\"",
            &format!("applicationId \"{}\"", package_id),
        )
        .replace(
            "namespace 'com.template.app'",
            &format!("namespace '{}'", package_id),
        )
        .replace(
            "namespace \"com.template.app\"",
            &format!("namespace \"{}\"", package_id),
        );
    if patched != content {
        std::fs::write(&build_gradle, patched)?;
        info!("[工具] 已设置 applicationId: {}", package_id);
    }
    Ok(())
}

pub use crate::tools_apk::{
    find_apk_by_filename, find_download_apk, find_latest_apk, stable_apk_url, STABLE_APK_FILENAME,
};
pub use crate::tools_exec::{build_project_via_agent, elon_pc_project_path, exec_via_agent};
pub use crate::tools_patch::apply_patch;
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

    // 命令执行前，自动归一化命令里引用到的脚本文件行尾。
    normalize_referenced_scripts(project_root, command)?;

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
///
/// 修复以下攻击向量：
/// 1. 路径前缀混淆：字符串前缀检查会让 /proj1 错误匹配 /proj10
/// 2. 绝对路径注入：Path::join 对绝对路径会替换整个 base
/// 3. 符号链接逃逸：通过 symlink 跳出工作区
fn safe_path(project_root: &Path, relative_path: &str) -> Result<PathBuf> {
    // 1. 拒绝包含 .. 的路径（防相对遍历）
    if relative_path.contains("..") {
        return Err(anyhow!("路径不允许包含 '..': {}", relative_path));
    }
    // 2. 拒绝绝对路径注入（Unix: 以 / 开头；Windows: drive letter）
    if Path::new(relative_path).is_absolute() {
        return Err(anyhow!("路径不允许为绝对路径: {}", relative_path));
    }

    let canonical_root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let full = canonical_root.join(relative_path);

    // 3. 解析符号链接：对已存在的路径直接 canonicalize；
    //    对尚未存在的路径（写入新文件），沿祖先链找到最近存在的目录 canonicalize，
    //    再拼接剩余路径段，避免符号链接逃逸。
    let effective = if full.exists() {
        full.canonicalize()
            .map_err(|e| anyhow!("路径规范化失败: {}", e))?
    } else {
        resolve_nonexistent_path(&full)?
    };

    // 4. 使用 Path::starts_with 逐组件比较（不是字符串比较，避免前缀混淆）
    if !effective.starts_with(&canonical_root) {
        return Err(anyhow!("路径越界: {}", relative_path));
    }
    Ok(full)
}

fn normalize_referenced_scripts(project_root: &Path, command: &str) -> Result<()> {
    for token in command.split_whitespace() {
        let candidate = token
            .trim_matches(|c: char| matches!(c, '\'' | '"' | '`' | ';' | '&' | '|' | '(' | ')'))
            .trim_start_matches("./")
            .trim_start_matches(".\\");

        if candidate.is_empty() || candidate.starts_with('-') {
            continue;
        }

        let path = project_root.join(candidate);
        if is_script_file(&path) && path.exists() {
            let _ = normalize_text_file_line_endings_if_needed(&path)?;
        }
    }
    Ok(())
}

fn normalize_project_scripts_for_shell(project_root: &Path) -> Result<()> {
    normalize_project_scripts_recursive(project_root)?;
    Ok(())
}

fn normalize_project_scripts_recursive(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            normalize_project_scripts_recursive(&entry.path())?;
        }
        return Ok(());
    }

    if is_script_file(path) {
        let _ = normalize_text_file_line_endings_if_needed(path)?;
    }
    Ok(())
}

fn is_script_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|v| v.to_str()) {
        Some(name) => name,
        None => return false,
    };
    if file_name == "gradlew" || file_name == "bash" {
        return true;
    }

    matches!(
        path.extension().and_then(|v| v.to_str()).map(|v| v.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "py" | "sh" | "bash")
    )
}

fn should_normalize_written_file(path: &Path, content: &str) -> bool {
    if is_script_file(path) || path.file_name().and_then(|v| v.to_str()) == Some("gradlew") {
        return true;
    }

    content.starts_with("#!")
}

fn normalize_text_file_line_endings_if_needed(path: &Path) -> Result<bool> {
    let content = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Ok(false),
    };

    let normalized = content.replace("\r\n", "\n");
    if normalized == content {
        return Ok(false);
    }

    std::fs::write(path, normalized)?;
    Ok(true)
}

/// 对尚不存在的路径，沿祖先链找到最近存在的目录 canonicalize，再拼接剩余路径段
fn resolve_nonexistent_path(path: &Path) -> Result<PathBuf> {
    let mut ancestor: &Path = path;
    let mut tail: Vec<std::ffi::OsString> = vec![];
    loop {
        let parent = ancestor
            .parent()
            .ok_or_else(|| anyhow!("路径无效，无法找到存在的祖先目录"))?;
        if let Some(name) = ancestor.file_name() {
            tail.push(name.to_owned());
        }
        if parent.exists() {
            let mut resolved = parent
                .canonicalize()
                .map_err(|e| anyhow!("路径规范化失败: {}", e))?;
            for component in tail.iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        ancestor = parent;
    }
}
