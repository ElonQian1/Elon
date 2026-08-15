//! Dormant Store-private V274 target, historical row audit, and process-custody seam.
//!
//! There is deliberately no Store facade, append path, current authority, active observation
//! producer, or V272 active carrier. V275 must add an opaque atomic activation witness first.

mod audit;
mod preparation;
mod provider_target;
mod read;
mod types;

pub(in crate::store) use preparation::{
    prepare_external_pool_adapter_provider_active_successor_target_on,
    PrepareExternalPoolAdapterProviderActiveSuccessorTarget,
};
pub(in crate::store) use types::PreparedExternalPoolAdapterProviderActiveSuccessorTarget;
