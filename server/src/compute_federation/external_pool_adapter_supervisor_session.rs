//! Ephemeral authenticated child-only session core for a future external-pool supervisor.
//!
//! This module creates anonymous Linux IPC and authenticates bounded frames. It does not launch a
//! process, execute a capsule, deliver production secrets, open a network connection, or change
//! Provider readiness.

mod bootstrap;
mod crypto;
mod roots;
mod transport;

pub(in crate::compute_federation) use bootstrap::{
    prepare_external_pool_adapter_supervisor_session, ExternalPoolAdapterChildBootstrap,
    ExternalPoolAdapterHostBootstrap, PreparedExternalPoolAdapterSupervisorSession,
};
pub(in crate::compute_federation) use roots::ExternalPoolAdapterSessionRoots;
pub(in crate::compute_federation) use transport::{
    AuthenticatedExternalPoolAdapterSession, AuthenticatedExternalPoolAdapterSessionFrame,
    ExternalPoolAdapterSessionFrameKind,
};

#[cfg(test)]
mod linux_tests;
