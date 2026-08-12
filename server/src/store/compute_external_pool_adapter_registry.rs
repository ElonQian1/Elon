//! Atomic Provider-neutral Adapter release and installed-instance companion registry.

mod audit;
mod audit_binding_projection;
mod current;
mod persistence;
mod projection;
mod read;
mod targets;
mod types;
mod write;

pub(in crate::store) use current::current_external_pool_adapter_registry_provider_binding_authority_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterRegistryProviderBindingAuthority;
pub(crate) use types::{
    ExternalPoolAdapterRegistryAuditTarget, ExternalPoolAdapterRegistryProviderBindingCurrentness,
    ExternalPoolAdapterRegistryWriteReceipt, RegisterExternalPoolAdapterInstalledInstance,
};
