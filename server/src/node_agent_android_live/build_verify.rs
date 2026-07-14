use std::time::{Duration, Instant, SystemTime};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_android_inspector::{
    adb_capture::{capture_screen_png, launch_app},
    adb_command::{run_adb_text, validate_device_id},
    png_probe::png_dimensions,
};

use super::adb_session::{start_runtime, stop_runtime, DEFAULT_DEVICE_PORT};
use super::broker::{LiveUiBroker, LiveUiSession};
use super::build_verify_apk::select_fresh_debug_apk;
use super::preview::{open_preview, PreviewOpenRequest};
use super::protocol::LiveUiNode;
use super::ui_ir::load_or_build_ui_ir;
use super::verification_gate::{
    evaluate_verification_gates, VerificationGateInput, VerificationGateResult,
    VerificationGateState,
};
use super::visual_diff::{
    compare_pngs, compare_target_with_png_projected, PixelRect, VisualDiffResult,
};

mod geometry;
mod gradle;

use geometry::{patched_bounds, patched_bounds_for_nodes, verification_bounds};
use gradle::{
    canonical_project_root, find_gradle_root, gradle_wrapper, infer_debug_application_id_suffix,
    run_debug_build, validate_debug_application_id_suffix, validate_package_name,
};

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
    pub(crate) lease: Option<crate::node_agent_android_device_lease::AndroidDeviceLeaseProof>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareDebugRuntimeResult {
    pub(crate) package_name: String,
    pub(crate) session_id: Option<String>,
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
    prepare_debug_runtime_inner(broker, request, host_port, false).await
}

pub(crate) async fn bootstrap_debug_runtime(
    broker: &LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    host_port: u16,
) -> Result<PrepareDebugRuntimeResult> {
    prepare_debug_runtime_inner(broker, request, host_port, true).await
}

async fn prepare_debug_runtime_inner(
    broker: &LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    host_port: u16,
    keep_session: bool,
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
    match result {
        Ok(build) if keep_session => Ok(PrepareDebugRuntimeResult {
            package_name,
            session_id: Some(session_id),
            build,
        }),
        Ok(build) => {
            let _ = stop_runtime(&session).await;
            broker.remove_session(&session_id).await;
            Ok(PrepareDebugRuntimeResult {
                package_name,
                session_id: None,
                build,
            })
        }
        Err(error) => {
            let _ = stop_runtime(&session).await;
            broker.remove_session(&session_id).await;
            Err(error)
        }
    }
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
    let explicit_debug_application_id_suffix = request
        .debug_application_id_suffix
        .as_deref()
        .map(validate_debug_application_id_suffix)
        .transpose()?
        .map(str::to_string);
    let debug_application_id_suffix = match explicit_debug_application_id_suffix {
        Some(value) => Some(value),
        None => infer_debug_application_id_suffix(&gradle_root, &session.package_name)?,
    };
    run_debug_build(
        &gradle_root,
        &wrapper,
        debug_application_id_suffix.as_deref(),
    )
    .await?;
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
    let target_comparison_rect =
        target_comparison_current_rect(request.projected_current_rect, verified_current_rect);
    let visual_diff = target_path
        .as_deref()
        .map(|path| {
            compare_target_with_png_projected(
                path,
                &screenshot,
                request.target_rect,
                target_comparison_rect,
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

fn target_comparison_current_rect(
    projected_current_rect: Option<PixelRect>,
    verified_current_rect: Option<PixelRect>,
) -> Option<PixelRect> {
    // 目标设计图的 targetRect 位于设计坐标系；projectedCurrentRect 是它
    // 校准到 Android 显示坐标后的同一屏幕区域。目标门禁必须裁剪这个
    // 区域，不能拿 TextView 等节点的语义 bounds 代替，否则字形溢出会
    // 让完全相同的画面仍产生误差。节点边界仍用于独立 Source Parity。
    projected_current_rect.or(verified_current_rect)
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod target_comparison_rect_tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
        PixelRect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn projected_design_region_wins_over_node_semantic_bounds() {
        let projected = rect(488, 134, 592, 266);
        let node_bounds = rect(498, 134, 582, 266);
        assert_eq!(
            target_comparison_current_rect(Some(projected), Some(node_bounds)),
            Some(projected)
        );
    }

    #[test]
    fn node_bounds_remain_fallback_without_calibration() {
        let node_bounds = rect(498, 134, 582, 266);
        assert_eq!(
            target_comparison_current_rect(None, Some(node_bounds)),
            Some(node_bounds)
        );
    }
}
