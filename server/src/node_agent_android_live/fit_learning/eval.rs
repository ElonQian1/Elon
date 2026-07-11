use anyhow::{bail, Result};

use super::types::{FitCase, FitHoldoutResult, FitHoldoutSummary, FitPrior};

pub(crate) trait FitHoldoutEvaluator: Send + Sync {
    fn evaluate(&self, prior: &FitPrior, case: &FitCase) -> Result<FitHoldoutResult>;
}

pub(crate) fn evaluate_holdouts(
    prior: &FitPrior,
    cases: &[&FitCase],
    evaluator: &dyn FitHoldoutEvaluator,
    max_regression: f64,
) -> Result<FitHoldoutSummary> {
    if !max_regression.is_finite() || max_regression < 0.0 {
        bail!("holdout maxRegression 必须是非负有限数");
    }
    let mut regressions = 0_u32;
    let mut maximum = 0.0_f64;
    let mut delta_sum = 0.0_f64;
    for case in cases {
        let result = evaluator.evaluate(prior, case)?;
        if !result.baseline_loss.is_finite() || !result.promoted_loss.is_finite() {
            bail!("holdout {} 返回非有限损失", result.case_id);
        }
        let delta = result.promoted_loss - result.baseline_loss;
        delta_sum += delta;
        maximum = maximum.max(delta);
        if !result.passed || delta > max_regression {
            regressions = regressions.saturating_add(1);
        }
    }
    Ok(FitHoldoutSummary {
        evaluated: cases.len() as u32,
        regressions,
        max_regression: maximum,
        mean_loss_delta: if cases.is_empty() {
            0.0
        } else {
            delta_sum / cases.len() as f64
        },
    })
}
