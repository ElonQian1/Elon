//! Commit-bound Map source-terminal review ledger.
//!
//! The ledger anchors reviewed branch outcomes, cleanup rewrites, the three SQLite result
//! projections plus the absent-output-slot shape, and six provisional authority success families.
//! Its exact ABI/raw reviewed-successor prefix stops at two explicit open frontiers. The wider
//! denominator-facing ABI fragment stops earlier at raw dispatch: it has 15 local terminal cells
//! and one continuation, not 16 frozen cases. The ledger intentionally retains `Pending`
//! dispositions and therefore is not a source-exhaustive terminal universe, `CaseKey`,
//! `SourceBranch`, `Expected`, `StaticContract`, denominator or Windows dynamic inventory.

mod invariants;
mod map;
mod model;
mod reviewed_trace;
mod scope;

pub(super) fn validate() -> Result<(), &'static str> {
    invariants::validate()
}
