//! Provider-neutral Adapter release registry plus inert installed-instance companions.
//!
//! These receipts bind audited supply-chain bytes to a stable global release identity and one
//! Provider-specific installed instance. They do not activate a Provider, create a route, resolve
//! credentials, execute an Adapter, meter work, or settle value.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
