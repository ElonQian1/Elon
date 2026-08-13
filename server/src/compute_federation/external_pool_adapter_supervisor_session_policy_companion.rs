//! Durable inert assignment of the server-fixed external-pool supervisor/session contract.
//!
//! This authority records a byte-level and Linux-confinement policy. It never spawns a process,
//! creates IPC, generates session keys, delivers secrets, opens a network connection, or activates
//! a Provider.

mod canonical;
mod policy;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use policy::*;
pub(crate) use types::*;
pub(crate) use validation::*;
