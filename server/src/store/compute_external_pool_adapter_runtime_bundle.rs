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
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod no_work_probe;
mod probe_preparation;
mod runtime;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod secret_delivery;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod task_delivery;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod test_materialization;
mod types;

pub(in crate::store) use current::current_external_pool_adapter_runtime_bundle_authority_on;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use no_work_probe::CurrentExternalPoolAdapterNoWorkProbeObservationAuthority;
pub(crate) use runtime::{
    external_pool_adapter_provider_runtime_readiness_runtime,
    initialize_external_pool_adapter_provider_runtime_readiness_runtime,
    verify_pending_external_pool_adapter_provider_active_successor_process_seal,
    ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    ExternalPoolAdapterProviderRuntimeReadinessUnavailable,
};
pub(in crate::store) use runtime::{
    ExternalPoolAdapterProviderActiveSuccessorProcessSeal,
    ExternalPoolAdapterProviderActiveSuccessorProcessSealInput,
    ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    ExternalPoolAdapterTaskProtocolConformanceSealInput, TaskProtocolConformanceProcessSeal,
};
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
pub(crate) use test_materialization::with_materialized_external_pool_adapter_test_capsule;
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
