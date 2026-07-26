//! Lightweight metrics facade for realtime connection lifecycle events.
//!
//! The project currently relies on structured tracing rather than a dedicated
//! metrics backend. Keeping this facade small gives realtime modules one stable
//! place to report counters, and leaves a clean adapter point for Prometheus or
//! OpenTelemetry later.

mod admin;
mod catalog;
mod counters;

pub use admin::{admin_close_metrics, admin_diagnostics};
pub use catalog::realtime_diagnostics_catalog;
pub use counters::{close_metric_snapshot, record_close_with_store, RealtimeChannel};

#[cfg(test)]
pub(crate) use admin::admin_close_metrics_payload;
#[cfg(test)]
pub use catalog::RealtimeDiagnosticCloseReason;
#[cfg(test)]
pub use counters::{record_close, reset_for_tests, RealtimeCloseMetricSnapshot};

#[cfg(test)]
#[path = "realtime_metrics_tests.rs"]
mod tests;
