//! Default-off external_pool Adapter task-delivery worker.
//!
//! S2 active preparation always runs before the S3 source stage. V278 intentionally adds no
//! Offer/Job/plan/start producer, so a normal deployment remains dynamically ineligible.

mod cycle;
mod lifecycle;
mod report;

use std::sync::Arc;

use anyhow::Result;

use crate::types::AppState;

pub(crate) fn initialize_external_pool_adapter_task_worker_runtime() -> Result<()> {
    lifecycle::initialize()
}

pub(crate) fn spawn(state: Arc<AppState>) {
    lifecycle::spawn(state)
}
