use std::cmp::Ordering;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct FitBudget {
    pub(crate) max_duration_ms: u64,
    pub(crate) max_local_evaluations: u32,
    pub(crate) max_codex_rounds: u32,
    pub(crate) max_build_rounds: u32,
    pub(crate) max_no_improvement_trials: u32,
}

impl Default for FitBudget {
    fn default() -> Self {
        Self {
            max_duration_ms: 20 * 60 * 1_000,
            max_local_evaluations: 96,
            max_codex_rounds: 3,
            max_build_rounds: 4,
            max_no_improvement_trials: 16,
        }
    }
}

impl FitBudget {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.max_duration_ms == 0
            || self.max_local_evaluations == 0
            || self.max_no_improvement_trials == 0
        {
            bail!("FitRun 预算必须为正数");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitBudgetUsage {
    pub(crate) elapsed_ms: u64,
    pub(crate) local_evaluations: u32,
    pub(crate) codex_rounds: u32,
    pub(crate) build_rounds: u32,
    pub(crate) no_improvement_trials: u32,
    pub(crate) codex_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct FitThresholds {
    pub(crate) max_overall_loss: f64,
    pub(crate) max_geometry_error: f64,
    pub(crate) max_color_error: f64,
    pub(crate) max_edge_error: f64,
    pub(crate) max_source_parity_loss: f64,
    pub(crate) min_meaningful_improvement: f64,
    pub(crate) plateau_window: u32,
}

impl Default for FitThresholds {
    fn default() -> Self {
        Self {
            max_overall_loss: 0.035,
            max_geometry_error: 0.02,
            max_color_error: 0.04,
            max_edge_error: 0.06,
            max_source_parity_loss: 0.035,
            min_meaningful_improvement: 0.001,
            plateau_window: 6,
        }
    }
}

impl FitThresholds {
    pub(crate) fn validate(&self) -> Result<()> {
        let values = [
            self.max_overall_loss,
            self.max_geometry_error,
            self.max_color_error,
            self.max_edge_error,
            self.max_source_parity_loss,
            self.min_meaningful_improvement,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || self.plateau_window == 0
        {
            bail!("FitRun 阈值必须是非负有限数，plateauWindow 必须为正数");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitScore {
    pub(crate) scorer_version: String,
    pub(crate) overall_loss: f64,
    pub(crate) geometry_error: f64,
    pub(crate) color_error: f64,
    pub(crate) edge_error: f64,
    pub(crate) alpha_error: f64,
    pub(crate) shape_error: Option<f64>,
    pub(crate) typography_error: Option<f64>,
    #[serde(default)]
    pub(crate) hard_failures: Vec<String>,
}

impl FitScore {
    pub(crate) fn passes(&self, thresholds: &FitThresholds) -> bool {
        self.uncontrolled_hard_failure_count() == 0
            && self.overall_loss <= thresholds.max_overall_loss
            && self.geometry_error <= thresholds.max_geometry_error
            && self.color_error <= thresholds.max_color_error
            && self.edge_error <= thresholds.max_edge_error
    }

    pub(crate) fn better_than(&self, other: &Self, minimum_delta: f64) -> bool {
        match self
            .uncontrolled_hard_failure_count()
            .cmp(&other.uncontrolled_hard_failure_count())
        {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => self.overall_loss + minimum_delta < other.overall_loss,
        }
    }

    fn uncontrolled_hard_failure_count(&self) -> usize {
        self.hard_failures
            .iter()
            .filter(|failure| !matches!(failure.as_str(), "geometry" | "color" | "edge"))
            .count()
    }

    pub(crate) fn reconcile_threshold_failures(&mut self, thresholds: &FitThresholds) {
        self.hard_failures.retain(|failure| match failure.as_str() {
            "geometry" => self.geometry_error > thresholds.max_geometry_error,
            "color" => self.color_error > thresholds.max_color_error,
            "edge" => self.edge_error > thresholds.max_edge_error,
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{FitScore, FitThresholds};

    fn score(hard_failures: Vec<String>) -> FitScore {
        FitScore {
            scorer_version: "test".to_string(),
            overall_loss: 0.019832,
            geometry_error: 0.01,
            color_error: 0.030732,
            edge_error: 0.02,
            alpha_error: 0.0,
            shape_error: None,
            typography_error: None,
            hard_failures,
        }
    }

    #[test]
    fn run_thresholds_override_generic_metric_gate_failures() {
        assert!(score(vec!["color".to_string()]).passes(&FitThresholds::default()));
    }

    #[test]
    fn unrelated_hard_failures_still_block_acceptance() {
        assert!(!score(vec!["perceptual".to_string()]).passes(&FitThresholds::default()));
    }

    #[test]
    fn serialized_metric_failures_follow_run_thresholds() {
        let mut score = score(vec![
            "geometry".to_string(),
            "color".to_string(),
            "edge".to_string(),
            "perceptual".to_string(),
        ]);
        score.reconcile_threshold_failures(&FitThresholds::default());
        assert_eq!(score.hard_failures, vec!["perceptual"]);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCandidate {
    pub(crate) trial_id: String,
    pub(crate) score: FitScore,
    #[serde(default)]
    pub(crate) operations: Vec<Value>,
    pub(crate) screenshot_path: Option<String>,
    pub(crate) diff_artifact_path: Option<String>,
    pub(crate) runtime_build_id: Option<String>,
    pub(crate) source_revision: Option<String>,
    pub(crate) commit_id: Option<String>,
    pub(crate) source_parity_loss: Option<f64>,
    #[serde(default)]
    pub(crate) source_parity_verified: bool,
}
