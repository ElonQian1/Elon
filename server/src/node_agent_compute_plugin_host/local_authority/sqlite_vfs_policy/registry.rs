//! Sealed ownership kernel for a future one-shot handle-bound SQLite session.
//!
//! The generic owner atomically retains authority-open custody, policy and lifecycle state behind
//! an exact route identity. A process-lifetime wrapper adds OS-random nonce generation and routed
//! callback leases, but no production instance, SQLite callback wiring or VFS registration exists.
//! Terminal quarantine has no transition back and deliberately leaks complete entry custody when
//! close facts are uncertain.

#![allow(dead_code)]

mod file_custody;
mod owner;
mod process_owner;
mod state;
mod types;

pub(in crate::node_agent_compute_plugin_host::local_authority) use file_custody::{
    ComputePluginHandleBoundSqliteAbiFile, HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiFile,
    HandleBoundSqliteAbiLockLevel, HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
    HandleBoundSqliteAbiUnlockLevel,
};
pub(in crate::node_agent_compute_plugin_host::local_authority) use owner::ManagedSqliteRegistryCustody;
pub(in crate::node_agent_compute_plugin_host::local_authority) use process_owner::ManagedSqliteRegistryNonceSource;

#[cfg(test)]
pub(super) use owner::ManagedSqliteRegistryRouteHandle;
#[cfg(test)]
pub(super) use process_owner::ManagedSqliteRegistryProcessOwner;
