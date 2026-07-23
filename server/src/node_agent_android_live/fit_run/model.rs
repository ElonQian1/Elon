mod command;
mod domain;
mod mask;
mod replay_attachment;
mod run;
mod scoring;
mod transitions;
mod trial;

pub(crate) use command::{FitCommand, FitCommandResult};
pub(crate) use domain::{
    validate_identifier, FitCodexHandoff, FitEnvironment, FitHandoffStatus, FitRect, FitRunPhase,
    FitSessionContext, FitStateReplay, FitStateReplayAction, FitStateReplayStep, FitStopReason,
    FitTargetPair,
};
pub(crate) use mask::{FitMaskKind, FitMaskRegion, FitVisualMask};
pub(crate) use replay_attachment::{
    AttachStateReplayRequest, AttachStateReplayResult, FitRunAuditEvent, FitRunAuditOutcome,
};
pub(crate) use run::{CreateFitRunRequest, FitRunDocument};
pub(crate) use scoring::{FitBudget, FitBudgetUsage, FitCandidate, FitScore, FitThresholds};
pub(crate) use trial::{FitTrial, FitTrialCheckpoint, FitTrialKind};
