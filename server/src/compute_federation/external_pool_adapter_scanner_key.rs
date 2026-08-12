//! V235 independent trust roots for future signed Adapter vulnerability reports.
//!
//! A current key can verify a future report signature. It does not assert that a scan ran,
//! that an Artifact is vulnerability-free, or that an Adapter may execute.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
