//! Canonical, API-free bridge from an exercised `capacity_future` allocation to retained
//! execution-verification and Attempt-settlement lineage.
//!
//! The bridge is an untrusted source projection and derived reference material only. It does not create capacity or
//! verify usage, settle money, release balances, classify delivery, or prove native owner digests by itself.

mod canonical;
mod settlement_equations;
mod source_equations;
mod source_inputs;
mod source_support;
mod types;
mod validation;

pub(crate) use canonical::{
    canonical_compute_capacity_future_settlement_lineage_json_and_digest,
    compute_capacity_future_settlement_lineage_from_json,
};
pub(crate) use source_equations::{
    build_compute_capacity_future_settlement_lineage,
    validate_compute_capacity_future_settlement_lineage_against_sources,
};
pub(crate) use source_inputs::{
    ComputeCapacityFutureSettlementLineageSources, ComputeCapacityFutureSettlementStageSources,
    UntrustedCapacityFutureAttemptSettlementAuditView,
};
pub(crate) use types::*;
pub(crate) use validation::validate_compute_capacity_future_settlement_lineage;
