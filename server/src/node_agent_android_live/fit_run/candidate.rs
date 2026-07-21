use anyhow::{anyhow, Result};
use serde_json::Value;

use super::model::{FitCandidate, FitRunDocument, FitScore};
use crate::node_agent_android_live::visual_diff::VisualDiffResult;

pub(super) fn from_diff(
    run: &FitRunDocument,
    trial_id: String,
    diff: VisualDiffResult,
    operations: Vec<Value>,
    screenshot_path: Option<String>,
) -> FitCandidate {
    let mut score = score_from_diff(&diff);
    score.reconcile_threshold_failures(&run.thresholds);
    FitCandidate {
        trial_id,
        score,
        operations,
        screenshot_path,
        diff_artifact_path: None,
        runtime_build_id: run.runtime_build_id.clone(),
        source_revision: run.source_revision.clone(),
        commit_id: None,
        source_parity_loss: None,
        source_parity_verified: false,
    }
}

pub(super) fn from_build_value(run: &FitRunDocument, value: &Value) -> Result<FitCandidate> {
    let visual = value
        .get("visualDiff")
        .ok_or_else(|| anyhow!("Build Verify 未返回目标设计图 visualDiff"))?;
    let source_parity_loss = value
        .get("sourceParityDiff")
        .and_then(|item| item.get("visualLoss"))
        .and_then(Value::as_f64);
    let mut score = FitScore {
        scorer_version: "visual-score-v1+projected-geometry-v1".to_string(),
        overall_loss: number(visual, "visualLoss")?,
        geometry_error: number(visual, "geometryError")?,
        color_error: number(visual, "meanAbsoluteColorError")?,
        edge_error: number(visual, "edgeError")?,
        alpha_error: number(visual, "alphaError")?,
        shape_error: None,
        typography_error: None,
        hard_failures: failed_metrics(visual),
    };
    score.reconcile_threshold_failures(&run.thresholds);
    Ok(FitCandidate {
        trial_id: new_trial_id("build"),
        score,
        operations: run
            .best
            .as_ref()
            .map(|value| value.operations.clone())
            .unwrap_or_default(),
        screenshot_path: None,
        diff_artifact_path: None,
        runtime_build_id: value
            .get("runtimeBuildId")
            .and_then(Value::as_str)
            .map(str::to_string),
        source_revision: run.source_revision.clone(),
        commit_id: run
            .handoff
            .as_ref()
            .and_then(|value| value.commit_id.clone()),
        source_parity_loss,
        source_parity_verified: value
            .get("sourceParityVerified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub(super) fn new_trial_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

fn score_from_diff(diff: &VisualDiffResult) -> FitScore {
    FitScore {
        scorer_version: "visual-score-v1+projected-geometry-v1".to_string(),
        overall_loss: diff.visual_loss,
        geometry_error: diff.geometry_error,
        color_error: diff.mean_absolute_color_error,
        edge_error: diff.edge_error,
        alpha_error: diff.alpha_error,
        shape_error: None,
        typography_error: None,
        hard_failures: diff
            .score_report
            .target_gate
            .failed_metrics
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn failed_metrics(value: &Value) -> Vec<String> {
    value
        .pointer("/scoreReport/targetGate/failedMetrics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn number(value: &Value, key: &str) -> Result<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Build Verify 缺少数值字段: {key}"))
}
