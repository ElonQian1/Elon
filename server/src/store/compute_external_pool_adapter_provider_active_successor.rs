//! Dormant Store-private V274 target, historical row audit, and process-custody seam.
//!
//! There is deliberately no public Store facade. The append/current seams are private to this
//! module, witness-gated, and process-custody-bound. The planned genesis kernel is assembled but
//! has no callable Store orchestrator; durable active observation and restart refresh remain
//! fail-closed until their purpose-specific runtime preparation path exists.

mod append;
mod atomic_activation;
mod audit;
mod preparation;
mod provider_target;
mod read;
mod types;

pub(in crate::store) use append::{
    require_current_external_pool_adapter_provider_active_successor_on,
    CurrentExternalPoolAdapterProviderActiveSuccessorAuthority,
};
pub(in crate::store) use atomic_activation::{
    current_external_pool_adapter_projected_active_historical_carrier_on,
    historical_external_pool_adapter_atomic_activation_authority_on,
    historical_external_pool_adapter_atomic_activation_for_binding_on,
    historical_external_pool_adapter_atomic_activation_for_observed_provider_on,
    CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority,
    HistoricalExternalPoolAdapterAtomicActivationAuthority,
};
pub(in crate::store) use types::PreparedExternalPoolAdapterProviderActiveSuccessorTarget;
