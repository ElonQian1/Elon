use serde::Serialize;

use super::visual_diff::VisualDiffResult;

pub(crate) const DEFAULT_SOURCE_MAX_OPTIMIZATION_SCORE: f64 = 0.035;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum VerificationGateState {
    Passed,
    Failed,
    NotRequired,
    NotConfigured,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VerificationGateInput<'a> {
    pub(crate) source_parity_diff: Option<&'a VisualDiffResult>,
    pub(crate) target_fidelity_diff: Option<&'a VisualDiffResult>,
    pub(crate) target_required: bool,
    pub(crate) source_max_optimization_score: f64,
}

impl<'a> VerificationGateInput<'a> {
    pub(crate) fn new(
        source_parity_diff: Option<&'a VisualDiffResult>,
        target_fidelity_diff: Option<&'a VisualDiffResult>,
        target_required: bool,
    ) -> Self {
        Self {
            source_parity_diff,
            target_fidelity_diff,
            target_required,
            source_max_optimization_score: DEFAULT_SOURCE_MAX_OPTIMIZATION_SCORE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerificationGateResult {
    pub(crate) status: &'static str,
    pub(crate) verified: bool,
    pub(crate) source_parity: VerificationGateState,
    pub(crate) target_fidelity: VerificationGateState,
    pub(crate) failed_metrics: Vec<String>,
}

pub(crate) fn evaluate_verification_gates(
    input: VerificationGateInput<'_>,
) -> VerificationGateResult {
    let source_parity = match input.source_parity_diff {
        Some(diff)
            if diff.visual_loss <= input.source_max_optimization_score
                && diff.score_report.target_gate.passed =>
        {
            VerificationGateState::Passed
        }
        Some(_) | None => VerificationGateState::Failed,
    };
    let target_fidelity = if !input.target_required {
        VerificationGateState::NotRequired
    } else {
        match input.target_fidelity_diff {
            Some(diff) if diff.score_report.target_gate.passed => VerificationGateState::Passed,
            Some(_) => VerificationGateState::Failed,
            None => VerificationGateState::NotConfigured,
        }
    };

    let status = if source_parity != VerificationGateState::Passed {
        "SOURCE_MISMATCH"
    } else {
        match target_fidelity {
            VerificationGateState::Passed | VerificationGateState::NotRequired => "BUILD_VERIFIED",
            VerificationGateState::Failed => "TARGET_MISMATCH",
            VerificationGateState::NotConfigured => "TARGET_NOT_CONFIGURED",
        }
    };
    let verified = status == "BUILD_VERIFIED";
    let mut failed_metrics = Vec::new();
    if let Some(diff) = input.source_parity_diff {
        if source_parity == VerificationGateState::Failed {
            failed_metrics.extend(
                diff.score_report
                    .target_gate
                    .failed_metrics
                    .iter()
                    .map(|metric| format!("source.{metric}")),
            );
            if diff.visual_loss > input.source_max_optimization_score {
                failed_metrics.push("source.optimizationScore".to_string());
            }
        }
    } else {
        failed_metrics.push("source.missing".to_string());
    }
    if input.target_required {
        match input.target_fidelity_diff {
            Some(diff) if target_fidelity == VerificationGateState::Failed => failed_metrics
                .extend(
                    diff.score_report
                        .target_gate
                        .failed_metrics
                        .iter()
                        .map(|metric| format!("target.{metric}")),
                ),
            None => failed_metrics.push("target.notConfigured".to_string()),
            _ => {}
        }
    }
    failed_metrics.sort();
    failed_metrics.dedup();

    VerificationGateResult {
        status,
        verified,
        source_parity,
        target_fidelity,
        failed_metrics,
    }
}
