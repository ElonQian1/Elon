use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::store::compute_external_pool_adapter_runtime_bundle::external_pool_adapter_provider_runtime_readiness_runtime;
use crate::types::AppState;

use super::report::ExternalPoolAdapterTaskWorkerCycleReport;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) async fn run(
    state: &Arc<AppState>,
    worker_id: &str,
) -> Result<ExternalPoolAdapterTaskWorkerCycleReport> {
    let runtime = external_pool_adapter_provider_runtime_readiness_runtime()
        .map_err(|_| anyhow!("external-pool Adapter task delivery lost V270 runtime custody"))?;
    let preparation = state
        .store
        .run_external_pool_adapter_active_preparation_cycle(
            &state.data_dir,
            runtime.as_ref(),
            worker_id,
        )
        .await?;

    // Source-stage delivery runs only after S2. There is deliberately no positive producer in
    // V278; if a durable row does appear, the report exposes it without fabricating a request.
    let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let cycle_state = Arc::clone(state);
    let observed_provider = tokio::task::spawn_blocking(move || {
        cycle_state
            .store
            .run_external_pool_adapter_task_delivery_source_cycle(&checked_at)
    })
    .await
    .map_err(|_| anyhow!("external-pool Adapter task source stage did not complete"))??;
    if let Some(provider_id) = observed_provider {
        let reproof_state = Arc::clone(state);
        let reproof_runtime = Arc::clone(&runtime);
        tokio::task::spawn_blocking(move || {
            reproof_state
                .store
                .reprove_external_pool_adapter_task_delivery_source(
                    &provider_id,
                    &reproof_state.data_dir,
                    reproof_runtime.as_ref(),
                )
        })
        .await
        .map_err(|_| {
            anyhow!("external-pool Adapter task final source reproof did not complete")
        })??;
    }
    Ok(ExternalPoolAdapterTaskWorkerCycleReport {
        active_preparation_completed: preparation.is_some(),
        eligible_rows: 0,
        delivery_attempted: false,
    })
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub(super) async fn run(
    _state: &Arc<AppState>,
    _worker_id: &str,
) -> Result<ExternalPoolAdapterTaskWorkerCycleReport> {
    Err(anyhow!(
        "external-pool Adapter task delivery requires Linux x86-64"
    ))
}
