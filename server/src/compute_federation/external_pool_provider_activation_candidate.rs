//! Inert external-pool activation candidates and owner delegations.
//!
//! These records reserve a Provider-specific logical-to-route identity and an owner-approved
//! platform service actor. They do not activate a Provider or create v213, market, or execution
//! authority.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
