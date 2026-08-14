//! Linux x86-64 confinement and lifecycle core for an external-pool Adapter child.
//!
//! This module is deliberately private to `compute_federation`. It launches only a retained,
//! sealed capsule into a dedicated cgroup and isolated namespaces. It does not load production
//! secrets, connect to an upstream, update Provider readiness, or expose an HTTP/MCP surface.

mod cgroup;
mod child;
mod launch;
mod lifecycle;
mod policy;
mod seccomp;

pub(crate) use cgroup::ExternalPoolAdapterSupervisorCgroupParent;
pub(crate) use launch::{
    launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCapsule,
};
pub(crate) use lifecycle::{ExternalPoolAdapterSupervisorChild, ExternalPoolAdapterSupervisorExit};

#[cfg(test)]
mod authenticated_runtime_tests;
#[cfg(test)]
mod linux_tests;
