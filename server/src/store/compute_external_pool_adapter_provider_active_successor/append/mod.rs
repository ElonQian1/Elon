//! V274 one-row genesis append, dormant refresh validation, and exact postcommit readback.

mod current;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod genesis;
mod material;
mod readback;
mod refresh;
mod refresh_material;
mod refresh_pending_plan;
mod refresh_postcommit;

pub(in crate::store) use current::{
    external_pool_adapter_provider_active_successor_refresh_needed_on,
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
pub(in crate::store) use refresh::{
    append_external_pool_adapter_provider_active_successor_refresh_on,
    PendingExternalPoolAdapterProviderActiveSuccessorRefresh,
};
pub(in crate::store) use refresh_material::build_external_pool_adapter_provider_active_successor_refresh_material_on;
pub(in crate::store) use refresh_postcommit::postcommit_external_pool_adapter_provider_active_successor_refresh_on;
