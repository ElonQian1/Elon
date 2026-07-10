use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::node_agent_android_inspector::{
    adb_capture::{capture_screen_png, launch_app},
    adb_command::{run_adb_text, validate_device_id},
    png_probe::png_dimensions,
};

use super::adb_session::start_runtime;
use super::broker::{LiveUiBroker, LiveUiSession};
use super::preview::{open_preview, PreviewOpenRequest};
use super::ui_ir::load_or_build_ui_ir;
use super::visual_diff::{compare_target_with_png, VisualDiffResult};

const BUILD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_BUILD_OUTPUT: usize = 256 * 1024;
const MAX_SCAN_FILES: usize = 40_000;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyRequest {
    pub(crate) preview: Option<PreviewOpenRequest>,
    pub(crate) debug_application_id_suffix: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyResult {
    status: &'static str,
    apk_path: String,
    build_duration_ms: u128,
    install_output: String,
    runtime_connected: bool,
    runtime_build_id: Option<String>,
    node_count: usize,
    screenshot_width: u32,
    screenshot_height: u32,
    visual_diff: Option<VisualDiffResult>,
    message: String,
}

pub(crate) async fn build_and_verify(
    broker: &LiveUiBroker,
    session_id: &str,
    request: BuildVerifyRequest,
    host_port: u16,
) -> Result<BuildVerifyResult> {
    let session = broker.session(session_id).await?;
    let target_path = load_or_build_ui_ir(broker, session_id)
        .await
        .ok()
        .and_then(|ir| ir.target_design.map(|target| target.path));
    let project_root = canonical_project_root(&session)?;
    let gradle_root = find_gradle_root(&project_root)?;
    let wrapper = gradle_wrapper(&gradle_root)?;

    let build_started = Instant::now();
    let debug_application_id_suffix = request
        .debug_application_id_suffix
        .as_deref()
        .map(validate_debug_application_id_suffix)
        .transpose()?;
    run_debug_build(&gradle_root, &wrapper, debug_application_id_suffix).await?;
    let build_duration_ms = build_started.elapsed().as_millis();
    let apk = newest_debug_apk(&gradle_root)?;

    validate_device_id(&session.device_id)?;
    let install_output = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "install".to_string(),
            "-r".to_string(),
            "-t".to_string(),
            apk.display().to_string(),
        ],
        Duration::from_secs(180),
        256 * 1024,
    )
    .await?;
    if !install_output.to_ascii_lowercase().contains("success") {
        bail!("Debug APK 安装未返回 Success: {}", install_output.trim());
    }

    session.reset_for_redeploy().await;
    launch_app(&session.device_id, &session.package_name).await?;
    start_runtime(&session, host_port).await?;
    if let Some(preview) = request.preview {
        open_preview(&session, preview).await?;
    }
    let runtime_view = wait_for_runtime(&session).await?;
    tokio::time::sleep(Duration::from_millis(650)).await;
    let screenshot = capture_screen_png(&session.device_id).await?;
    let (screenshot_width, screenshot_height) = png_dimensions(&screenshot)?;
    let visual_diff = target_path
        .as_deref()
        .map(|path| compare_target_with_png(path, &screenshot, None, None))
        .transpose()?;

    Ok(BuildVerifyResult {
        status: "BUILD_VERIFIED",
        apk_path: apk.display().to_string(),
        build_duration_ms,
        install_output: install_output.trim().to_string(),
        runtime_connected: runtime_view.connected,
        runtime_build_id: runtime_view.runtime_build_id,
        node_count: runtime_view.node_count,
        screenshot_width,
        screenshot_height,
        visual_diff,
        message: "已由源码重新构建并安装；临时 Patch 已清空，当前画面来自新 Debug APK。"
            .to_string(),
    })
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("Live 会话未绑定 projectRoot"))?;
    PathBuf::from(root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {root}"))
}

fn find_gradle_root(project_root: &Path) -> Result<PathBuf> {
    for candidate in [project_root.to_path_buf(), project_root.join("android")] {
        if candidate.join("gradlew").is_file() || candidate.join("gradlew.bat").is_file() {
            return candidate
                .canonicalize()
                .with_context(|| format!("Gradle 目录不可访问: {}", candidate.display()));
        }
    }
    bail!("项目根目录及 android/ 下均未找到 Gradle Wrapper")
}

fn gradle_wrapper(gradle_root: &Path) -> Result<PathBuf> {
    let candidates = if cfg!(windows) {
        [gradle_root.join("gradlew.bat"), gradle_root.join("gradlew")]
    } else {
        [gradle_root.join("gradlew"), gradle_root.join("gradlew.bat")]
    };
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow!("Gradle Wrapper 不存在"))
}

async fn run_debug_build(
    gradle_root: &Path,
    wrapper: &Path,
    debug_application_id_suffix: Option<&str>,
) -> Result<()> {
    let mut command =
        if cfg!(windows) && wrapper.extension().and_then(|v| v.to_str()) == Some("bat") {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/C"]).arg(wrapper);
            command
        } else {
            Command::new(wrapper)
        };
    command
        .current_dir(gradle_root)
        .args(["assembleDebug", "--no-daemon"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(suffix) = debug_application_id_suffix {
        command.arg(format!("-PELON_DEBUG_APPLICATION_ID_SUFFIX={suffix}"));
    }
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    let output = tokio::time::timeout(BUILD_TIMEOUT, command.output())
        .await
        .context("Android Debug 构建超时")?
        .context("无法启动 Gradle Wrapper")?;
    if output.stdout.len() + output.stderr.len() > MAX_BUILD_OUTPUT * 4 {
        bail!("Gradle 输出异常过大，已停止验收");
    }
    if !output.status.success() {
        let message = tail_output(&output.stdout, &output.stderr);
        bail!("Android Debug 构建失败: {message}");
    }
    Ok(())
}

fn validate_debug_application_id_suffix(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.len() > 40
        || !value.starts_with('.')
        || value == "."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
    {
        bail!("debugApplicationIdSuffix 仅允许以点开头的字母、数字、点和下划线，长度不超过 40")
    }
    Ok(value)
}

fn newest_debug_apk(gradle_root: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    let mut visited = 0;
    collect_debug_apks(gradle_root, 0, &mut visited, &mut candidates)?;
    candidates
        .into_iter()
        .max_by_key(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
        .ok_or_else(|| anyhow!("assembleDebug 完成后没有找到 *-debug.apk"))
}

fn collect_debug_apks(
    dir: &Path,
    depth: usize,
    visited: &mut usize,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > 10 || *visited > MAX_SCAN_FILES {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).with_context(|| format!("读取构建目录失败: {}", dir.display()))?
    {
        let entry = entry?;
        *visited += 1;
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(".git" | ".gradle" | "node_modules")
            ) {
                collect_debug_apks(&path, depth + 1, visited, output)?;
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("-debug.apk"))
            .unwrap_or(false)
            && path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("/build/outputs/apk/")
        {
            output.push(path);
        }
        if *visited > MAX_SCAN_FILES {
            break;
        }
    }
    Ok(())
}

async fn wait_for_runtime(session: &LiveUiSession) -> Result<super::protocol::LiveSessionView> {
    let started = Instant::now();
    loop {
        let view = session.view().await;
        if view.connected && view.runtime_build_id.is_some() {
            return Ok(view);
        }
        if started.elapsed() > Duration::from_secs(15) {
            bail!("新 APK 已安装，但 Debug Runtime 在 15 秒内没有重新连接");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn tail_output(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr),
    );
    let start = combined.len().saturating_sub(MAX_BUILD_OUTPUT);
    combined
        .get(start..)
        .unwrap_or(&combined)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_debug_application_id_suffix() {
        assert_eq!(
            validate_debug_application_id_suffix(".uitest_2").unwrap(),
            ".uitest_2"
        );
    }

    #[test]
    fn rejects_gradle_argument_injection() {
        assert!(validate_debug_application_id_suffix(".uitest -Pbad=true").is_err());
        assert!(validate_debug_application_id_suffix("uitest").is_err());
        assert!(validate_debug_application_id_suffix(".").is_err());
    }

    #[test]
    fn locates_android_gradle_root_without_leaving_project() {
        let root = std::env::temp_dir().join(format!("elon-build-verify-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("android")).unwrap();
        fs::write(root.join("android/gradlew.bat"), "@echo off").unwrap();
        assert_eq!(
            find_gradle_root(&root).unwrap(),
            root.join("android").canonicalize().unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
