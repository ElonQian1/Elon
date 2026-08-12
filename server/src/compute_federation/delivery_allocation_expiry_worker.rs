//! Server-owned recovery worker for exercised DeliveryAllocation reservations.
//!
//! The worker consumes one bounded Store page per tick. The Store owns the durable
//! checkpoint and the exact Broker-finish authority; this scheduler never impersonates
//! an administrator and never inspects or logs per-reservation evidence.

#[cfg(test)]
#[path = "delivery_allocation_expiry_worker_tests.rs"]
mod tests;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use tokio::time::MissedTickBehavior;

use crate::{
    store::{ComputeDeliveryAllocationReservationExpiryWorkerPageReport, Store},
    types::AppState,
};

const INTERVAL_ENV: &str = "COMPUTE_DELIVERY_ALLOCATION_EXPIRY_WORKER_SECS";
const DEFAULT_INTERVAL_SECS: u64 = 60;
const MIN_INTERVAL_SECS: u64 = 10;
const PAGE_LIMIT: usize = 100;

pub(crate) fn spawn(state: Arc<AppState>) {
    let interval_secs = interval_secs(std::env::var(INTERVAL_ENV).ok().as_deref());
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let cycle_state = state.clone();
            match tokio::task::spawn_blocking(move || run_cycle(&cycle_state.store)).await {
                Ok(Ok(report)) => log_report(&report),
                Ok(Err(_)) => {
                    tracing::warn!("DeliveryAllocation reservation expiry worker page failed")
                }
                Err(_) => tracing::warn!(
                    "DeliveryAllocation reservation expiry worker task did not complete"
                ),
            }
        }
    });
}

fn run_cycle(store: &Store) -> Result<ComputeDeliveryAllocationReservationExpiryWorkerPageReport> {
    store.expire_due_compute_delivery_allocation_reservations_worker_page(PAGE_LIMIT)
}

fn interval_secs(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value >= MIN_INTERVAL_SECS)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

fn log_report(report: &ComputeDeliveryAllocationReservationExpiryWorkerPageReport) {
    if report.selected_count == 0 && report.sweep_completed {
        return;
    }
    if report.failed_count > 0 || report.blocked_count > 0 {
        tracing::warn!(
            selected = report.selected_count,
            expired = report.expired_count,
            replayed = report.replayed_count,
            blocked = report.blocked_count,
            failed = report.failed_count,
            sweep_completed = report.sweep_completed,
            "DeliveryAllocation reservation expiry worker page completed"
        );
    } else {
        tracing::info!(
            selected = report.selected_count,
            expired = report.expired_count,
            replayed = report.replayed_count,
            blocked = report.blocked_count,
            failed = report.failed_count,
            sweep_completed = report.sweep_completed,
            "DeliveryAllocation reservation expiry worker page completed"
        );
    }
}
