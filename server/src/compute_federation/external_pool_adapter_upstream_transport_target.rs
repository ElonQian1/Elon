//! Durable inert authority for a Provider-specific brokered upstream transport target.
//!
//! A target records what a future server-owned broker may connect to. It never resolves DNS,
//! opens a socket, performs TLS, probes an upstream, launches a runtime, or activates a Provider.

mod canonical;
mod policy;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use policy::*;
pub(crate) use types::*;
pub(crate) use validation::*;
