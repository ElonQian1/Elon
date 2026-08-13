//! Store-private composition of current database roots with ephemeral runtime-bundle custody.

mod current;
#[path = "../compute_federation/external_pool_adapter_entrypoint_capsule.rs"]
mod entrypoint_capsule;
#[path = "../compute_federation/external_pool_adapter_runtime_bundle/filesystem.rs"]
mod filesystem;
#[path = "../compute_federation/external_pool_adapter_runtime_bundle/locked_bytes.rs"]
mod locked_bytes;
#[path = "../compute_federation/external_pool_adapter_runtime_bundle/manifest.rs"]
mod manifest;
mod probe_preparation;
mod types;

pub(in crate::store) use types::{
    CurrentExternalPoolAdapterProbePreparationAuthority,
    CurrentExternalPoolAdapterRuntimeBundleAuthority, ExternalPoolAdapterRuntimeBundleRoot,
};
