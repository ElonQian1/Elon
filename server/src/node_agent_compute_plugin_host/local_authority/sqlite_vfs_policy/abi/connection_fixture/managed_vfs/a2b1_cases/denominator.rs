//! A2a/A2b1 map/lock schema and incomplete candidate branch-atom review scaffold.
//!
//! Source review found that cold lock acquire also traverses the complete node-initialization
//! graph. Consequently the schema, quotient, exact key set, Expected vectors, StaticContract, and
//! denominator count remain pending. This module validates the typed schema, this table's internal
//! partition/projection consistency, the commit-bound source-owner graph, and a separate Map-only
//! terminal-template review ledger. Separate ABI, source-neutral raw-state, Map raw-projection,
//! typed Map outer-result, route/callback, and adapter-projection fragments freeze only their
//! local quotients; managed provenance and all later custody axes remain pending. The ledger
//! preserves its own pending closure gates and does not establish source-universe equality or
//! terminal-leaf coverage.

mod abi_map_fragment;
mod adapter_projection_fragment;
mod branch_atoms;
mod case_key;
mod invariants;
mod projection;
mod raw_map_fragment;
mod raw_state_fragment;
mod route_callback_fragment;
mod source_owner_graph;
mod typed_map_fragment;

pub(super) fn validate_candidate_branch_atom_scaffold() -> Result<(), &'static str> {
    invariants::validate()
}

pub(super) fn validate_source_owner_graph() -> Result<(), &'static str> {
    source_owner_graph::validate_source_owner_graph()
}

pub(super) fn validate_map_terminal_review_ledger() -> Result<(), &'static str> {
    source_owner_graph::validate_map_terminal_review_ledger()
}
