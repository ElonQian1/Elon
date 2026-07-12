//! 可恢复的 Android UI 设计稿拟合任务。
//!
//! 本模块只编排已有 Live Runtime、视觉求解、Codex 和构建验收能力；
//! Android WebSocket 与 Patch Journal 仍由 `LiveUiBroker` 负责。

mod candidate;
mod handoff;
mod live_artifacts;
mod live_backend;
mod live_values;
mod model;
mod orchestrator;
mod routes;
mod service;
mod store;
mod workspace_revision;

pub(crate) use model::{
    CreateFitRunRequest, FitCommand, FitEnvironment, FitRect, FitRunDocument, FitRunPhase,
    FitScore, FitSessionContext, FitTargetPair, FitTrial,
};
pub(crate) use routes::protected_routes;
pub(crate) use service::FitRunService;
pub(crate) use workspace_revision::workspace_fingerprint;

#[cfg(test)]
mod tests;
