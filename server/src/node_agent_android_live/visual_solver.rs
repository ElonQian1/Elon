use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::node_agent_android_inspector::adb_capture::capture_screen_png;

use super::broker::LiveUiBroker;
use super::protocol::{
    LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch, LiveUiNode,
};
use super::ui_ir::load_or_build_ui_ir;
use super::visual_diff::{compare_target_with_png, PixelRect, VisualDiffResult};

const DEFAULT_MAX_EVALUATIONS: usize = 16;
const HARD_MAX_EVALUATIONS: usize = 24;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualSolverRequest {
    pub(crate) runtime_node_id: String,
    pub(crate) target_rect: PixelRect,
    #[serde(default)]
    pub(crate) properties: Vec<String>,
    pub(crate) max_evaluations: Option<usize>,
    pub(crate) initial_step_dp: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualSolverResult {
    pub(crate) status: &'static str,
    pub(crate) runtime_node_id: String,
    pub(crate) evaluations: usize,
    pub(crate) baseline: VisualDiffResult,
    pub(crate) final_diff: VisualDiffResult,
    pub(crate) improvement_percent: f64,
    pub(crate) operations: Vec<LivePatchOperation>,
}

pub(crate) async fn solve_visual_style(
    broker: &LiveUiBroker,
    session_id: &str,
    request: VisualSolverRequest,
) -> Result<VisualSolverResult> {
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let target = ir
        .target_design
        .ok_or_else(|| anyhow!("请先导入并绑定目标设计图"))?;
    let node = ir
        .nodes
        .iter()
        .find(|node| node.runtime_node_id == request.runtime_node_id)
        .cloned()
        .ok_or_else(|| anyhow!("目标 Live Node 不存在"))?;
    if !node.geometry.visible {
        bail!("目标 Live Node 当前不可见");
    }
    let properties = solver_properties(&node, &request.properties)?;
    let max_evaluations = request
        .max_evaluations
        .unwrap_or(DEFAULT_MAX_EVALUATIONS)
        .clamp(1, HARD_MAX_EVALUATIONS);
    let initial_step = request.initial_step_dp.unwrap_or(4.0).clamp(0.25, 32.0);
    let session = broker.session(session_id).await?;
    let base_rect = rect_from_node(&node);
    let baseline_png = capture_screen_png(&session.device_id).await?;
    let baseline = compare_target_with_png(
        &target.path,
        &baseline_png,
        Some(request.target_rect),
        Some(base_rect),
    )?;
    let mut best_values = initial_values(&node, &properties);
    seed_geometry_target(&mut best_values, &node, request.target_rect);
    let mut best = evaluate(
        broker,
        session_id,
        &session.device_id,
        &target.path,
        &node,
        request.target_rect,
        &best_values,
    )
    .await?;
    let mut evaluations = 1;
    let mut step = initial_step;

    while evaluations < max_evaluations && step >= 0.25 {
        let mut improved = false;
        for property in &properties {
            if evaluations >= max_evaluations {
                break;
            }
            for direction in [-1.0, 1.0] {
                if evaluations >= max_evaluations {
                    break;
                }
                let mut candidate = best_values.clone();
                let current = candidate.get(property).copied().unwrap_or_default();
                candidate.insert(
                    property.clone(),
                    constrained_value(&node, property, current + step * direction),
                );
                let scored = evaluate(
                    broker,
                    session_id,
                    &session.device_id,
                    &target.path,
                    &node,
                    request.target_rect,
                    &candidate,
                )
                .await?;
                evaluations += 1;
                if scored.visual_loss + 0.000_001 < best.visual_loss {
                    best = scored;
                    best_values = candidate;
                    improved = true;
                }
            }
        }
        if !improved {
            step /= 2.0;
        }
    }

    let operations = operations_from_values(&best_values);
    let improved = best.visual_loss + 0.000_001 < baseline.visual_loss;
    let final_diff = if improved {
        broker
            .apply_patch(session_id, patch_for_node(&node, operations.clone(), false))
            .await?;
        tokio::time::sleep(Duration::from_millis(120)).await;
        let png = capture_screen_png(&session.device_id).await?;
        compare_target_with_png(
            &target.path,
            &png,
            Some(request.target_rect),
            Some(predicted_rect(&node, &best_values)),
        )?
    } else {
        baseline.clone()
    };
    let improvement_percent = if baseline.visual_loss <= f64::EPSILON {
        0.0
    } else {
        (((baseline.visual_loss - final_diff.visual_loss) / baseline.visual_loss) * 10_000.0)
            .round()
            / 100.0
    };
    Ok(VisualSolverResult {
        status: if improved { "APPLIED" } else { "NO_CHANGE" },
        runtime_node_id: node.runtime_node_id,
        evaluations,
        baseline,
        final_diff,
        improvement_percent,
        operations: if improved { operations } else { Vec::new() },
    })
}

async fn evaluate(
    broker: &LiveUiBroker,
    session_id: &str,
    device_id: &str,
    target_path: &str,
    node: &LiveUiNode,
    target_rect: PixelRect,
    values: &BTreeMap<String, f64>,
) -> Result<VisualDiffResult> {
    let patch = patch_for_node(node, operations_from_values(values), true);
    let (_, inverse) = broker.apply_probe_patch(session_id, patch).await?;
    tokio::time::sleep(Duration::from_millis(80)).await;
    let comparison = match capture_screen_png(device_id).await {
        Ok(png) => compare_target_with_png(
            target_path,
            &png,
            Some(target_rect),
            Some(predicted_rect(node, values)),
        ),
        Err(error) => Err(error),
    };
    let restore = broker.restore_probe_patch(session_id, inverse).await;
    if let Err(error) = restore {
        return Err(anyhow!("视觉求解试探后恢复失败: {error:#}"));
    }
    comparison
}

fn solver_properties(node: &LiveUiNode, requested: &[String]) -> Result<Vec<String>> {
    let defaults = ["width", "height", "translationX", "translationY"];
    let source = if requested.is_empty() {
        defaults
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    } else {
        requested.to_vec()
    };
    let mut result = Vec::new();
    for property in source {
        if !matches!(
            property.as_str(),
            "width"
                | "height"
                | "translationX"
                | "translationY"
                | "opacity"
                | "padding.start"
                | "padding.top"
                | "padding.end"
                | "padding.bottom"
                | "cornerRadius.all"
                | "textSize"
                | "borderWidth"
        ) {
            bail!("视觉求解不支持属性: {property}");
        }
        let editable = node
            .properties
            .get(&property)
            .map(|value| value.change_level == "LIVE")
            .unwrap_or(matches!(property.as_str(), "translationX" | "translationY"));
        if editable && !result.contains(&property) {
            result.push(property);
        }
    }
    if result.is_empty() {
        bail!("目标节点没有可用于视觉求解的 LIVE 数值属性");
    }
    Ok(result)
}

fn initial_values(node: &LiveUiNode, properties: &[String]) -> BTreeMap<String, f64> {
    let density = node.geometry.density.max(0.01) as f64;
    properties
        .iter()
        .map(|property| {
            let fallback = match property.as_str() {
                "width" => node.geometry.bounds_in_display_px.width as f64 / density,
                "height" => node.geometry.bounds_in_display_px.height as f64 / density,
                "opacity" => 1.0,
                _ => 0.0,
            };
            let value = node
                .properties
                .get(property)
                .and_then(|snapshot| snapshot.effective.as_ref())
                .and_then(|value| value.value.as_f64())
                .unwrap_or(fallback);
            (property.clone(), value)
        })
        .collect()
}

fn seed_geometry_target(values: &mut BTreeMap<String, f64>, node: &LiveUiNode, target: PixelRect) {
    let density = node.geometry.density.max(0.01) as f64;
    let current = &node.geometry.bounds_in_display_px;
    if values.contains_key("width") {
        values.insert(
            "width".to_string(),
            (target.right - target.left).max(1) as f64 / density,
        );
    }
    if values.contains_key("height") {
        values.insert(
            "height".to_string(),
            (target.bottom - target.top).max(1) as f64 / density,
        );
    }
    if values.contains_key("translationX") {
        values.insert(
            "translationX".to_string(),
            values.get("translationX").copied().unwrap_or_default()
                + (target.left - current.left) as f64 / density,
        );
    }
    if values.contains_key("translationY") {
        values.insert(
            "translationY".to_string(),
            values.get("translationY").copied().unwrap_or_default()
                + (target.top - current.top) as f64 / density,
        );
    }
}

fn constrained_value(node: &LiveUiNode, property: &str, value: f64) -> f64 {
    let constraints = node
        .properties
        .get(property)
        .and_then(|item| item.constraints.as_ref());
    let minimum = constraints
        .and_then(|value| value.get("minimum"))
        .and_then(|value| value.as_f64())
        .unwrap_or(if property == "opacity" {
            0.0
        } else {
            -10_000.0
        });
    let maximum = constraints
        .and_then(|value| value.get("maximum"))
        .and_then(|value| value.as_f64())
        .unwrap_or(if property == "opacity" { 1.0 } else { 10_000.0 });
    value.clamp(minimum, maximum)
}

fn operations_from_values(values: &BTreeMap<String, f64>) -> Vec<LivePatchOperation> {
    values
        .iter()
        .map(|(property, value)| LivePatchOperation {
            property: property.clone(),
            value: LivePropertyValue {
                value_type: match property.as_str() {
                    "textSize" => "sp",
                    "opacity" => "float",
                    _ => "dp",
                }
                .to_string(),
                value: json!((value * 1000.0).round() / 1000.0),
            },
        })
        .collect()
}

fn patch_for_node(
    node: &LiveUiNode,
    operations: Vec<LivePatchOperation>,
    probe: bool,
) -> LiveStylePatch {
    LiveStylePatch {
        protocol_version: 1,
        message_type: String::new(),
        session_id: String::new(),
        request_id: String::new(),
        gesture_id: Some(if probe {
            "visual-solver-probe".to_string()
        } else {
            format!("visual-solver:{}", uuid::Uuid::new_v4().simple())
        }),
        sequence: 0,
        base_tree_revision: None,
        target: LivePatchTarget {
            scope: "INSTANCE".to_string(),
            runtime_node_id: Some(node.runtime_node_id.clone()),
            definition_id: Some(node.definition_id.clone()),
            instance_key: node.instance_key.clone(),
        },
        atomic: true,
        ephemeral: true,
        operations,
    }
}

fn rect_from_node(node: &LiveUiNode) -> PixelRect {
    let value = &node.geometry.bounds_in_display_px;
    PixelRect {
        left: value.left,
        top: value.top,
        right: value.right,
        bottom: value.bottom,
    }
}

fn predicted_rect(node: &LiveUiNode, values: &BTreeMap<String, f64>) -> PixelRect {
    let base = &node.geometry.bounds_in_display_px;
    let density = node.geometry.density.max(0.01) as f64;
    let tx = values.get("translationX").copied().unwrap_or_default() * density;
    let ty = values.get("translationY").copied().unwrap_or_default() * density;
    let width = values
        .get("width")
        .map(|value| value * density)
        .unwrap_or(base.width as f64)
        .max(1.0);
    let height = values
        .get("height")
        .map(|value| value * density)
        .unwrap_or(base.height as f64)
        .max(1.0);
    let left = base.left as f64 + tx;
    let top = base.top as f64 + ty;
    PixelRect {
        left: left.round() as i32,
        top: top.round() as i32,
        right: (left + width).round() as i32,
        bottom: (top + height).round() as i32,
    }
}
