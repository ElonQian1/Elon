use serde::{Deserialize, Serialize};

use super::{FitBudgetUsage, FitCandidate, FitRunPhase, FitStopReason};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitTrialKind {
    Baseline,
    LiveProbe,
    LiveApply,
    CodexEdit,
    BuildVerify,
    SourceVerify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitTrialCheckpoint {
    pub(crate) phase: FitRunPhase,
    pub(crate) stop_reason: Option<FitStopReason>,
    pub(crate) usage: FitBudgetUsage,
    pub(crate) current: Option<FitCandidate>,
    pub(crate) best: Option<FitCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitTrial {
    pub(crate) sequence: u64,
    pub(crate) trial_id: String,
    pub(crate) kind: FitTrialKind,
    pub(crate) created_at: String,
    pub(crate) duration_ms: u64,
    pub(crate) evaluations: u32,
    pub(crate) candidate: Option<FitCandidate>,
    pub(crate) accepted_as_best: bool,
    pub(crate) error: Option<String>,
    pub(crate) checkpoint: FitTrialCheckpoint,
}
