//! Verifier-signed dynamic sandbox evidence for one exact external-pool Adapter artifact.
//!
//! The server derives the six-capability test plan from the immutable V222 admission and binds
//! signed observations to the current V236 artifact-security chain and one active V237 verifier.
//! This evidence does not execute bytes or grant Adapter, credential, route, or settlement power.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
