//! Exact-connection deterministic SHM teardown faults for tests.
//!
//! Installation requires a live WAL-main holder, so every step is fenced to its runtime
//! generation and SHM connection id. No constructor or state exists in non-test builds.

#[path = "test_faults/api.rs"]
mod api;
#[path = "test_faults/controller.rs"]
mod controller;
#[cfg(test)]
#[path = "test_faults/tests.rs"]
mod tests;

pub(super) use controller::ManagedSqliteShmTestFaultController;
