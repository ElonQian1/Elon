//! Commit-bound owner graph for the A2a/A2b1 map/lock source review.
//!
//! This graph freezes source owners and ordered edge classes only. It is intentionally not a
//! terminal-source universe, `CaseKey` set, `Expected` table, denominator, `StaticContract`, or
//! Windows dynamic-evidence inventory.

mod invariants;
mod lock;
mod map;
mod map_terminal_ledger;
mod model;
mod owners;
mod shared;

pub(super) fn validate_source_owner_graph() -> Result<(), &'static str> {
    invariants::validate()
}

pub(super) fn validate_map_terminal_review_ledger() -> Result<(), &'static str> {
    invariants::validate()?;
    map_terminal_ledger::validate()
}
