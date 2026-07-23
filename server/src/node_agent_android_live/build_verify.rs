use std::time::{Duration, Instant, SystemTime};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_android_inspector::adb_command::validate_device_id;

use super::adb_session::{
    runtime_failure_diagnostics, start_runtime, stop_runtime, RuntimeStartEvidence,
    DEFAULT_DEVICE_PORT,
};
use super::broker::{LiveUiBroker, LiveUiSession};
use super::build_verify_apk::select_fresh_debug_apk;
use super::fit_run::workspace_fingerprint;
use super::frame::{capture_runtime_frame_image, RuntimeFrameImage};
use super::preview::{open_preview, PreviewOpenRequest};
use super::protocol::LiveUiNode;
use super::ui_ir::load_or_build_ui_ir;
use super::verification_gate::{
    evaluate_verification_gates, VerificationGateInput, VerificationGateResult,
    VerificationGateState,
};
use super::visual_diff::{
    compare_pngs, compare_target_with_png_projected_masked, PixelRect, VisualDiffResult, VisualMask,
};

mod device_health;
mod geometry;
mod gradle;
mod install;
mod preparation;
mod runtime_preparation;
mod runtime_reconnect;

pub(crate) use preparation::PreparationRegistry;

use device_health::ensure_android_framework_ready;
use geometry::{patched_bounds, patched_bounds_for_nodes, verification_bounds};
use gradle::{
    canonical_project_root, find_gradle_root, gradle_wrapper, infer_debug_application_id_suffix,
    run_debug_build, validate_debug_application_id_suffix, validate_package_name,
};
use install::{install_debug_apk, list_legacy_debug_packages};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyRequest {
    pub(crate) preview: Option<PreviewOpenRequest>,
    pub(crate) debug_application_id_suffix: Option<String>,
    #[serde(default)]
    pub(crate) lkg_enabled: bool,
    pub(crate) target_rect: Option<PixelRect>,
    pub(crate) current_rect: Option<PixelRect>,
    pub(crate) projected_current_rect: Option<PixelRect>,
    pub(crate) target_definition_id: Option<String>,
    pub(crate) target_instance_key: Option<String>,
    #[serde(default)]
    pub(crate) visual_mask: VisualMask,
}

#[derive(Debug, Clone, Serialize)]
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
    pub(crate) source_parity_scope: &'static str,
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
    #[serde(default)]
    pub(crate) isolated_emulator_package: bool,
    #[serde(default)]
    pub(crate) lkg_enabled: bool,
    pub(crate) candidate: Option<super::debug_integration::DebugMergeCandidateRequest>,
    pub(crate) lease: Option<crate::node_agent_android_device_lease::AndroidDeviceLeaseProof>,
    #[serde(skip)]
    pub(crate) integration_plan: Option<super::debug_integration::DebugIntegrationPlan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareDebugRuntimeResult {
    pub(crate) package_name: String,
    pub(crate) session_id: Option<String>,
    pub(crate) build: BuildVerifyResult,
    pub(crate) integration: super::debug_integration::DebugIntegrationStatus,
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
    prepare_debug_runtime_inner(broker, request, host_port, false, None).await
}

async fn bootstrap_debug_runtime_with_reporter(
    broker: &LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    host_port: u16,
    reporter: Option<&preparation::PreparationReporter>,
) -> Result<PrepareDebugRuntimeResult> {
    prepare_debug_runtime_inner(broker, request, host_port, true, reporter).await
}

async fn prepare_debug_runtime_inner(
    broker: &LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    host_port: u16,
    keep_session: bool,
    reporter: Option<&preparation::PreparationReporter>,
) -> Result<PrepareDebugRuntimeResult> {
    let device_id = request.device_id.trim();
    let requested_base_package_name = validate_package_name(request.base_package_name.trim())?;
    let base_package_name = super::debug_base_package_name(requested_base_package_name);
    let project_root = request.project_root.trim();
    let install_id = broker
        .node_install_id()
        .context("PC 节点缺少稳定安装标识，拒绝创建会话级临时调试包")?;
    let suffix = super::resolve_debug_application_id_suffix(
        request.debug_application_id_suffix.trim(),
        install_id,
        device_id,
        request.isolated_emulator_package,
    )?;
    validate_debug_application_id_suffix(&suffix)?;
    if device_id.is_empty() {
        bail!("deviceId 不能为空");
    }
    validate_device_id(device_id)?;
    if project_root.is_empty() {
        bail!("projectRoot 不能为空；请先在 PC 工作台选择本机项目");
    }
    let package_name = format!("{base_package_name}{suffix}");
    let project_id = request
        .lease
        .as_ref()
        .map(|lease| lease.project_id.as_str())
        .unwrap_or(project_root);
    let device_identity = request
        .lease
        .as_ref()
        .map(|lease| lease.hardware_serial.as_str())
        .unwrap_or(device_id);
    let integration_plan = match request.integration_plan.clone() {
        Some(plan) => plan,
        None => broker.debug_integration.register_candidate(
            project_root,
            project_id,
            device_identity,
            &package_name,
            request.candidate.as_ref(),
            "compat-debug-prepare",
            Some(request.lkg_enabled),
        )?,
    };
    if let Some(reporter) = reporter {
        reporter
            .phase(
                "DEPLOYMENT_SLOT",
                format!("等待设备 {device_id} 的固定调试包 {package_name} 部署时隙"),
            )
            .await;
    }
    let deployment = broker
        .debug_deployments
        .acquire(device_identity, &package_name)
        .await;
    // Keep every fallible build/install/handshake step inside this result
    // boundary. `?` returns from the async block, then the deployment lease is
    // explicitly released before the failure reaches the preparation reporter
    // or another MCP request.
    let outcome = async {
        let integration_root = broker
            .debug_integration
            .materialize(&integration_plan)
            .context("MERGE 阶段失败")?;
        if let Some(reporter) = reporter {
            reporter
                .evidence("DEPLOYMENT_SLOT", "ACQUIRED", "已取得独占部署时隙")
                .await;
        }
        let reusable_session = if keep_session {
            broker
                .runtime_session_for(
                    project_root,
                    device_id,
                    &package_name,
                    device_identity,
                    project_id,
                )
                .await
        } else {
            None
        };
        let reused_session = reusable_session.is_some();
        let session = match reusable_session {
            Some(session) => session,
            None => {
                broker
                    .create_session_with_identity(
                        device_id.to_string(),
                        device_identity.to_string(),
                        package_name.clone(),
                        Some(project_root.to_string()),
                        project_id.to_string(),
                        DEFAULT_DEVICE_PORT,
                    )
                    .await
            }
        };
        if let Some(reporter) = reporter {
            reporter
                .evidence(
                    "SESSION",
                    if reused_session { "REUSED" } else { "CREATED" },
                    format!(
                        "sessionId={} device={} package={}",
                        session.id, session.device_id, session.package_name
                    ),
                )
                .await;
        }
        let session_id = session.id.clone();
        let result = runtime_preparation::run(
            broker,
            &session,
            &integration_root,
            &suffix,
            host_port,
            &integration_plan,
            reporter,
        )
        .await;
        if let Err(error) = result.as_ref() {
            let _ = broker
                .debug_integration
                .record_runtime_failure(&integration_plan, format!("{error:#}"));
        }
        match result {
            Ok(build) if keep_session => {
                let integration = broker
                    .debug_integration
                    .status(&integration_plan.slot_id)?
                    .context("合并调试状态丢失")?;
                Ok(PrepareDebugRuntimeResult {
                    package_name,
                    session_id: Some(session_id),
                    build,
                    integration,
                })
            }
            Ok(build) => {
                let _ = stop_runtime(&session).await;
                broker.remove_session(&session_id).await;
                let integration = broker
                    .debug_integration
                    .status(&integration_plan.slot_id)?
                    .context("合并调试状态丢失")?;
                Ok(PrepareDebugRuntimeResult {
                    package_name,
                    session_id: None,
                    build,
                    integration,
                })
            }
            Err(error) if !keep_session => {
                let _ = stop_runtime(&session).await;
                broker.remove_session(&session_id).await;
                Err(error)
            }
            Err(error) => Err(error),
        }
    }
    .await;
    finish_debug_deployment(deployment, outcome)
}

fn finish_debug_deployment<T>(
    deployment: tokio::sync::OwnedMutexGuard<()>,
    outcome: Result<T>,
) -> Result<T> {
    drop(deployment);
    outcome
}

pub(crate) async fn build_and_verify(
    broker: &LiveUiBroker,
    session_id: &str,
    request: BuildVerifyRequest,
    host_port: u16,
) -> Result<BuildVerifyResult> {
    build_and_verify_inner(broker, session_id, request, host_port).await
}

async fn build_and_verify_inner(
    broker: &LiveUiBroker,
    session_id: &str,
    request: BuildVerifyRequest,
    host_port: u16,
) -> Result<BuildVerifyResult> {
    let session = broker.session(session_id).await?;
    let source_project_root = canonical_project_root(&session)?;
    let install_id = broker
        .node_install_id()
        .context("PC 节点缺少稳定安装标识，拒绝部署调试 APK")?;
    let normalized_package =
        super::normalize_debug_package_name(&session.package_name, install_id, &session.device_id)?;
    if normalized_package != session.package_name {
        bail!("DEBUG_SESSION_PACKAGE_NOT_CANONICAL: 旧会话包 {} 会产生第二个真机应用；请用固定包 {} 重新连接", session.package_name, normalized_package);
    }
    let integration_plan = broker.debug_integration.register_candidate(
        source_project_root.to_string_lossy().as_ref(),
        &session.debug_project_id,
        &session.device_identity,
        &session.package_name,
        None,
        &session.id,
        Some(request.lkg_enabled),
    )?;
    let _deployment = broker
        .debug_deployments
        .acquire(&session.device_identity, &session.package_name)
        .await;
    let integration_root = broker.debug_integration.materialize(&integration_plan)?;
    let target_path = load_or_build_ui_ir(broker, session_id)
        .await
        .ok()
        .and_then(|ir| ir.target_design.map(|target| target.path));
    let project_root = integration_root.canonicalize()?;
    let gradle_root = find_gradle_root(&project_root)?;
    let wrapper = gradle_wrapper(&gradle_root)?;
    let live_snapshot = session.commit_snapshot().await;
    let frame = capture_stable_runtime_frame(&session).await?;
    let rect = verification_bounds(
        &live_snapshot.nodes,
        request.target_definition_id.as_deref(),
        request.target_instance_key.as_deref(),
    )?
    .or_else(|| patched_bounds(&live_snapshot, true));
    let live_preview = (frame.bytes, rect);

    broker.debug_integration.mark_building(&integration_plan)?;
    let build_started = Instant::now();
    let artifact_not_before = SystemTime::now()
        .checked_sub(Duration::from_secs(2))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let explicit_debug_application_id_suffix = request
        .debug_application_id_suffix
        .as_deref()
        .map(|requested| {
            super::resolve_debug_application_id_suffix(
                requested,
                install_id,
                &session.device_id,
                false,
            )
        })
        .transpose()?
        .map(|value| validate_debug_application_id_suffix(&value).map(str::to_string))
        .transpose()?;
    let debug_application_id_suffix = match explicit_debug_application_id_suffix {
        Some(value) => Some(value),
        None => infer_debug_application_id_suffix(&gradle_root, &session.package_name)?,
    };
    run_debug_build(
        &gradle_root,
        &wrapper,
        debug_application_id_suffix.as_deref(),
        broker.fixed_debug_label().as_deref(),
        true,
    )
    .await?;
    let build_duration_ms = build_started.elapsed().as_millis();
    let apk = select_fresh_debug_apk(&gradle_root, &session.package_name, artifact_not_before)?;
    let expected_label = session
        .package_name
        .starts_with("com.elon.app.uituner_")
        .then(|| broker.fixed_debug_label())
        .flatten();
    let artifact = super::apk_identity::verify_and_stage_apk(
        &apk,
        &gradle_root,
        &broker.debug_integration.artifact_root(&integration_plan),
        &session.package_name,
        expected_label.as_deref(),
        integration_plan.generation,
    )?;
    broker
        .debug_integration
        .record_artifact(&integration_plan, artifact.clone())?;
    broker
        .debug_integration
        .authorize_install(&integration_plan)?;
    let staged_apk = std::path::PathBuf::from(&artifact.apk_path);
    validate_device_id(&session.device_id)?;
    let legacy_packages = list_legacy_debug_packages(&session.device_id, &session.package_name)
        .await
        .context("LEGACY_PACKAGE_SCAN 阶段失败")?;
    broker
        .debug_integration
        .record_legacy_packages(&integration_plan, legacy_packages)?;

    let install_output = install_debug_apk(
        &session.device_id,
        &session.package_name,
        &staged_apk,
        debug_application_id_suffix.is_some(),
    )
    .await?;
    if !install_output.to_ascii_lowercase().contains("success") {
        bail!("Debug APK 安装未返回 Success: {}", install_output.trim());
    }
    broker
        .debug_integration
        .record_deployed(&integration_plan)?;

    session.reset_for_redeploy().await;
    let start_evidence = start_runtime(&session, host_port).await?;
    // A connected socket with an empty tree is not a verified UI. Always wait
    // for at least one runtime node, including normal Activity verification.
    let preview = request.preview.clone();
    if let Some(preview) = preview.as_ref() {
        open_preview(&session, preview.clone()).await?;
    }
    let expected_screen_id = preview.as_ref().map(|preview| preview.screen_id.as_str());
    let runtime_view = match wait_for_runtime(
        broker,
        session_id,
        &session,
        expected_screen_id,
        &start_evidence,
    )
    .await
    {
        Ok(view) => view,
        Err(first_error) => {
            // Some vendor systems finish `adb install -r` before the replaced
            // process and its debug receiver are fully ready. Re-launch and
            // bootstrap once more instead of leaving the PC page in a false
            // disconnected state after a successful install.
            let retry_evidence = start_runtime(&session, host_port).await?;
            if let Some(preview) = preview.as_ref() {
                open_preview(&session, preview.clone()).await?;
            }
            wait_for_runtime(
                broker,
                session_id,
                &session,
                expected_screen_id,
                &retry_evidence,
            )
            .await
            .with_context(|| format!("重新连接 Debug Runtime 失败；首次错误: {first_error:#}"))?
        }
    };
    // The Activity can expose its first View tree before async data binding and
    // entrance rendering finish. Comparing that intermediate frame against a
    // stable Live preview creates false BUILD_MISMATCH results. Give the debug
    // renderer a deterministic settle window, then require consecutive frames.
    tokio::time::sleep(Duration::from_millis(2_200)).await;
    let source_frame = capture_stable_runtime_frame(&session).await?;
    let screenshot_width = source_frame.width;
    let screenshot_height = source_frame.height;
    let screenshot = source_frame.bytes;
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
            compare_target_with_png_projected_masked(
                path,
                &screenshot,
                request.target_rect,
                target_comparison_rect,
                request.projected_current_rect,
                &request.visual_mask,
            )
        })
        .transpose()?;
    // A first-time Runtime preparation has no meaningful Live preview yet.
    // Treat the freshly installed renderer as its own baseline and reserve the
    // strict before/after parity gate for a real committed Live design session.
    let (source_parity_diff, source_parity_scope) =
        compare_source_parity(&live_preview.0, &screenshot, live_preview.1, source_rect)?;
    let verification_gate = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source_parity_diff),
        visual_diff.as_ref(),
        target_path.is_some(),
    ));
    let source_parity_verified = verification_gate.source_parity == VerificationGateState::Passed;
    let runtime_build_id = runtime_view.runtime_build_id.clone();
    if source_parity_verified {
        if let Some(source_revision) =
            workspace_fingerprint(project_root.to_string_lossy().as_ref())?
        {
            session
                .record_source_proof(
                    source_revision,
                    runtime_build_id.clone(),
                    source_parity_diff.visual_loss,
                )
                .await;
        }
    }
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
        apk_path: staged_apk.display().to_string(),
        build_duration_ms,
        install_output: install_output.trim().to_string(),
        runtime_connected: runtime_view.connected,
        runtime_build_id,
        node_count: runtime_view.node_count,
        screenshot_width,
        screenshot_height,
        visual_diff,
        source_parity_diff,
        source_parity_scope,
        source_parity_verified,
        verification_gate,
        message,
    })
}

/// Prefer an exact whole-process-frame match before consulting node geometry.
///
/// Runtime geometry and the PixelCopy frame travel over separate messages. A
/// recomposition can therefore leave a short-lived, stale node rectangle even
/// though the before/after Android renderer output is byte-for-byte identical.
/// Exact encoded-frame equality is stronger evidence than any crop comparison.
/// When even one frame byte differs we retain the strict target-node crop gate,
/// so unrelated dynamic pixels cannot hide a real component mismatch.
fn compare_source_parity(
    live_frame: &[u8],
    source_frame: &[u8],
    live_rect: Option<PixelRect>,
    source_rect: Option<PixelRect>,
) -> Result<(VisualDiffResult, &'static str)> {
    if live_frame == source_frame {
        let mut diff = compare_pngs(live_frame, source_frame, None, None)?;
        // Transparent or letterboxed frames can have zero eligible foreground
        // pixels, which makes the generic design scorer fail its coverage gate.
        // Byte-for-byte process frames are stronger evidence than that derived
        // coverage heuristic, so normalize the exact-match result explicitly.
        diff.mean_absolute_color_error = 0.0;
        diff.edge_error = 0.0;
        diff.alpha_error = 0.0;
        diff.geometry_error = 0.0;
        diff.visual_loss = 0.0;
        diff.score_report.optimization_score = 0.0;
        diff.score_report.target_gate.passed = true;
        diff.score_report.target_gate.geometry_passed = true;
        diff.score_report.target_gate.position_passed = true;
        diff.score_report.target_gate.color_passed = true;
        diff.score_report.target_gate.edge_passed = true;
        diff.score_report.target_gate.perceptual_passed = true;
        diff.score_report.target_gate.coverage_passed = true;
        diff.score_report.target_gate.failed_metrics.clear();
        return Ok((diff, "PROCESS_FRAME_EXACT"));
    }
    Ok((
        compare_pngs(live_frame, source_frame, live_rect, source_rect)?,
        "TARGET_NODE_CROP",
    ))
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

pub(super) async fn capture_stable_runtime_frame(
    session: &LiveUiSession,
) -> Result<RuntimeFrameImage> {
    let mut previous = capture_runtime_frame_image(session).await?;
    let mut stable_frames = 0_u8;
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(350)).await;
        let current = capture_runtime_frame_image(session).await?;
        let diff = compare_pngs(&previous.bytes, &current.bytes, None, None)?;
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

pub(super) async fn wait_for_runtime(
    broker: &LiveUiBroker,
    session_id: &str,
    session: &LiveUiSession,
    expected_screen_id: Option<&str>,
    start_evidence: &RuntimeStartEvidence,
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
            let diagnostics = runtime_failure_diagnostics(session, Some(start_evidence)).await;
            if view.connected {
                if let Some(expected_screen_id) = expected_screen_id {
                    bail!(
                        "新 APK 已安装且 Runtime 已连接，但 Preview 场景 {expected_screen_id} 的节点树在 15 秒内没有上报；{diagnostics}"
                    );
                }
                bail!("新 APK 已安装且 Runtime 已连接，但节点树在 15 秒内没有上报；{diagnostics}");
            }
            bail!("新 APK 已安装，但 Debug Runtime 在 15 秒内没有重新连接；{diagnostics}");
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
