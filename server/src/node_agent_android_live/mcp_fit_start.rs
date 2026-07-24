use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::broker::LiveUiBroker;
use super::fit_run::{CreateFitRunRequest, FitEnvironment, FitRect, FitRunService, FitTargetPair};
use super::preview::{activate_preview_scenario, PreviewOpenRequest};
use super::ui_ir::load_or_build_ui_ir;

pub(super) async fn start(
    broker: &Arc<LiveUiBroker>,
    fit_runs: &FitRunService,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let mut environment: FitEnvironment = serde_json::from_value(
        arguments
            .get("environment")
            .cloned()
            .unwrap_or_else(|| json!({})),
    )
    .context("environment 参数无效")?;
    if let Some(preview) = preview_request(&environment) {
        let host_port = crate::node_agent_admin_open::admin_port_from_env();
        activate_preview_scenario(broker, session_id, preview, host_port).await?;
    }
    // Environment activation must finish before this refresh and selector
    // resolution, otherwise the selector observes the previous scenario tree.
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let target = ir
        .target_design
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("尚未绑定 TARGET_DESIGN，不能启动 FitRun"))?;
    let node = super::node_selector::resolve(&ir.nodes, arguments)?;
    let current = &node.geometry.bounds_in_display_px;
    if environment.screen_id.is_none() {
        environment.screen_id = Some(node.screen_id.clone());
    }
    let session = broker.session(session_id).await?;
    let request = CreateFitRunRequest {
        task_id: super::design_bootstrap::design_task_id(&session, arguments),
        pair: FitTargetPair {
            target_design_id: target.id.clone(),
            target_sha256: target.sha256.clone(),
            target_rect: fit_rect(arguments.get("targetRect"))?,
            runtime_node_id: node.runtime_node_id.clone(),
            definition_id: node.definition_id.clone(),
            component_kind: Some(node.kind.clone()),
            parent_layout_kind: None,
            instance_key: node.instance_key.clone(),
            current_rect: FitRect {
                left: current.left,
                top: current.top,
                right: current.right,
                bottom: current.bottom,
            },
            projected_target_rect: fit_rect(arguments.get("projectedTargetRect"))?,
            calibration_id: None,
            confidence: Some(1.0),
        },
        environment,
        properties: arguments
            .get("properties")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        budget: Default::default(),
        thresholds: Default::default(),
        visual_mask: serde_json::from_value(
            arguments
                .get("visualMask")
                .cloned()
                .unwrap_or_else(|| json!({})),
        )
        .context("visualMask 参数无效")?,
        auto_start: true,
    };
    let context = super::mcp::fit_session_context(broker, session_id).await?;
    Ok(json!({ "run": fit_runs.create_run(context, request).await? }))
}

fn preview_request(environment: &FitEnvironment) -> Option<PreviewOpenRequest> {
    let screen_id = environment
        .screen_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let scenario = environment
        .scenario
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if environment.state_replay.is_some() {
        return None;
    }
    Some(PreviewOpenRequest {
        screen_id: screen_id.to_string(),
        scenario: scenario.to_string(),
        theme: environment
            .theme
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("system")
            .to_string(),
        font_scale: environment.font_scale.unwrap_or(1.0),
        locale: environment
            .locale
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("zh-CN")
            .to_string(),
    })
}

fn fit_rect(value: Option<&Value>) -> Result<FitRect> {
    let rect: FitRect = serde_json::from_value(value.cloned().unwrap_or(Value::Null))
        .context("FitRun 矩形参数无效")?;
    rect.validate("FitRun rect")?;
    Ok(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_environment_materializes_registered_preview_before_selector_lookup() {
        let environment: FitEnvironment = serde_json::from_value(json!({
            "screenId": "elon.social.sidebar",
            "scenario": "favorites",
            "theme": "dark",
            "fontScale": 1.25,
            "locale": "zh-CN"
        }))
        .unwrap();
        let preview = preview_request(&environment).unwrap();
        assert_eq!(preview.screen_id, "elon.social.sidebar");
        assert_eq!(preview.scenario, "favorites");
        assert_eq!(preview.theme, "dark");
        assert_eq!(preview.font_scale, 1.25);
        assert_eq!(preview.locale, "zh-CN");
    }

    #[test]
    fn fit_environment_without_screen_keeps_the_existing_runtime() {
        let environment: FitEnvironment =
            serde_json::from_value(json!({"scenario":"CHAT_PAGE"})).unwrap();
        assert!(preview_request(&environment).is_none());
    }
}
