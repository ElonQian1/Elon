use crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt;

/// Exact provider-neutral V249 receipt. Reusing the authoritative type prevents a partial root
/// projection whenever V249 is deserialized, validated, or canonically bound by V268.
pub(crate) type ExternalPoolAdapterRuntimeCompatibilityRegistryReleaseRoots =
    ExternalPoolAdapterRegistryReleaseReceipt;
