//! Durable V268 provider-neutral runtime-compatibility evidence and Store-private runner.

mod challenge;
mod current;
#[path = "../compute_federation/external_pool_adapter_entrypoint_capsule.rs"]
mod entrypoint_capsule;
mod error;
mod persistence;
mod read;
mod record;
mod revocation;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod run;
mod types;

pub(crate) mod api {
    pub(crate) use super::error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError;
    pub(crate) use super::types::{
        ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt,
        ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt,
        ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt,
    };
}
pub(in crate::store) use current::current_external_pool_adapter_runtime_compatibility_verification_authority_on;
pub(in crate::store) use types::{
    CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
    ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt,
};
