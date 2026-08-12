//! Inert, content-addressed installation of one adopted external-pool Adapter package.
//!
//! Installation materializes exact V232 package bytes under a server-owned DATA_DIR namespace.
//! It never resolves credentials, starts a process, opens the network, activates a Provider, or
//! grants route/execution/settlement authority.

mod canonical;
mod filesystem;
mod types;
mod validation;

#[cfg(test)]
#[path = "external_pool_adapter_installation/filesystem_test.rs"]
mod filesystem_test;

pub(crate) use canonical::*;
pub(crate) use filesystem::*;
pub(crate) use types::*;
pub(crate) use validation::*;
