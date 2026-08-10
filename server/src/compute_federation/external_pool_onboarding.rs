//! Owner-declared material for the first external-pool onboarding boundary.
//!
//! These DTOs and validators are not Provider, Adapter, credential, route, or v213 authority.
//! Persistence, review, immutable apply, and all validated wrappers remain outside this module.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
