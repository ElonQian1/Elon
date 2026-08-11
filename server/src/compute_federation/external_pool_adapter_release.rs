//! Platform-declared staging material for an external-pool Adapter release.
//!
//! These DTOs and validators prove only canonical request shape. They do not resolve an artifact,
//! verify an implementation or verifier, construct v213 authority, or grant execution custody.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
