//! Background billing reconciliation monitor.

use std::{sync::Arc, time::Duration};

use crate::types::AppState;

pub fn spawn_reconciliation_monitor(state: Arc<AppState>) {
    let interval_secs = std::env::var("BILLING_RECONCILIATION_MONITOR_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(300)
        .max(30);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            match state.store.refresh_billing_alerts() {
                Ok(alerts) => {
                    let critical = alerts
                        .iter()
                        .filter(|alert| alert.severity == "critical")
                        .count();
                    let warning = alerts
                        .iter()
                        .filter(|alert| alert.severity == "warning")
                        .count();
                    if critical > 0 {
                        tracing::warn!(
                            critical,
                            warning,
                            "billing reconciliation monitor found open alerts"
                        );
                    } else if warning > 0 {
                        tracing::info!(
                            warning,
                            "billing reconciliation monitor found warning alerts"
                        );
                    }
                }
                Err(e) => tracing::warn!("billing reconciliation monitor error: {}", e),
            }
        }
    });
}
