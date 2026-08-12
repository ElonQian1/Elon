//! Independent V241 registry for external-pool credential-verifier identities.
//!
//! Registration and activation make an exact verifier implementation reference current. They do
//! not inspect credentials, store bearer material, issue verification receipts, adopt an Adapter,
//! authorize a route, or execute work.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
