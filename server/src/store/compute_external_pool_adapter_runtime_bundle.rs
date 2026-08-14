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
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod secret_delivery;
mod types;

pub(in crate::store) use types::{
    CurrentExternalPoolAdapterProbePreparationAuthority,
    CurrentExternalPoolAdapterRuntimeBundleAuthority, ExternalPoolAdapterRuntimeBundleRoot,
};

pub(in crate::store) fn external_pool_adapter_entrypoint_capsule_policy_root(
) -> anyhow::Result<(String, u64, String)> {
    let root = entrypoint_capsule::external_pool_adapter_entrypoint_capsule_policy_root()?;
    Ok((
        root.policy_id.to_string(),
        root.policy_revision,
        root.policy_digest,
    ))
}
