use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

pub(crate) const FIT_CASE_SCHEMA_VERSION: u32 = 1;
pub(crate) const FIT_PRIOR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitUserDecision {
    Accepted,
    Rejected,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCaseReview {
    pub(crate) decision: FitUserDecision,
    pub(crate) component_kind: String,
    pub(crate) decided_at: Option<String>,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitCaseOutcome {
    Accepted,
    Rejected,
    Failed,
    Plateau,
    Cancelled,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitPropertyAdjustment {
    pub(crate) property: String,
    pub(crate) first_value: Option<f64>,
    pub(crate) final_value: Option<f64>,
    pub(crate) delta: Option<f64>,
    pub(crate) observations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitScoreEvidence {
    pub(crate) scorer_version: String,
    pub(crate) overall_loss: f64,
    pub(crate) geometry_error: f64,
    pub(crate) color_error: f64,
    pub(crate) edge_error: f64,
    #[serde(default)]
    pub(crate) hard_failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitTrialEvidence {
    pub(crate) trial_id: String,
    pub(crate) kind: String,
    pub(crate) accepted_as_best: bool,
    pub(crate) score: Option<FitScoreEvidence>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCaseProvenance {
    pub(crate) run_id: String,
    pub(crate) target_sha256: String,
    pub(crate) source_revision: Option<String>,
    pub(crate) runtime_build_id: Option<String>,
    pub(crate) commit_id: Option<String>,
    pub(crate) trial_ids: Vec<String>,
    pub(crate) final_screenshot_path: Option<String>,
    pub(crate) final_diff_artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCaseEnvironment {
    pub(crate) screen_id: Option<String>,
    pub(crate) scenario: Option<String>,
    pub(crate) theme: Option<String>,
    pub(crate) locale: Option<String>,
    pub(crate) density: Option<f32>,
    pub(crate) font_scale: Option<f32>,
    pub(crate) viewport_width: Option<u32>,
    pub(crate) viewport_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCase {
    pub(crate) schema_version: u32,
    pub(crate) case_id: String,
    pub(crate) project_root: String,
    pub(crate) package_name: String,
    pub(crate) definition_id: String,
    pub(crate) component_kind: String,
    pub(crate) property_set: Vec<String>,
    pub(crate) environment: FitCaseEnvironment,
    pub(crate) run_phase: String,
    pub(crate) outcome: FitCaseOutcome,
    pub(crate) user_decision: FitUserDecision,
    pub(crate) target_score_passed: bool,
    pub(crate) source_parity_passed: bool,
    pub(crate) promotable: bool,
    pub(crate) baseline_score: Option<FitScoreEvidence>,
    pub(crate) final_score: Option<FitScoreEvidence>,
    pub(crate) source_parity_loss: Option<f64>,
    pub(crate) adjustments: Vec<FitPropertyAdjustment>,
    pub(crate) trials: Vec<FitTrialEvidence>,
    pub(crate) provenance: FitCaseProvenance,
    pub(crate) reviewed_at: String,
    pub(crate) review_note: Option<String>,
}

impl FitCase {
    pub(crate) fn screen_key(&self) -> &str {
        self.environment
            .screen_id
            .as_deref()
            .unwrap_or("unknown-screen")
    }

    pub(crate) fn passes_promotion_gates(&self) -> bool {
        self.run_phase == "ACCEPTED"
            && self.outcome == FitCaseOutcome::Accepted
            && self.user_decision == FitUserDecision::Accepted
            && self.target_score_passed
            && self.source_parity_passed
            && self.promotable
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitPriorScope {
    ExactComponent,
    CrossComponent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitPriorEnvironment {
    pub(crate) density: Option<f64>,
    pub(crate) font_scale: Option<f64>,
    pub(crate) theme: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitHoldoutSummary {
    pub(crate) evaluated: u32,
    pub(crate) regressions: u32,
    pub(crate) max_regression: f64,
    pub(crate) mean_loss_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitPrior {
    pub(crate) prior_id: String,
    pub(crate) scope: FitPriorScope,
    pub(crate) component_kind: String,
    pub(crate) definition_id: Option<String>,
    pub(crate) property_set: Vec<String>,
    pub(crate) environment: FitPriorEnvironment,
    pub(crate) success_count: u32,
    pub(crate) failure_count: u32,
    pub(crate) screen_count: u32,
    pub(crate) success_rate: f64,
    pub(crate) confidence: f64,
    pub(crate) median_deltas: BTreeMap<String, f64>,
    pub(crate) case_ids: Vec<String>,
    pub(crate) run_ids: Vec<String>,
    pub(crate) source_revisions: Vec<String>,
    pub(crate) target_sha256s: Vec<String>,
    pub(crate) holdout: Option<FitHoldoutSummary>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitPriorDocument {
    pub(crate) schema_version: u32,
    pub(crate) updated_at: String,
    pub(crate) priors: Vec<FitPrior>,
}

impl Default for FitPriorDocument {
    fn default() -> Self {
        Self {
            schema_version: FIT_PRIOR_SCHEMA_VERSION,
            updated_at: Utc::now().to_rfc3339(),
            priors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCaseDocument {
    pub(crate) schema_version: u32,
    pub(crate) updated_at: String,
    pub(crate) cases: Vec<FitCase>,
}

impl Default for FitCaseDocument {
    fn default() -> Self {
        Self {
            schema_version: FIT_CASE_SCHEMA_VERSION,
            updated_at: Utc::now().to_rfc3339(),
            cases: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FitPriorQuery {
    pub(crate) component_kind: String,
    pub(crate) definition_id: Option<String>,
    pub(crate) properties: Vec<String>,
    pub(crate) density: Option<f32>,
    pub(crate) font_scale: Option<f32>,
    pub(crate) theme: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct FitPriorMatch {
    pub(crate) prior: FitPrior,
    pub(crate) score: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct FitHoldoutResult {
    pub(crate) case_id: String,
    pub(crate) baseline_loss: f64,
    pub(crate) promoted_loss: f64,
    pub(crate) passed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FitPromotionDecision {
    pub(crate) prior_id: String,
    pub(crate) promoted: bool,
    pub(crate) reason: String,
    pub(crate) holdout: Option<FitHoldoutSummary>,
}

#[derive(Debug, Clone)]
pub(crate) struct FitPromotionResult {
    pub(crate) document: FitPriorDocument,
    pub(crate) decisions: Vec<FitPromotionDecision>,
}

#[derive(Debug, Clone)]
pub(crate) struct FitRecordAndPromoteResult {
    pub(crate) case: FitCase,
    pub(crate) recorded_case_count: usize,
    pub(crate) promotion: FitPromotionResult,
}
