//! Append-only negative lifecycle facts for staged external-pool Adapter admissions.
//!
//! These contracts can only remove future eligibility. They do not trust artifact bytes, create an
//! Adapter or verifier, authorize a route, or automatically follow a superseding admission.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
