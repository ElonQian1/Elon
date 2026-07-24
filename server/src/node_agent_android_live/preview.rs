use std::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_android_inspector::adb_command::{
    run_adb_text, validate_device_id, validate_package_name,
};

use super::adb_session::start_runtime;
use super::broker::{LiveUiBroker, LiveUiSession};
use super::frame_artifact::{capture_latest_frame_artifact, LiveFrameArtifact};
use super::ui_ir::load_or_build_ui_ir;

const PREVIEW_ACTIVITY: &str = "com.elon.uiruntime.view.UiRuntimePreviewHostActivity";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewOpenRequest {
    pub(crate) screen_id: String,
    #[serde(default = "normal_scenario")]
    pub(crate) scenario: String,
    #[serde(default = "system_theme")]
    pub(crate) theme: String,
    #[serde(default = "default_font_scale")]
    pub(crate) font_scale: f32,
    #[serde(default = "default_locale")]
    pub(crate) locale: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewOpenResult {
    screen_id: String,
    scenario: String,
    theme: String,
    font_scale: f32,
    locale: String,
    adb_output: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewActivationResult {
    preview: PreviewOpenResult,
    runtime_connected: bool,
    runtime_build_id: Option<String>,
    tree_revision: u64,
    node_count: usize,
    root_definition_ids: Vec<String>,
    screenshot: LiveFrameArtifact,
}

pub(crate) async fn activate_preview_scenario(
    broker: &LiveUiBroker,
    session_id: &str,
    request: PreviewOpenRequest,
    host_port: u16,
) -> Result<PreviewActivationResult> {
    let session = broker.session(session_id).await?;
    validate_request(&request)?;
    session.reset_for_redeploy().await;
    let start_evidence = start_runtime(&session, host_port).await?;
    let expected_screen_id = request.screen_id.clone();
    let preview = open_preview(&session, request).await?;
    let runtime = super::build_verify::wait_for_runtime(
        broker,
        session_id,
        &session,
        Some(&expected_screen_id),
        &start_evidence,
    )
    .await?;
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let root_definition_ids = ir
        .nodes
        .iter()
        .filter(|node| node.parent_runtime_node_id.is_none())
        .map(|node| node.definition_id.clone())
        .collect();
    let screenshot = capture_latest_frame_artifact(&session, None).await?;
    Ok(PreviewActivationResult {
        preview,
        runtime_connected: runtime.connected,
        runtime_build_id: runtime.runtime_build_id,
        tree_revision: runtime.tree_revision,
        node_count: ir.nodes.len(),
        root_definition_ids,
        screenshot,
    })
}

pub(crate) async fn open_preview(
    session: &LiveUiSession,
    request: PreviewOpenRequest,
) -> Result<PreviewOpenResult> {
    validate_device_id(&session.device_id)?;
    validate_package_name(&session.package_name)?;
    validate_request(&request)?;
    let component = format!("{}/{}", session.package_name, PREVIEW_ACTIVITY);
    let args = vec![
        "-s".to_string(),
        session.device_id.clone(),
        "shell".to_string(),
        "am".to_string(),
        "start".to_string(),
        "-n".to_string(),
        component,
        "--es".to_string(),
        "screen_id".to_string(),
        request.screen_id.clone(),
        "--es".to_string(),
        "scenario".to_string(),
        request.scenario.clone(),
        "--es".to_string(),
        "theme".to_string(),
        request.theme.clone(),
        "--ef".to_string(),
        "font_scale".to_string(),
        request.font_scale.to_string(),
        "--es".to_string(),
        "locale".to_string(),
        request.locale.clone(),
    ];
    let output = run_adb_text(&args, Duration::from_secs(12), 128 * 1024).await?;
    if output.contains("Error:") || output.contains("SecurityException") {
        bail!("启动 Preview Host 失败: {}", output.trim());
    }
    Ok(PreviewOpenResult {
        screen_id: request.screen_id,
        scenario: request.scenario,
        theme: request.theme,
        font_scale: request.font_scale,
        locale: request.locale,
        adb_output: output.trim().to_string(),
    })
}

fn validate_request(request: &PreviewOpenRequest) -> Result<()> {
    validate_text("screenId", &request.screen_id, 180)?;
    validate_text("scenario", &request.scenario, 80)?;
    validate_text("locale", &request.locale, 40)?;
    if !matches!(request.theme.as_str(), "system" | "light" | "dark") {
        bail!("theme 只允许 system/light/dark");
    }
    if !(0.5..=2.0).contains(&request.font_scale) {
        bail!("fontScale 必须在 0.5 到 2.0 之间");
    }
    Ok(())
}

fn validate_text(label: &str, value: &str, max_len: usize) -> Result<()> {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        bail!("{label} 为空或过长");
    }
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':')))
    {
        bail!("{label} 包含非法字符");
    }
    Ok(())
}

fn normal_scenario() -> String {
    "normal".to_string()
}
fn system_theme() -> String {
    "system".to_string()
}
fn default_font_scale() -> f32 {
    1.0
}
fn default_locale() -> String {
    "zh-CN".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_preview_inputs() {
        let valid = PreviewOpenRequest {
            screen_id: "checkout.main".to_string(),
            scenario: "loading".to_string(),
            theme: "dark".to_string(),
            font_scale: 1.3,
            locale: "zh-CN".to_string(),
        };
        assert!(validate_request(&valid).is_ok());
        assert!(validate_request(&PreviewOpenRequest {
            screen_id: "../../escape".to_string(),
            ..valid.clone()
        })
        .is_err());
        assert!(validate_request(&PreviewOpenRequest {
            font_scale: 5.0,
            ..valid
        })
        .is_err());
    }
}
