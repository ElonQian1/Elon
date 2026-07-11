use anyhow::Result;

use super::eval::FitHoldoutEvaluator;
use super::types::{FitCase, FitHoldoutResult, FitPrior};

#[derive(Debug, Clone)]
pub(crate) struct HistoricalAdjustmentEvaluator {
    pub(crate) max_absolute_delta_error: f64,
    pub(crate) max_relative_delta_error: f64,
}

impl Default for HistoricalAdjustmentEvaluator {
    fn default() -> Self {
        Self {
            max_absolute_delta_error: 2.0,
            max_relative_delta_error: 0.35,
        }
    }
}

impl FitHoldoutEvaluator for HistoricalAdjustmentEvaluator {
    fn evaluate(&self, prior: &FitPrior, case: &FitCase) -> Result<FitHoldoutResult> {
        let mut errors = Vec::new();
        let mut passed = true;
        for (property, expected) in &prior.median_deltas {
            let Some(actual) = case
                .adjustments
                .iter()
                .find(|adjustment| adjustment.property == *property)
                .and_then(|adjustment| adjustment.delta)
            else {
                passed = false;
                continue;
            };
            let absolute = (actual - expected).abs();
            let scale = actual.abs().max(expected.abs()).max(1.0);
            let relative = absolute / scale;
            if absolute > self.max_absolute_delta_error && relative > self.max_relative_delta_error
            {
                passed = false;
            }
            errors.push(relative);
        }
        if errors.is_empty() {
            passed = false;
        }
        let consistency_loss = if errors.is_empty() {
            1.0
        } else {
            errors.iter().sum::<f64>() / errors.len() as f64
        };
        Ok(FitHoldoutResult {
            case_id: case.case_id.clone(),
            baseline_loss: 0.0,
            promoted_loss: consistency_loss,
            passed,
        })
    }
}
