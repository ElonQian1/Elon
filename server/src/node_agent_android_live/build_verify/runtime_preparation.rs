use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};

use crate::node_agent_android_inspector::adb_capture::launch_app;

use super::gradle::{
    find_gradle_root, gradle_wrapper, run_debug_build, validate_debug_application_id_suffix,
};
use super::install::{install_debug_apk_with_evidence, list_legacy_debug_packages};
use super::preparation::PreparationReporter;
use super::runtime_reconnect::{ensure_live_without_install, RuntimeReuse};
use super::{
    capture_stable_runtime_frame, compare_source_parity, wait_for_runtime, BuildVerifyResult,
};
use crate::node_agent_android_live::adb_session::start_runtime_with_evidence;
use crate::node_agent_android_live::broker::{LiveUiBroker, LiveUiSession};
use crate::node_agent_android_live::build_verify_apk::{
    select_debug_apk_after_successful_build, select_reusable_debug_apk,
};
use crate::node_agent_android_live::debug_integration::DebugIntegrationPlan;
use crate::node_agent_android_live::fit_run::workspace_fingerprint;
use crate::node_agent_android_live::verification_gate::{
    evaluate_verification_gates, VerificationGateInput, VerificationGateState,
};

pub(super) const RUNTIME_HANDSHAKE_ATTEMPTS: usize = 2;
const APP_LAUNCH_ATTEMPTS: usize = 2;

pub(super) async fn run(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
    build_project_root: &Path,
    debug_application_id_suffix: &str,
    host_port: u16,
    integration_plan: &DebugIntegrationPlan,
    reporter: Option<&PreparationReporter>,
) -> Result<BuildVerifyResult> {
    let project_root = build_project_root.canonicalize()?;
    let project_root_display = project_root.to_string_lossy().to_string();
    let gradle_root = find_gradle_root(&project_root)?;
    let wrapper = gradle_wrapper(&gradle_root)?;
    let suffix = validate_debug_application_id_suffix(debug_application_id_suffix)?;
    broker.debug_integration.mark_building(integration_plan)?;

    report_phase(reporter, "APK_REUSE_CHECK", "检查已成功生成的 Debug APK").await;
    let build_started = Instant::now();
    let (apk, reused_apk) = match select_reusable_debug_apk(&gradle_root, &session.package_name)? {
        Some(apk) => {
            report_evidence(
                reporter,
                "APK_REUSE_CHECK",
                "REUSED",
                format!("复用源码时序有效的 Debug APK: {}", apk.display()),
            )
            .await;
            (apk, true)
        }
        None => {
            report_phase(
                reporter,
                "BUILD",
                "没有可安全复用的 APK；启动增量 assembleDebug",
            )
            .await;
            run_debug_build(
                &gradle_root,
                &wrapper,
                Some(suffix),
                broker.fixed_debug_label().as_deref(),
                false,
            )
            .await
            .context("BUILD 阶段失败")?;
            let apk = match select_reusable_debug_apk(&gradle_root, &session.package_name)? {
                Some(apk) => apk,
                None => {
                    report_evidence(
                        reporter,
                        "BUILD",
                        "RETRY",
                        "增量构建没有刷新 APK；执行一次 --rerun-tasks 以排除陈旧产物",
                    )
                    .await;
                    run_debug_build(
                        &gradle_root,
                        &wrapper,
                        Some(suffix),
                        broker.fixed_debug_label().as_deref(),
                        true,
                    )
                    .await
                    .context("BUILD 强制刷新阶段失败")?;
                    let apk = select_debug_apk_after_successful_build(
                        &gradle_root,
                        &session.package_name,
                    )
                    .context("强制 Gradle 构建后的 APK 选择失败")?;
                    report_evidence(
                        reporter,
                        "BUILD",
                        "CACHE_OUTPUT_ACCEPTED",
                        "强制 Gradle 构建成功；接受 applicationId 精确匹配的时间戳保留缓存产物",
                    )
                    .await;
                    apk
                }
            };
            report_evidence(
                reporter,
                "BUILD",
                "PASSED",
                format!("assembleDebug 完成: {}", apk.display()),
            )
            .await;
            (apk, false)
        }
    };
    let build_duration_ms = if reused_apk {
        0
    } else {
        build_started.elapsed().as_millis()
    };

    let source_revision = workspace_fingerprint(&project_root_display)?;
    if reused_apk {
        match ensure_live_without_install(
            broker,
            session,
            host_port,
            source_revision.as_deref(),
            reporter,
        )
        .await
        {
            RuntimeReuse::Live(runtime_view) => {
                return finalize_runtime(
                    session,
                    &project_root_display,
                    &apk,
                    build_duration_ms,
                    "SKIPPED_RUNTIME_REUSED",
                    runtime_view,
                    "当前源码对应的 APK 与 Runtime 已存在；已跳过安装并恢复稳定帧。",
                    reporter,
                )
                .await;
            }
            RuntimeReuse::NeedsInstall(reason) => {
                report_evidence(reporter, "RUNTIME_RECONNECT", "INSTALL_REQUIRED", reason).await;
            }
        }
    }

    let expected_label = session
        .package_name
        .starts_with("com.elon.app.uituner_")
        .then(|| broker.fixed_debug_label())
        .flatten();
    let artifact = crate::node_agent_android_live::apk_identity::verify_and_stage_apk(
        &apk,
        &gradle_root,
        &broker.debug_integration.artifact_root(integration_plan),
        &session.package_name,
        expected_label.as_deref(),
        integration_plan.generation,
    )
    .context("APK_IDENTITY 阶段失败")?;
    broker
        .debug_integration
        .record_artifact(integration_plan, artifact.clone())?;
    broker
        .debug_integration
        .authorize_install(integration_plan)?;
    let staged_apk = std::path::PathBuf::from(&artifact.apk_path);
    let legacy_packages = list_legacy_debug_packages(&session.device_id, &session.package_name)
        .await
        .context("LEGACY_PACKAGE_SCAN 阶段失败")?;
    broker
        .debug_integration
        .record_legacy_packages(integration_plan, legacy_packages)?;

    report_phase(
        reporter,
        "INSTALL",
        format!(
            "先校验设备 {} 的授权与在线状态，再安装 Debug APK",
            session.device_id
        ),
    )
    .await;
    let install = install_debug_apk_with_evidence(
        &session.device_id,
        &session.package_name,
        &staged_apk,
        false,
    )
    .await
    .context("INSTALL 阶段失败")?;
    report_evidence(
        reporter,
        "ADB_DEVICE_CHECK",
        "PASSED",
        format!(
            "deviceState={} probeAttempts={} reconnect={}",
            install.device_state,
            install.device_probe_attempts,
            install.reconnect_output.as_deref().unwrap_or("not-needed")
        ),
    )
    .await;
    broker.debug_integration.record_deployed(integration_plan)?;
    report_evidence(
        reporter,
        "INSTALL",
        "PASSED",
        format!(
            "installAttempts={} output={}",
            install.install_attempts,
            install.output.trim()
        ),
    )
    .await;

    session.reset_for_redeploy().await;
    let launch_output = launch_with_retry(session)
        .await
        .context("LAUNCH 阶段失败")?;
    report_evidence(
        reporter,
        "LAUNCH",
        "PASSED",
        format!(
            "package={} output={}",
            session.package_name,
            launch_output.trim()
        ),
    )
    .await;

    let mut first_handshake_error = None;
    let mut runtime_view = None;
    for attempt in 1..=RUNTIME_HANDSHAKE_ATTEMPTS {
        report_phase(
            reporter,
            "PORT_HANDSHAKE",
            format!(
                "配置 adb reverse 并等待 Runtime，attempt={attempt}/{RUNTIME_HANDSHAKE_ATTEMPTS}"
            ),
        )
        .await;
        let start = match start_runtime_with_evidence(session, host_port).await {
            Ok(start) => start,
            Err(error) if attempt < RUNTIME_HANDSHAKE_ATTEMPTS => {
                let detail = format!("attempt={attempt} startError={error:#}");
                report_evidence(reporter, "PORT_HANDSHAKE", "RETRY", &detail).await;
                first_handshake_error = Some(detail);
                let _ = launch_with_retry(session).await?;
                continue;
            }
            Err(error) => {
                return Err(
                    handshake_failure(session, first_handshake_error.as_deref(), &error).await,
                )
            }
        };
        report_evidence(
            reporter,
            "PORT_FORWARD",
            "PASSED",
            format!(
                "devicePort={} hostPort={} reverseOutput={}",
                session.device_port,
                host_port,
                start.reverse_output.trim()
            ),
        )
        .await;
        report_evidence(
            reporter,
            "RUNTIME_START",
            "PASSED",
            format!(
                "launchOutput={} broadcastOutput={}",
                start.launch_output.trim(),
                start.broadcast_output.trim()
            ),
        )
        .await;
        match wait_for_runtime(broker, &session.id, session, None, &start).await {
            Ok(view) => {
                runtime_view = Some(view);
                break;
            }
            Err(error) if attempt < RUNTIME_HANDSHAKE_ATTEMPTS => {
                let view = session.view().await;
                let detail = format!(
                    "attempt={attempt} connected={} runtimeBuildId={} nodeCount={} error={error:#}",
                    view.connected,
                    view.runtime_build_id.as_deref().unwrap_or("none"),
                    view.node_count
                );
                report_evidence(reporter, "RUNTIME_HANDSHAKE", "RETRY", &detail).await;
                first_handshake_error = Some(detail);
                let _ = launch_with_retry(session).await?;
                continue;
            }
            Err(error) => {
                return Err(
                    handshake_failure(session, first_handshake_error.as_deref(), &error).await,
                )
            }
        }
    }
    let runtime_view = runtime_view.expect("bounded handshake loop either connects or returns");

    finalize_runtime(
        session,
        &project_root_display,
        &staged_apk,
        build_duration_ms,
        install.output.trim(),
        runtime_view,
        if reused_apk {
            "已复用当前源码对应的 Debug APK，并恢复会话完成安装、启动和 Runtime 握手。"
        } else {
            "Debug Runtime 已增量构建、安装并连接，节点树已经就绪。"
        },
        reporter,
    )
    .await
}

async fn finalize_runtime(
    session: &LiveUiSession,
    project_root: &str,
    apk: &std::path::Path,
    build_duration_ms: u128,
    install_output: &str,
    runtime_view: crate::node_agent_android_live::protocol::LiveSessionView,
    message: &str,
    reporter: Option<&PreparationReporter>,
) -> Result<BuildVerifyResult> {
    report_phase(reporter, "CAPTURE", "Runtime 已连接，等待稳定帧与节点树").await;
    tokio::time::sleep(Duration::from_millis(2_200)).await;
    let source_frame = capture_stable_runtime_frame(session).await?;
    let (source_parity_diff, _) =
        compare_source_parity(&source_frame.bytes, &source_frame.bytes, None, None)?;
    let verification_gate = evaluate_verification_gates(VerificationGateInput::new(
        Some(&source_parity_diff),
        None,
        false,
    ));
    let source_parity_verified = verification_gate.source_parity == VerificationGateState::Passed;
    let runtime_build_id = runtime_view.runtime_build_id.clone();
    if source_parity_verified {
        if let Some(source_revision) = workspace_fingerprint(project_root)? {
            session
                .record_source_proof(
                    source_revision,
                    runtime_build_id.clone(),
                    source_parity_diff.visual_loss,
                )
                .await;
        }
    }
    Ok(BuildVerifyResult {
        status: verification_gate.status,
        apk_path: apk.display().to_string(),
        build_duration_ms,
        install_output: install_output.to_string(),
        runtime_connected: runtime_view.connected,
        runtime_build_id,
        node_count: runtime_view.node_count,
        screenshot_width: source_frame.width,
        screenshot_height: source_frame.height,
        visual_diff: None,
        source_parity_diff,
        source_parity_scope: "PROCESS_FRAME_BASELINE",
        source_parity_verified,
        verification_gate,
        message: message.to_string(),
    })
}

async fn launch_with_retry(session: &LiveUiSession) -> Result<String> {
    let mut first_error = None;
    for attempt in 1..=APP_LAUNCH_ATTEMPTS {
        match launch_app(&session.device_id, &session.package_name).await {
            Ok(output) => return Ok(output),
            Err(error) if attempt < APP_LAUNCH_ATTEMPTS && is_transient_adb_error(&error) => {
                first_error = Some(error.to_string());
                tokio::time::sleep(Duration::from_millis(800)).await;
            }
            Err(error) => {
                return Err(anyhow!(
                    "包启动失败 attempt={attempt}/{APP_LAUNCH_ATTEMPTS}; firstError={}; finalError={error:#}",
                    first_error.as_deref().unwrap_or("none")
                ));
            }
        }
    }
    unreachable!("bounded launch loop always returns")
}

fn is_transient_adb_error(error: &anyhow::Error) -> bool {
    let detail = error.to_string().to_ascii_lowercase();
    [
        "device offline",
        "device not found",
        "connection reset",
        "closed",
    ]
    .iter()
    .any(|signature| detail.contains(signature))
}

async fn handshake_failure(
    session: &LiveUiSession,
    first_error: Option<&str>,
    final_error: &anyhow::Error,
) -> anyhow::Error {
    let view = session.view().await;
    anyhow!(
        "RUNTIME_HANDSHAKE 在 {RUNTIME_HANDSHAKE_ATTEMPTS} 次有界尝试后失败；device={} package={} connected={} runtimeBuildId={} nodeCount={} firstAttempt={} finalError={final_error:#}",
        session.device_id,
        session.package_name,
        view.connected,
        view.runtime_build_id.as_deref().unwrap_or("none"),
        view.node_count,
        first_error.unwrap_or("none")
    )
}

async fn report_phase(
    reporter: Option<&PreparationReporter>,
    phase: &str,
    detail: impl AsRef<str>,
) {
    if let Some(reporter) = reporter {
        reporter.phase(phase, detail).await;
    }
}

async fn report_evidence(
    reporter: Option<&PreparationReporter>,
    phase: &str,
    status: &str,
    detail: impl AsRef<str>,
) {
    if let Some(reporter) = reporter {
        reporter.evidence(phase, status, detail).await;
    }
}
