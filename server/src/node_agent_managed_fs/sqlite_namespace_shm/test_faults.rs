//! Exact-connection deterministic SHM operation, barrier and teardown faults for tests.
//!
//! Installation requires a live WAL-main holder, so every step is fenced to its runtime
//! generation and SHM connection id. No constructor or state exists in non-test builds.

#[cfg(test)]
#[path = "test_faults/a2b2_phase_tests.rs"]
mod a2b2_phase_tests;
#[path = "test_faults/api.rs"]
mod api;
#[path = "test_faults/controller.rs"]
mod controller;
#[cfg(test)]
#[path = "test_faults/internal_phase_tests.rs"]
mod internal_phase_tests;
#[path = "test_faults/mapping.rs"]
mod mapping;
#[path = "test_faults/operation.rs"]
mod operation;
#[cfg(test)]
#[path = "test_faults/tests.rs"]
mod tests;

pub(crate) use api::ManagedSqliteShmTestFaultProbe;
pub(super) use controller::ManagedSqliteShmTestFaultController;
