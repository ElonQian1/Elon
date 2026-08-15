//! Transaction-independent sealed capsule preparation for child launch.

use anyhow::Result;

use super::super::{
    entrypoint_capsule::{
        external_pool_adapter_entrypoint_capsule_policy_root,
        prepare_external_pool_adapter_entrypoint_capsule,
        PreparedExternalPoolAdapterEntrypointCapsule,
    },
    types::{
        CurrentExternalPoolAdapterProbePreparationAuthority,
        CurrentExternalPoolAdapterRuntimeBundleAuthority,
    },
};
use super::{
    audit_capsule, prepared_entrypoint, recheck_callback_freshness, RetainedEntrypointSource,
};

/// Produces a transaction-independent sealed launch image after validating the exact preparation.
///
/// The returned capsule owns only anonymous sealed files. It retains neither the source
/// installation audit nor any database authority and is therefore safe to use after the owning
/// transaction commits.
pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn prepare_owned_probe_capsule(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    selected: &super::CurrentExternalPoolAdapterProbePreparationRoots,
) -> Result<PreparedExternalPoolAdapterEntrypointCapsule> {
    let source = RetainedEntrypointSource {
        prepared: prepared_entrypoint(bundle),
    };
    bundle.revalidate()?;
    let capsule = prepare_external_pool_adapter_entrypoint_capsule(&source)?;
    let policy = external_pool_adapter_entrypoint_capsule_policy_root()?;
    audit_capsule(bundle, selected, &capsule, &policy)?;
    bundle.revalidate()?;
    recheck_callback_freshness(bundle, selected)?;
    Ok(capsule)
}

pub(in crate::store::compute_external_pool_adapter_runtime_bundle) fn with_owned_probe_preparation(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    selected: &super::CurrentExternalPoolAdapterProbePreparationRoots,
    capsule: &PreparedExternalPoolAdapterEntrypointCapsule,
    consume: impl FnOnce(&CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>) -> Result<()>,
) -> Result<()> {
    let policy = external_pool_adapter_entrypoint_capsule_policy_root()?;
    audit_capsule(bundle, selected, capsule, &policy)?;
    bundle.revalidate()?;
    let authority = CurrentExternalPoolAdapterProbePreparationAuthority::new(
        capsule,
        &selected.vulnerability,
        &selected.sandbox,
        bundle,
        policy.policy_id,
        policy.policy_revision,
        &policy.policy_digest,
    );
    recheck_callback_freshness(bundle, selected)?;
    consume(&authority)?;
    bundle.revalidate()
}
