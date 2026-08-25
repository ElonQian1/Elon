//! Commit-bound Map source-terminal review ledger.
//!
//! The ledger anchors reviewed branch outcomes, cleanup rewrites, the three SQLite result
//! projections plus the absent-output-slot shape, and six provisional authority success families.
//! Its exact ABI/raw reviewed-successor prefix stops at two explicit open frontiers. The wider
//! ledger intentionally retains `Pending` dispositions and therefore is not a source-exhaustive
//! terminal universe, `CaseKey`, `SourceBranch`, `Expected`, `StaticContract`, denominator or
//! Windows dynamic inventory.

mod invariants;
mod map;
mod model;
mod reviewed_trace;
mod scope;

pub(super) fn validate() -> Result<(), &'static str> {
    invariants::validate()
}
