//! V274 one-row genesis append, dormant refresh validation, and exact postcommit readback.

mod current;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod genesis;
mod material;
mod readback;
mod refresh;

pub(in crate::store) use current::{
    require_current_external_pool_adapter_provider_active_successor_on,
    CurrentExternalPoolAdapterProviderActiveSuccessorAuthority,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) use genesis::{
    insert_prepared_external_pool_adapter_provider_active_successor_genesis_on,
    prepare_external_pool_adapter_provider_active_successor_genesis_append_on,
};
pub(super) use material::PendingExternalPoolAdapterProviderActiveSuccessorAppend;
pub(super) use readback::{
    postcommit_external_pool_adapter_provider_active_successor_readback_on,
    CommittedExternalPoolAdapterProviderActiveSuccessorAppend,
};
