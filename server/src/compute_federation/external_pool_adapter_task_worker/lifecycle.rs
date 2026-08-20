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

use super::{cycle, report};

const ENABLED_ENV: &str = "ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED";
const RECOVERY_INTERVAL: Duration = Duration::from_secs(60);

static WORKER_ENABLED: OnceLock<bool> = OnceLock::new();

pub(super) fn initialize() -> Result<()> {
    let enabled = configured_enabled()?;
    if enabled {
        require_production_runtime_custody()?;
    }
    WORKER_ENABLED
        .set(enabled)
        .map_err(|_| anyhow!("external-pool Adapter task worker initialized more than once"))
}

pub(super) fn spawn(state: Arc<AppState>) {
    if WORKER_ENABLED.get().copied() != Some(true) {
        return;
    }
    let worker_id = format!("v278_task_worker_{}", uuid::Uuid::new_v4().simple());
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(RECOVERY_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match cycle::run(&state, &worker_id).await {
                Ok(cycle_report) => report::record(&cycle_report),
                Err(_) => {
                    tracing::warn!("external-pool Adapter task-delivery worker cycle failed closed")
                }
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
