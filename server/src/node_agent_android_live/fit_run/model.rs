mod command;
mod domain;
mod run;
mod scoring;
mod transitions;
mod trial;

pub(crate) use command::{FitCommand, FitCommandResult};
pub(crate) use domain::{
    validate_identifier, FitCodexHandoff, FitEnvironment, FitHandoffStatus, FitRect, FitRunPhase,
    FitSessionContext, FitStopReason, FitTargetPair,
};
pub(crate) use run::{CreateFitRunRequest, FitRunDocument};
pub(crate) use scoring::{FitBudget, FitBudgetUsage, FitCandidate, FitScore, FitThresholds};
pub(crate) use trial::{FitTrial, FitTrialCheckpoint, FitTrialKind};
