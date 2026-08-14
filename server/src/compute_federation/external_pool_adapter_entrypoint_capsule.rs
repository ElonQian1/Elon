//! Store-private, ephemeral executable materialization for an external-pool Adapter.
//!
//! This source is deliberately absent from `compute_federation::mod`. The Store includes it as
//! a private child next to the V256 runtime-bundle authority. It never launches or executes the
//! image and never accepts a filesystem path from its caller.

use anyhow::Result;
use std::fs::File;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "external_pool_adapter_entrypoint_capsule/elf.rs"]
mod elf;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "external_pool_adapter_entrypoint_capsule/linux.rs"]
mod linux;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "external_pool_adapter_entrypoint_capsule/linux_tests.rs"]
mod linux_tests;
#[path = "external_pool_adapter_entrypoint_capsule/policy.rs"]
mod policy;
#[path = "external_pool_adapter_entrypoint_capsule/types.rs"]
mod types;

pub(super) use types::{
    PreparedExternalPoolAdapterEntrypointCapsule, ACTIVATION_READY, ENTRYPOINT_CAPSULE_EFFECT,
    PROBE_OBSERVED, RUNTIME_LAUNCH_READY,
};

pub(super) struct ExternalPoolAdapterEntrypointCapsulePolicyRoot {
    pub(super) policy_id: &'static str,
    pub(super) policy_revision: u64,
    pub(super) policy_digest: String,
}

pub(super) fn external_pool_adapter_entrypoint_capsule_policy_root(
) -> Result<ExternalPoolAdapterEntrypointCapsulePolicyRoot> {
    let policy = policy::entrypoint_capsule_policy()
        .map_err(|_| anyhow::anyhow!("external-pool Adapter capsule policy is unavailable"))?;
    Ok(ExternalPoolAdapterEntrypointCapsulePolicyRoot {
        policy_id: policy.policy_id,
        policy_revision: policy.policy_revision,
        policy_digest: policy.policy_digest,
    })
}

/// The only input seam: one already-retained entrypoint handle and its Store-derived identity.
///
/// Implementations live inside the Store authority. A path, raw descriptor, executable name, or
/// caller-selected policy cannot cross this seam.
pub(super) trait ExternalPoolAdapterEntrypointSource {
    fn retained_entrypoint(&self) -> Result<(&File, &str, u64)>;
}

/// Materializes one capsule and keeps it borrowed for exactly one Store-owned callback.
pub(super) fn with_external_pool_adapter_entrypoint_capsule(
    source: &impl ExternalPoolAdapterEntrypointSource,
    consume: impl FnOnce(&PreparedExternalPoolAdapterEntrypointCapsule) -> Result<()>,
) -> Result<()> {
    let capsule = platform_materialize(source)
        .map_err(|_| anyhow::anyhow!("external-pool Adapter entrypoint capsule is unavailable"))?;
    consume(&capsule)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn platform_materialize(
    source: &impl ExternalPoolAdapterEntrypointSource,
) -> std::result::Result<
    PreparedExternalPoolAdapterEntrypointCapsule,
    types::ExternalPoolAdapterEntrypointCapsuleError,
> {
    linux::materialize(source)
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn platform_materialize(
    _source: &impl ExternalPoolAdapterEntrypointSource,
) -> std::result::Result<
    PreparedExternalPoolAdapterEntrypointCapsule,
    types::ExternalPoolAdapterEntrypointCapsuleError,
> {
    Err(types::ExternalPoolAdapterEntrypointCapsuleError::Unavailable)
}
