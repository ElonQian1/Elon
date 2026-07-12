use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use crate::node_agent_android_inspector::adb_capture::capture_screen_png;

use super::broker::LiveUiBroker;
use super::protocol::{LivePatchOperation, LivePatchTarget, LiveStylePatch, LiveUiNode};
use super::ui_ir::load_or_build_ui_ir;
use super::visual_diff::{compare_target_with_png_projected, PixelRect, VisualDiffResult};
use super::visual_solver_style_hints::target_color_operations;
use super::visual_solver_values::{
    constrained_value, initial_values, operations_from_values, predicted_rect,
    property_search_step, seed_geometry_target, seed_prior_deltas, solver_properties,
};

const DEFAULT_MAX_EVALUATIONS: usize = 16;
const HARD_MAX_EVALUATIONS: usize = 24;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisualSolverRequest {
    pub(crate) runtime_node_id: String,
    pub(crate) target_rect: PixelRect,
    /// The target rectangle after the design canvas has been projected into
    /// Android display coordinates. Keeping it separate prevents design pixels
    /// from being treated as device pixels.
    pub(crate) projected_current_rect: Option<PixelRect>,
    #[serde(default)]
    pub(crate) properties: Vec<String>,
    pub(crate) max_evaluations: Option<usize>,
    pub(crate) initial_step_dp: Option<f64>,
    /// 来自项目内已验收 FitCase 的属性增量先验。只作为求解起点，
    /// 最终仍必须由当前真机帧和硬门禁重新验证。
    #[serde(default)]
    pub(crate) initial_property_deltas: BTreeMap<String, f64>,
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
    pub(crate) projected_current_rect: PixelRect,
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
    let fixed_operations = target_color_operations(
        &target.path,
        request.target_rect,
        &node,
        &request.properties,
    )?;
    if properties.is_empty() && fixed_operations.is_empty() {
        bail!("目标节点没有可用于视觉求解的 LIVE 样式属性");
    }
    let max_evaluations = request
        .max_evaluations
        .unwrap_or(DEFAULT_MAX_EVALUATIONS)
        .clamp(1, HARD_MAX_EVALUATIONS);
    let initial_step = request.initial_step_dp.unwrap_or(4.0).clamp(0.25, 32.0);
    let session = broker.session(session_id).await?;
    let base_rect = rect_from_node(&node);
    let projected_current_rect = request
        .projected_current_rect
        .unwrap_or(request.target_rect);
    let baseline_png = capture_screen_png(&session.device_id).await?;
    let baseline = compare_target_with_png_projected(
        &target.path,
        &baseline_png,
        Some(request.target_rect),
        Some(base_rect),
        Some(projected_current_rect),
    )?;
    let mut best_values = initial_values(&node, &properties);
    seed_prior_deltas(&mut best_values, &node, &request.initial_property_deltas);
    seed_geometry_target(&mut best_values, &node, projected_current_rect);
    let mut best = evaluate(
        broker,
        session_id,
        &session.device_id,
        &target.path,
        &node,
        request.target_rect,
        projected_current_rect,
        &best_values,
        &fixed_operations,
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
                let property_step = property_search_step(property, step);
                candidate.insert(
                    property.clone(),
                    constrained_value(&node, property, current + property_step * direction),
                );
                let scored = evaluate(
                    broker,
                    session_id,
                    &session.device_id,
                    &target.path,
                    &node,
                    request.target_rect,
                    projected_current_rect,
                    &candidate,
                    &fixed_operations,
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

    let operations = combined_operations(&best_values, &fixed_operations);
    let improved = best.visual_loss + 0.000_001 < baseline.visual_loss;
    let final_diff = if improved {
        broker
            .apply_patch(session_id, patch_for_node(&node, operations.clone(), false))
            .await?;
        tokio::time::sleep(Duration::from_millis(120)).await;
        let png = capture_screen_png(&session.device_id).await?;
        compare_target_with_png_projected(
            &target.path,
            &png,
            Some(request.target_rect),
            Some(predicted_rect(&node, &best_values)),
            Some(projected_current_rect),
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
        projected_current_rect,
    })
}

async fn evaluate(
    broker: &LiveUiBroker,
    session_id: &str,
    device_id: &str,
    target_path: &str,
    node: &LiveUiNode,
    target_rect: PixelRect,
    projected_current_rect: PixelRect,
    values: &BTreeMap<String, f64>,
    fixed_operations: &[LivePatchOperation],
) -> Result<VisualDiffResult> {
    let patch = patch_for_node(node, combined_operations(values, fixed_operations), true);
    let (_, inverse) = broker.apply_probe_patch(session_id, patch).await?;
    tokio::time::sleep(Duration::from_millis(80)).await;
    let comparison = match capture_screen_png(device_id).await {
        Ok(png) => compare_target_with_png_projected(
            target_path,
            &png,
            Some(target_rect),
            Some(predicted_rect(node, values)),
            Some(projected_current_rect),
        ),
        Err(error) => Err(error),
    };
    let restore = broker.restore_probe_patch(session_id, inverse).await;
    if let Err(error) = restore {
        return Err(anyhow!("视觉求解试探后恢复失败: {error:#}"));
    }
    comparison
}

fn combined_operations(
    values: &BTreeMap<String, f64>,
    fixed_operations: &[LivePatchOperation],
) -> Vec<LivePatchOperation> {
    let mut operations = fixed_operations.to_vec();
    operations.extend(operations_from_values(values));
    operations
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
