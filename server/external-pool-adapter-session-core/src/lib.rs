#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

//! Shared Linux protocol core for an external-pool Adapter supervisor and its sealed child.
//!
//! The crate owns only ephemeral root binding, mutual bootstrap, authenticated ELSP frames and
//! terminal key cleanup. It does not launch a process, resolve secrets, open an upstream network,
//! update Provider readiness, or create economic effects.

mod bootstrap;
mod crypto;
mod roots;
mod transport;

pub use bootstrap::{
    prepare_external_pool_adapter_supervisor_session, ExternalPoolAdapterChildBootstrap,
    ExternalPoolAdapterHostBootstrap, ExternalPoolAdapterSupervisorDescriptorTransfer,
    PreparedExternalPoolAdapterSupervisorSession,
};
pub use roots::{ExternalPoolAdapterSessionRootArguments, ExternalPoolAdapterSessionRoots};
pub use transport::{
    AuthenticatedExternalPoolAdapterSession, AuthenticatedExternalPoolAdapterSessionFrame,
    ExternalPoolAdapterSessionFrameKind,
};
