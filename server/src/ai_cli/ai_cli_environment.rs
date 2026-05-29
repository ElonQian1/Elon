use anyhow::{anyhow, Result};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use crate::types::AiCliOption;

pub(crate) fn ensure_git(
    workspace: &Path,
    user_id: &str,
    require_existing_git: bool,
) -> Result<()> {
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

pub(crate) fn environment_notes(user_message: &str, option: &AiCliOption) -> Vec<String> {
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

pub(crate) fn looks_like_android_task(user_message: &str) -> bool {
    let lower = user_message.to_ascii_lowercase();
    lower.contains("apk")
        || lower.contains("android")
        || user_message.contains("安卓")
        || user_message.contains("应用")
        || user_message.contains("打包")
        || user_message.contains("编译")
}
