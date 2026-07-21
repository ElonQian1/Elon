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
    CreateFitRunRequest, FitCommand, FitEnvironment, FitMaskKind, FitMaskRegion, FitRect,
    FitRunDocument, FitRunPhase, FitScore, FitSessionContext, FitTargetPair, FitTrial,
    FitVisualMask,
};
pub(crate) use routes::protected_routes;
pub(crate) use service::FitRunService;
pub(crate) use workspace_revision::workspace_fingerprint;

pub(crate) fn durable_runtime_candidates(
    project_root: &str,
) -> anyhow::Result<Vec<(String, String, Option<String>, Option<String>, String)>> {
    Ok(store::FitRunStore::new()
        .list_for_project(project_root)?
        .into_iter()
        .map(|run| {
            (
                run.device_id,
                run.package_name,
                run.source_revision,
                run.task_id,
                run.updated_at,
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
