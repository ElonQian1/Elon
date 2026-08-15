//! Default-off recovery scheduler for external-pool Adapter task delivery.
//!
//! V273 deliberately has no eligible production rows. This worker only freezes the startup gate
//! and scheduler seam; it does not construct v213 authority or perform network I/O.

use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::{anyhow, bail, Result};
use tokio::time::MissedTickBehavior;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::store::{
    compute_external_pool_adapter_runtime_bundle::external_pool_adapter_provider_runtime_readiness_runtime,
    external_pool_adapter_task_protocol_conformance_runtime,
};
use crate::types::AppState;

const ENABLED_ENV: &str = "ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED";
const RECOVERY_INTERVAL: Duration = Duration::from_secs(60);

static WORKER_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(Clone, Copy)]
struct ExternalPoolAdapterTaskWorkerCycleReport {
    eligible_rows: usize,
}

const DORMANT_CYCLE_REPORT: ExternalPoolAdapterTaskWorkerCycleReport =
    ExternalPoolAdapterTaskWorkerCycleReport { eligible_rows: 0 };

/// Freezes the process-wide worker gate after the V270 and V272 runtimes are initialized.
pub(crate) fn initialize_external_pool_adapter_task_worker_runtime() -> Result<()> {
    let enabled = configured_enabled()?;
    if enabled {
        require_production_runtime_custody()?;
    }
    WORKER_ENABLED
        .set(enabled)
        .map_err(|_| anyhow!("external-pool Adapter task worker initialized more than once"))
}

pub(crate) fn spawn(state: Arc<AppState>) {
    if WORKER_ENABLED.get().copied() != Some(true) {
        return;
    }
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RECOVERY_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let cycle_state = state.clone();
            let recovered = tokio::task::spawn_blocking(move || {
                cycle_state
                    .store
                    .recover_external_pool_adapter_task_delivery()
            })
            .await;
            if let Ok(Ok(eligible_rows)) = recovered {
                if eligible_rows != DORMANT_CYCLE_REPORT.eligible_rows {
                    continue;
                }
                let _eligible_rows = eligible_rows;
            }
        }
    });
}

fn configured_enabled() -> Result<bool> {
    match std::env::var_os(ENABLED_ENV) {
        None => Ok(false),
        Some(value) => match value.to_str() {
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            _ => bail!("external-pool Adapter task delivery enabled value is invalid"),
        },
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn require_production_runtime_custody() -> Result<()> {
    external_pool_adapter_provider_runtime_readiness_runtime().map_err(|_| {
        anyhow!("external-pool Adapter task delivery requires V270 runtime custody")
    })?;
    external_pool_adapter_task_protocol_conformance_runtime().map_err(|_| {
        anyhow!("external-pool Adapter task delivery requires V272 runtime custody")
    })?;
    Ok(())
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn require_production_runtime_custody() -> Result<()> {
    bail!("external-pool Adapter task delivery requires Linux x86-64")
}
