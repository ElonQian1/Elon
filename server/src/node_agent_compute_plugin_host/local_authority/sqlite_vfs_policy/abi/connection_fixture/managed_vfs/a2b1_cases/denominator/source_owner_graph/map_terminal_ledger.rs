//! Commit-bound Map source-terminal review ledger.
//!
//! The ledger anchors reviewed branch outcomes, cleanup rewrites, the three SQLite result
//! projections plus the absent-output-slot shape, and six provisional authority success families.
//! It intentionally retains `Pending`
//! dispositions and therefore is not a source-exhaustive terminal universe, `CaseKey`, `Expected`,
//! `StaticContract`, denominator or Windows dynamic inventory.

mod invariants;
mod map;
mod model;
mod scope;

pub(super) fn validate() -> Result<(), &'static str> {
    invariants::validate()
}
