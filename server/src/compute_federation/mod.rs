//! Versioned domain contracts for the task-level distributed compute federation.
//!
//! This first layer is intentionally disconnected from persistence, HTTP, WebSocket and
//! scheduling. Existing node LLM routing remains the active compatibility path.

pub(crate) mod execution;
pub(crate) mod legacy;
pub(crate) mod market;
pub(crate) mod offer;
pub(crate) mod provider;
pub(crate) mod receipts;
pub(crate) mod workload;
