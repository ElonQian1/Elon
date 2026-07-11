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

use super::adb_session::{start_runtime, stop_runtime, DEFAULT_DEVICE_PORT};
use super::broker::{LiveCommitSnapshot, LiveUiBroker, LiveUiSession};
use super::build_verify_apk::select_fresh_debug_apk;
use super::preview::{open_preview, PreviewOpenRequest};
use super::protocol::{LiveStylePatch, LiveUiNode};
use super::ui_ir::load_or_build_ui_ir;
use super::verification_gate::{
    evaluate_verification_gates, VerificationGateInput, VerificationGateResult,
    VerificationGateState,
};
use super::visual_diff::{
    compare_pngs, compare_target_with_png_projected, PixelRect, VisualDiffResult,
};

const BUILD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_BUILD_OUTPUT: usize = 256 * 1024;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyRequest {
    pub(crate) preview: Option<PreviewOpenRequest>,
    pub(crate) debug_application_id_suffix: Option<String>,
    pub(crate) target_rect: Option<PixelRect>,
    pub(crate) current_rect: Option<PixelRect>,
    pub(crate) projected_current_rect: Option<PixelRect>,
    pub(crate) target_definition_id: Option<String>,
    pub(crate) target_instance_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyResult {
    pub(crate) status: &'static str,
    pub(crate) apk_path: String,
    pub(crate) build_duration_ms: u128,
    pub(crate) install_output: String,
    pub(crate) runtime_connected: bool,
    pub(crate) runtime_build_id: Option<String>,
    pub(crate) node_count: usize,
    pub(crate) screenshot_width: u32,
    pub(crate) screenshot_height: u32,
    pub(crate) visual_diff: Option<VisualDiffResult>,
    pub(crate) source_parity_diff: VisualDiffResult,
    pub(crate) source_parity_verified: bool,
    pub(crate) verification_gate: VerificationGateResult,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareDebugRuntimeRequest {
    pub(crate) device_id: String,
    pub(crate) base_package_name: String,
    pub(crate) project_root: String,
    pub(crate) debug_application_id_suffix: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareDebugRuntimeResult {
    pub(crate) package_name: String,
    pub(crate) build: BuildVerifyResult,
}

/// Builds and installs a side-by-side Debug APK before the normal Live session
/// exists. The temporary session is deliberately removed afterwards: the PC
/// page captures the newly installed package and then creates the user-owned
/// Live session with a fresh token.
pub(crate) async fn prepare_debug_runtime(
    broker: &LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    host_port: u16,
) -> Result<PrepareDebugRuntimeResult> {
    let device_id = request.device_id.trim();
    let base_package_name = validate_package_name(request.base_package_name.trim())?;
    let project_root = request.project_root.trim();
    let suffix = validate_debug_application_id_suffix(request.debug_application_id_suffix.trim())?;
    if device_id.is_empty() {
        bail!("deviceId 不能为空");
    }
    if project_root.is_empty() {
        bail!("projectRoot 不能为空；请先在 PC 工作台选择本机项目");
    }
    let package_name = format!("{base_package_name}{suffix}");
    let session = broker
        .create_session(
            device_id.to_string(),
            package_name.clone(),
            Some(project_root.to_string()),
            DEFAULT_DEVICE_PORT,
        )
        .await;
    let session_id = session.id.clone();
    let result = build_and_verify(
        broker,
        &session_id,
        BuildVerifyRequest {
            preview: None,
            debug_application_id_suffix: Some(suffix.to_string()),
            ..BuildVerifyRequest::default()
        },
        host_port,
    )
    .await;
    let _ = stop_runtime(&session).await;
    broker.remove_session(&session_id).await;
    result.map(|build| PrepareDebugRuntimeResult {
        package_name,
        build,
    })
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
    let live_snapshot = session.commit_snapshot().await;
    let live_preview_png = capture_screen_png(&session.device_id).await?;
    let live_preview_rect = verification_bounds(
        &live_snapshot.nodes,
        request.target_definition_id.as_deref(),
        request.target_instance_key.as_deref(),
    )?
    .or_else(|| patched_bounds(&live_snapshot, true));

    let build_started = Instant::now();
    let artifact_not_before = SystemTime::now()
        .checked_sub(Duration::from_secs(2))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let debug_application_id_suffix = request
        .debug_application_id_suffix
        .as_deref()
        .map(validate_debug_application_id_suffix)
        .transpose()?;
    run_debug_build(&gradle_root, &wrapper, debug_application_id_suffix).await?;
    let build_duration_ms = build_started.elapsed().as_millis();
    let apk = select_fresh_debug_apk(&gradle_root, &session.package_name, artifact_not_before)?;

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
    tokio::time::sleep(Duration::from_millis(650)).await;
    start_runtime(&session, host_port).await?;
    // A connected socket with an empty tree is not a verified UI. Always wait
    // for at least one runtime node, including normal Activity verification.
    let preview = request.preview.clone();
    if let Some(preview) = preview.as_ref() {
        open_preview(&session, preview.clone()).await?;
    }
    let expected_screen_id = preview.as_ref().map(|preview| preview.screen_id.as_str());
    let runtime_view =
        match wait_for_runtime(broker, session_id, &session, expected_screen_id).await {
            Ok(view) => view,
            Err(first_error) => {
                // Some vendor systems finish `adb install -r` before the replaced
                // process and its debug receiver are fully ready. Re-launch and
                // bootstrap once more instead of leaving the PC page in a false
                // disconnected state after a successful install.
                launch_app(&session.device_id, &session.package_name).await?;
                tokio::time::sleep(Duration::from_millis(650)).await;
                start_runtime(&session, host_port).await?;
                if let Some(preview) = preview.as_ref() {
                    open_preview(&session, preview.clone()).await?;
                }
                wait_for_runtime(broker, session_id, &session, expected_screen_id)
                    .await
                    .with_context(|| {
                        format!("重新连接 Debug Runtime 失败；首次错误: {first_error:#}")
                    })?
            }
        };
    // The Activity can expose its first View tree before async data binding and
    // entrance rendering finish. Comparing that intermediate frame against a
    // stable Live preview creates false BUILD_MISMATCH results. Give the debug
    // renderer a deterministic settle window, then require consecutive frames.
    tokio::time::sleep(Duration::from_millis(2_200)).await;
    let screenshot = capture_stable_screen(&session.device_id).await?;
    let (screenshot_width, screenshot_height) = png_dimensions(&screenshot)?;
    let (_, redeployed_nodes) = broker.tree(session_id).await?;
    let source_rect = verification_bounds(
        &redeployed_nodes,
        request.target_definition_id.as_deref(),
        request.target_instance_key.as_deref(),
    )?
    .or_else(|| patched_bounds_for_nodes(&redeployed_nodes, &live_snapshot.patches, false));
    // 重新部署后节点几何可能已经改变。目标门禁必须裁剪纯源码版本的
    // 实际 Runtime bounds，而不是继续沿用设计会话开始时的旧 currentRect。
    let verified_current_rect = source_rect.or(request.current_rect);
    let visual_diff = target_path
        .as_deref()
        .map(|path| {
            compare_target_with_png_projected(
                path,
                &screenshot,
                request.target_rect,
                verified_current_rect,
                request.projected_current_rect,
            )
        })
        .transpose()?;
    let source_parity_diff = compare_pngs(
        &live_preview_png,
        &screenshot,
        live_preview_rect,
        source_rect,
    )?;
    let verification_gate = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source_parity_diff),
        visual_diff.as_ref(),
        target_path.is_some(),
    ));
    let source_parity_verified = verification_gate.source_parity == VerificationGateState::Passed;
    let status = verification_gate.status;
    let message = match status {
        "BUILD_VERIFIED" => {
            "已由源码重新构建并安装；临时 Patch 已清空，源码一致性和目标设计门禁均通过。"
        }
        "TARGET_MISMATCH" => "源码结果已与 Live 预览一致，但尚未达到目标设计门禁。",
        "TARGET_NOT_CONFIGURED" => "源码结果已与 Live 预览一致，但设计拟合任务尚未配置目标配对。",
        _ => "源码已构建安装，但纯源码画面与 Live 预览不一致；本次不能标记完成。",
    }
    .to_string();

    Ok(BuildVerifyResult {
        status,
        apk_path: apk.display().to_string(),
        build_duration_ms,
        install_output: install_output.trim().to_string(),
        runtime_connected: runtime_view.connected,
        runtime_build_id: runtime_view.runtime_build_id,
        node_count: runtime_view.node_count,
        screenshot_width,
        screenshot_height,
        visual_diff,
        source_parity_diff,
        source_parity_verified,
        verification_gate,
        message,
    })
}

async fn capture_stable_screen(device_id: &str) -> Result<Vec<u8>> {
    let mut previous = capture_screen_png(device_id).await?;
    let mut stable_frames = 0_u8;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(350)).await;
        let current = capture_screen_png(device_id).await?;
        let diff = compare_pngs(&previous, &current, None, None)?;
        if diff.visual_loss <= 0.012 {
            stable_frames += 1;
            if stable_frames >= 2 {
                return Ok(current);
            }
        } else {
            stable_frames = 0;
        }
        previous = current;
    }
    Ok(previous)
}

fn patched_bounds(snapshot: &LiveCommitSnapshot, allow_runtime_id: bool) -> Option<PixelRect> {
    patched_bounds_for_nodes(&snapshot.nodes, &snapshot.patches, allow_runtime_id)
}

fn verification_bounds(
    nodes: &[LiveUiNode],
    definition_id: Option<&str>,
    instance_key: Option<&str>,
) -> Result<Option<PixelRect>> {
    let Some(definition_id) = definition_id else {
        return Ok(None);
    };
    let candidates = nodes
        .iter()
        .filter_map(|node| {
            if node.definition_id != definition_id
                || instance_key.is_some_and(|key| node.instance_key.as_deref() != Some(key))
                || !node.geometry.visible
            {
                return None;
            }
            let bounds = &node.geometry.bounds_in_display_px;
            (bounds.right > bounds.left && bounds.bottom > bounds.top).then_some(PixelRect {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            })
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [bounds] => Ok(Some(*bounds)),
        [] => bail!(
            "构建验收找不到目标节点 definitionId={definition_id} instanceKey={:?}",
            instance_key
        ),
        _ => bail!("构建验收目标节点不唯一 definitionId={definition_id}；请提供稳定 instanceKey"),
    }
}

fn patched_bounds_for_nodes(
    nodes: &[LiveUiNode],
    patches: &[LiveStylePatch],
    allow_runtime_id: bool,
) -> Option<PixelRect> {
    let mut result: Option<PixelRect> = None;
    for patch in patches {
        let node = allow_runtime_id
            .then(|| patch.target.runtime_node_id.as_deref())
            .flatten()
            .and_then(|id| nodes.iter().find(|node| node.runtime_node_id == id))
            .or_else(|| {
                patch
                    .target
                    .definition_id
                    .as_deref()
                    .and_then(|definition| {
                        nodes.iter().find(|node| node.definition_id == definition)
                    })
            });
        let Some(node) = node.filter(|node| node.geometry.visible) else {
            continue;
        };
        let bounds = &node.geometry.bounds_in_display_px;
        if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
            continue;
        }
        result = Some(match result {
            Some(current) => PixelRect {
                left: current.left.min(bounds.left),
                top: current.top.min(bounds.top),
                right: current.right.max(bounds.right),
                bottom: current.bottom.max(bounds.bottom),
            },
            None => PixelRect {
                left: bounds.left,
                top: bounds.top,
                right: bounds.right,
                bottom: bounds.bottom,
            },
        });
    }
    result
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
            // cmd.exe applies special quote stripping after /C. Passing an absolute
            // \\?\ path containing spaces can therefore be truncated before Gradle is
            // launched. The command already runs in gradle_root, so invoke only the
            // wrapper file name and avoid both long-path and quoting ambiguity.
            command
                .args(["/D", "/C"])
                .arg(wrapper.file_name().unwrap_or_default());
            command
        } else {
            Command::new(wrapper)
        };
    command
        .current_dir(gradle_root)
        // Build verification must produce a fresh artifact. Otherwise a stale
        // APK from an UP-TO-DATE task can be installed and falsely certified.
        .args(["assembleDebug", "--no-daemon", "--rerun-tasks"])
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

fn validate_package_name(value: &str) -> Result<&str> {
    let valid = !value.is_empty()
        && value.len() <= 220
        && value.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
    if !valid {
        bail!("basePackageName 不是合法的 Android applicationId")
    }
    Ok(value)
}

async fn wait_for_runtime(
    broker: &LiveUiBroker,
    session_id: &str,
    session: &LiveUiSession,
    expected_screen_id: Option<&str>,
) -> Result<super::protocol::LiveSessionView> {
    let started = Instant::now();
    loop {
        let view = session.view().await;
        let nodes_ready = if view.node_count == 0 {
            false
        } else if let Some(expected_screen_id) = expected_screen_id {
            let (_, nodes) = broker.tree(session_id).await?;
            nodes_match_preview(&nodes, expected_screen_id)
        } else {
            true
        };
        if view.connected && view.runtime_build_id.is_some() && nodes_ready {
            return Ok(view);
        }
        if started.elapsed() > Duration::from_secs(15) {
            if view.connected {
                if let Some(expected_screen_id) = expected_screen_id {
                    bail!(
                        "新 APK 已安装且 Runtime 已连接，但 Preview 场景 {expected_screen_id} 的节点树在 15 秒内没有上报"
                    );
                }
                bail!("新 APK 已安装且 Runtime 已连接，但节点树在 15 秒内没有上报");
            }
            bail!("新 APK 已安装，但 Debug Runtime 在 15 秒内没有重新连接");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn nodes_match_preview(nodes: &[LiveUiNode], expected_screen_id: &str) -> bool {
    let preview_definition_prefix = format!("preview.{expected_screen_id}.");
    nodes.iter().any(|node| {
        node.screen_id == expected_screen_id
            || node.definition_id == expected_screen_id
            || node.definition_id.starts_with(&preview_definition_prefix)
    })
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
    use std::collections::BTreeMap;
    use std::fs;

    use super::*;
    use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect};

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
    fn validates_android_base_package_name() {
        assert_eq!(
            validate_package_name("com.elon.app").unwrap(),
            "com.elon.app"
        );
        assert!(validate_package_name("com.elon.app;rm").is_err());
        assert!(validate_package_name("com..app").is_err());
        assert!(validate_package_name("1com.elon.app").is_err());
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

    #[test]
    fn matches_compose_and_view_preview_nodes() {
        let mut node = LiveUiNode {
            runtime_node_id: "runtime-1".to_string(),
            definition_id: "preview.compose.primary_card".to_string(),
            instance_key: None,
            parent_runtime_node_id: None,
            screen_id: "elon.compose.gallery".to_string(),
            kind: "compose".to_string(),
            text: None,
            resource_id: None,
            class_name: "PrimaryCard".to_string(),
            source: None,
            geometry: Default::default(),
            properties: Default::default(),
            capabilities: Default::default(),
        };
        assert!(nodes_match_preview(&[node.clone()], "elon.compose.gallery"));

        node.screen_id = "com.elon.uiruntime.view.UiRuntimePreviewHostActivity".to_string();
        node.definition_id = "preview.elon.view.gallery.root".to_string();
        assert!(nodes_match_preview(&[node], "elon.view.gallery"));
    }

    #[test]
    fn verification_target_requires_unique_stable_instance() {
        let nodes = vec![
            test_node("runtime-1", None, 0),
            test_node("runtime-2", None, 100),
        ];
        assert!(verification_bounds(&nodes, Some("card.action"), None).is_err());
    }

    #[test]
    fn verification_target_respects_instance_key() {
        let nodes = vec![
            test_node("runtime-1", Some("sku-1"), 0),
            test_node("runtime-2", Some("sku-2"), 100),
        ];
        let bounds = verification_bounds(&nodes, Some("card.action"), Some("sku-2"))
            .unwrap()
            .unwrap();
        assert_eq!(bounds.left, 100);
    }

    fn test_node(runtime_id: &str, instance_key: Option<&str>, left: i32) -> LiveUiNode {
        LiveUiNode {
            runtime_node_id: runtime_id.to_string(),
            definition_id: "card.action".to_string(),
            instance_key: instance_key.map(str::to_string),
            parent_runtime_node_id: None,
            screen_id: "catalog".to_string(),
            kind: "button".to_string(),
            text: None,
            resource_id: None,
            class_name: "Button".to_string(),
            source: None,
            geometry: LiveGeometry {
                bounds_in_display_px: LiveRect {
                    left,
                    top: 0,
                    right: left + 80,
                    bottom: 40,
                    width: 80,
                    height: 40,
                },
                density: 2.0,
                font_scale: 1.0,
                rotation: 0,
                visible: true,
            },
            properties: BTreeMap::new(),
            capabilities: BTreeMap::new(),
        }
    }
}
