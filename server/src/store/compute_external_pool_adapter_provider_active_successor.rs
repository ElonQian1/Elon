//! Store-private V274 target, historical row audit, and process-custody seam.
//!
//! There is deliberately no public Store facade. The append/current seams are private to this
//! module, witness-gated, and process-custody-bound. Linux genesis is callable only by the
//! purpose-specific active-preparation orchestrator after real no-work and V272 reproof.

mod append;
mod atomic_activation;
mod audit;
mod preparation;
mod provider_target;
mod read;
mod types;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use append::build_external_pool_adapter_provider_active_successor_refresh_material_on;
pub(in crate::store) use append::{
    append_external_pool_adapter_provider_active_successor_refresh_on,
    external_pool_adapter_provider_active_successor_refresh_needed_on,
    postcommit_external_pool_adapter_provider_active_successor_refresh_on,
    require_current_external_pool_adapter_provider_active_successor_on,
    CurrentExternalPoolAdapterProviderActiveSuccessorAuthority,
    PendingExternalPoolAdapterProviderActiveSuccessorRefresh,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use atomic_activation::{
    build_external_pool_adapter_atomic_activation_genesis_on,
    finalize_external_pool_adapter_atomic_activation_after_commit_on,
    persist_external_pool_adapter_atomic_activation_closure_on,
    BuiltExternalPoolAdapterAtomicActivationGenesis,
};
pub(in crate::store) use atomic_activation::{
    current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
    current_external_pool_adapter_renewed_route_runtime_carrier_on,
    historical_external_pool_adapter_atomic_activation_authority_on,
    historical_external_pool_adapter_atomic_activation_for_binding_on,
    historical_external_pool_adapter_atomic_activation_for_observed_provider_on,
    historical_external_pool_adapter_atomic_activation_history_for_binding_on,
    CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
    HistoricalExternalPoolAdapterAtomicActivationAuthority,
};
pub(in crate::store) use preparation::{
    prepare_external_pool_adapter_provider_active_successor_target_on,
    reprove_external_pool_adapter_provider_active_successor_target_on,
    PrepareExternalPoolAdapterProviderActiveSuccessorTarget,
};
pub(in crate::store) use types::PreparedExternalPoolAdapterProviderActiveSuccessorTarget;
