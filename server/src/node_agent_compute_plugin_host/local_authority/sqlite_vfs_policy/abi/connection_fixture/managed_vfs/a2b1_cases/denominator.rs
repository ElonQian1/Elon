//! A2a/A2b1 Map/Lock review scaffolds and exact static source-leaf authority.
//!
//! The older fragment tables remain review aids and do not contribute to a denominator. The
//! adjacent `static_contract` module roots each callback in one acyclic production-source graph,
//! freezes the exact included and excluded leaf sets, binds ordered witnesses and Expected
//! custody, and validates source-universe equality before reporting Map or Lock StaticContract.
//! Windows dynamic evidence remains outside this module.

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
mod static_contract;
mod typed_map_fragment;

pub(super) use static_contract::{CapabilityGapV1, DynamicQuotientCandidateGateErrorV1};

pub(super) fn validate_candidate_branch_atom_scaffold() -> Result<(), &'static str> {
    invariants::validate()
}

pub(super) fn validate_source_owner_graph() -> Result<(), &'static str> {
    source_owner_graph::validate_source_owner_graph()
}

pub(super) fn validate_map_terminal_review_ledger() -> Result<(), &'static str> {
    source_owner_graph::validate_map_terminal_review_ledger()
}

pub(super) fn validate_map_static_contract() -> Result<usize, String> {
    static_contract::validate_map()
}

pub(super) fn validate_lock_static_contract() -> Result<usize, String> {
    static_contract::validate_lock()
}

pub(super) fn validate_map_lock_static_contract() -> Result<(usize, usize), String> {
    static_contract::validate_all()
}

pub(super) fn validate_map_dynamic_quotient_candidate_gate(
) -> Result<(), DynamicQuotientCandidateGateErrorV1> {
    static_contract::validate_map_dynamic_quotient_candidate_gate()
}

pub(super) fn validate_lock_dynamic_quotient_candidate_gate(
) -> Result<(), DynamicQuotientCandidateGateErrorV1> {
    static_contract::validate_lock_dynamic_quotient_candidate_gate()
}
