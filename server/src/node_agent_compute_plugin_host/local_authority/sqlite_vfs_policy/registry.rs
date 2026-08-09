//! Sealed ownership kernel for a future one-shot handle-bound SQLite session.
//!
//! The generic owner atomically retains authority-open custody, policy and lifecycle state behind
//! an exact route identity. No production registry instance, nonce source, callback or VFS
//! registration exists yet. Terminal quarantine has no transition back and deliberately leaks
//! complete entry custody for the process lifetime when close facts are uncertain.

#![allow(dead_code)]

mod owner;
mod state;
mod types;

pub(in crate::node_agent_compute_plugin_host::local_authority) use owner::ManagedSqliteRegistryCustody;
