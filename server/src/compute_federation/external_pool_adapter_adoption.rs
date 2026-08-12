//! Revocable adoption authority over exact current V239 and V243 evidence roots.
//!
//! Adoption authorizes a later installation transaction. It does not activate the Provider,
//! register a route, execute work, or create settlement authority.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
