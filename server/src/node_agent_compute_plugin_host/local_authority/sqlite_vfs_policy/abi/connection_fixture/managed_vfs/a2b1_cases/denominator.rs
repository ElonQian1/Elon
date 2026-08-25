//! A2a/A2b1 map/lock schema and incomplete candidate branch-atom review scaffold.
//!
//! Source review found that cold lock acquire also traverses the complete node-initialization
//! graph. Consequently the schema, quotient, exact key set, Expected vectors, StaticContract, and
//! denominator count remain pending. This module validates the typed schema, this table's internal
//! partition/projection consistency, and the commit-bound source-owner graph. It does not establish
//! source-universe equality or terminal-leaf coverage.

mod branch_atoms;
mod case_key;
mod invariants;
mod projection;
mod source_owner_graph;

pub(super) fn validate_candidate_branch_atom_scaffold() -> Result<(), &'static str> {
    invariants::validate()
}

pub(super) fn validate_source_owner_graph() -> Result<(), &'static str> {
    source_owner_graph::validate_source_owner_graph()
}
