//! Durable, Provider-specific post-cleanup runtime-readiness evidence.
//!
//! This Domain records an extremely short-lived observation. It does not expose the process
//! custody commitments, activate a Provider, create a route, or grant execution/market authority.

mod builders;
mod canonical;
mod input;
mod policy;
mod summary;
mod types;
mod validation;

pub(crate) use builders::*;
pub(crate) use canonical::*;
pub(crate) use input::*;
pub(crate) use policy::*;
pub(crate) use summary::*;
pub(crate) use types::*;
pub(crate) use validation::*;
