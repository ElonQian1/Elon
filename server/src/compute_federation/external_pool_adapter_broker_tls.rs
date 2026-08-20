//! Server-owned direct TLS transport for one exact V258 upstream target.
//!
//! This module performs DNS, public-address validation, direct TCP and TLS identity validation.
//! It exposes only bounded, purpose-specific exchanges and never receives Adapter secrets.

mod address_policy;
mod no_work;
mod target;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod task_protocol;
mod task_protocol_types;
mod transport;

pub(crate) use no_work::exchange_external_pool_adapter_broker_no_work;
pub(crate) use target::ExternalPoolAdapterBrokerTlsTarget;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use task_protocol::exchange_external_pool_adapter_broker_task;
pub(crate) use task_protocol_types::{
    ExternalPoolAdapterBrokerTaskObservationValidator,
    ExternalPoolAdapterBrokerTaskVerifiedObservation,
    VerifiedExternalPoolAdapterBrokerTaskExchange,
};
pub(crate) use transport::{
    connect_external_pool_adapter_broker_tls, ExternalPoolAdapterBrokerTlsChannel,
};

#[cfg(test)]
mod tests;
