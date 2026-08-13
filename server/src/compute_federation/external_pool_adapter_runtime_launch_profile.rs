//! Inert Provider-specific Adapter runtime launch-profile authority.
//!
//! A profile records a server-derived future launch contract. It never resolves credentials,
//! starts a process, opens a transport, probes a Provider, or activates compute authority.

mod canonical;
mod policy;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use policy::*;
pub(crate) use types::*;
pub(crate) use validation::*;
