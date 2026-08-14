//! Server-owned direct TLS transport for one exact V258 upstream target.
//!
//! This module performs DNS, public-address validation, direct TCP and TLS identity validation.
//! It deliberately exposes no application read/write API and never receives Adapter secrets.

mod address_policy;
mod target;
mod transport;

pub(crate) use target::ExternalPoolAdapterBrokerTlsTarget;
pub(crate) use transport::{
    connect_external_pool_adapter_broker_tls, ExternalPoolAdapterBrokerTlsChannel,
};

#[cfg(test)]
mod tests;
