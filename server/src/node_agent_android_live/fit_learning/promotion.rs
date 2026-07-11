use anyhow::{bail, Result};
use chrono::Utc;

use super::eval::{evaluate_holdouts, FitHoldoutEvaluator};
use super::prior_index::{prior_matches_case, FitPriorIndex};
use super::types::{
    FitCase, FitPriorDocument, FitPromotionDecision, FitPromotionResult, FIT_PRIOR_SCHEMA_VERSION,
};

#[derive(Debug, Clone)]
pub(crate) struct FitPromotionPolicy {
    pub(crate) max_holdout_regression: f64,
    pub(crate) max_mean_loss_regression: f64,
    pub(crate) evaluate_training_evidence: bool,
}

impl Default for FitPromotionPolicy {
    fn default() -> Self {
        Self {
            max_holdout_regression: 0.002,
            max_mean_loss_regression: 0.0,
            evaluate_training_evidence: false,
        }
    }
}

impl FitPromotionPolicy {
    fn validate(&self) -> Result<()> {
        if !self.max_holdout_regression.is_finite()
            || self.max_holdout_regression < 0.0
            || !self.max_mean_loss_regression.is_finite()
            || self.max_mean_loss_regression < 0.0
        {
            bail!("FitPromotionPolicy 阈值非法");
        }
        Ok(())
    }
}

pub(crate) fn promote_priors(
    training_cases: &[FitCase],
    holdout_cases: &[FitCase],
    evaluator: &dyn FitHoldoutEvaluator,
    policy: &FitPromotionPolicy,
) -> Result<FitPromotionResult> {
    policy.validate()?;
    let candidates = FitPriorIndex::build_candidates(training_cases).into_priors();
    let mut promoted = Vec::new();
    let mut decisions = Vec::with_capacity(candidates.len());
    for mut prior in candidates {
        let holdouts = holdout_cases
            .iter()
            .filter(|case| {
                (policy.evaluate_training_evidence
                    || !prior
                        .case_ids
                        .iter()
                        .any(|case_id| case_id == &case.case_id))
                    && prior_matches_case(&prior, case)
            })
            .collect::<Vec<_>>();
        let report =
            evaluate_holdouts(&prior, &holdouts, evaluator, policy.max_holdout_regression)?;
        let regressed =
            report.regressions > 0 || report.mean_loss_delta > policy.max_mean_loss_regression;
        prior.holdout = Some(report.clone());
        if regressed {
            decisions.push(FitPromotionDecision {
                prior_id: prior.prior_id,
                promoted: false,
                reason: format!(
                    "holdout 回归：{} 个失败，平均损失变化 {:.6}",
                    report.regressions, report.mean_loss_delta
                ),
                holdout: Some(report),
            });
        } else {
            decisions.push(FitPromotionDecision {
                prior_id: prior.prior_id.clone(),
                promoted: true,
                reason: if report.evaluated == 0 {
                    "满足晋升证据门槛；当前无适用 holdout".to_string()
                } else {
                    format!("{} 个 holdout 无回归", report.evaluated)
                },
                holdout: Some(report),
            });
            promoted.push(prior);
        }
    }
    promoted.sort_by(|left, right| left.prior_id.cmp(&right.prior_id));
    Ok(FitPromotionResult {
        document: FitPriorDocument {
            schema_version: FIT_PRIOR_SCHEMA_VERSION,
            updated_at: Utc::now().to_rfc3339(),
            priors: promoted,
        },
        decisions,
    })
}
