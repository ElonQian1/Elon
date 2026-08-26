//! Linear Windows Runner process-custody prerequisite.
//!
//! This private boundary can only hold a Runner created suspended with sealed restricted launch
//! security and atomically attached to a query-verified kill-on-close Job Object. A neighboring
//! source-only contract now defines the single loader-locked admission successor, but its hard
//! resolution/namespace prerequisites and successful reopen producer remain unavailable; launch
//! security also has no producer. The OS path therefore stays unreachable. It does not resume a
//! thread, write runtime state, authenticate IPC, mint health or Ready authority, or produce any
//! market effect.

#![allow(dead_code)]

#[cfg(windows)]
mod encoding;
#[cfg(windows)]
mod launch_security;
#[cfg(windows)]
mod model;
#[cfg(windows)]
mod namespace_query;
#[cfg(windows)]
mod policy;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windows_job;
#[cfg(windows)]
mod windows_rollback;

#[cfg(windows)]
pub(in crate::node_agent_compute_plugin_host) use model::PreparedComputePluginRunnerProcess;
