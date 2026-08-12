use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    external_pool_adapter_artifact_package::ExternalPoolAdapterArtifactManifest,
    external_pool_adapter_release::{
        ComputeExternalPoolAdapterReleaseCapability,
        ComputeExternalPoolAdapterReleaseVerifierIntent,
    },
};

pub(crate) const REGISTRY_RELEASE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_registry_release_receipt.v1";
pub(crate) const REGISTRY_PROVIDER_BINDING_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_registry_provider_binding_receipt.v1";
pub(crate) const REGISTRY_PROVIDER_BINDING_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_registry_provider_binding_currentness.v1";
pub(crate) const REGISTRY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const REGISTRY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const REGISTRY_BINDING_CONFIRMATION: &str =
    "confirm_external_pool_adapter_registry_binding";
pub(crate) const REGISTRY_RELEASE_EFFECT: &str = "provider_neutral_release_registered";
pub(crate) const REGISTRY_BINDING_EFFECT: &str = "installed_instance_companion_recorded";
pub(crate) const REGISTRY_NO_EFFECT: &str = "none";

/// Global release material. Provider, adoption, installation, credential-location, actor, admin,
/// idempotency and observation-time facts are deliberately absent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRegistryReleaseMaterial {
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    /// Exact executable identity; V249 requires this to equal both the V222 declaration and
    /// the V227/V232 archive SHA-256.
    pub implementation_digest: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub credential_verifier_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest: ExternalPoolAdapterArtifactManifest,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub installation_content_digest: String,
    pub registered_at: String,
    pub recorded_at: String,
    pub registry_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRegistryReleaseReceipt {
    pub schema: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub release: ExternalPoolAdapterRegistryReleaseMaterial,
}

/// One Provider's immutable companion to a global release. The route projection ID is only a
/// reserved opaque identity; recording it grants no route or dispatch authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRegistryProviderBindingMaterial {
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub route_adapter_projection_id: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_material_digest: String,
    pub installation_content_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub adoption_material_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub credential_locator_commitment: String,
    pub bound_by_admin_user_id: String,
    pub confirmation: String,
    pub checked_at: String,
    pub bound_at: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub registry_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRegistryProviderBindingReceipt {
    pub schema: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_binding_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub binding: ExternalPoolAdapterRegistryProviderBindingMaterial,
}
