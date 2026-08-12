use serde::Serialize;

use crate::compute_federation::{
    external_pool_adapter_installation::{
        ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
    },
    external_pool_adapter_registry::{
        ExternalPoolAdapterRegistryProviderBindingReceipt,
        ExternalPoolAdapterRegistryReleaseReceipt,
    },
};

pub(crate) struct RegisterExternalPoolAdapterInstalledInstance {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub bound_by_admin_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct ExternalPoolAdapterRegistryAuditTarget {
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_binding: ExternalPoolAdapterInstallationBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRegistryReleaseSummary {
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub installation_content_digest: String,
    pub registered_at: String,
    pub registry_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRegistryProviderBindingSummary {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_binding_material_digest: String,
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
    pub provider_id: String,
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
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub checked_at: String,
    pub bound_at: String,
    pub registry_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRegistryWriteReceipt {
    pub release: ExternalPoolAdapterRegistryReleaseSummary,
    pub binding: ExternalPoolAdapterRegistryProviderBindingSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRegistryProviderBindingCurrentness {
    pub schema: &'static str,
    pub release: ExternalPoolAdapterRegistryReleaseSummary,
    pub binding: ExternalPoolAdapterRegistryProviderBindingSummary,
    pub current_status: String,
    pub release_status: String,
    pub admission_status: String,
    pub package_status: String,
    pub source_status: String,
    pub adoption_terminal_status: String,
    pub installation_terminal_status: String,
    pub provider_status: String,
    pub file_inventory_status: String,
    pub route_projection_status: String,
}

pub(super) struct StoredRegistryRelease {
    pub receipt: ExternalPoolAdapterRegistryReleaseReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredRegistryProviderBinding {
    pub receipt: ExternalPoolAdapterRegistryProviderBindingReceipt,
    pub receipt_json: String,
}

/// Store-only current proof retaining V247's reopened and rehashed file handles.
pub(in crate::store) struct CurrentExternalPoolAdapterRegistryProviderBindingAuthority {
    release: ExternalPoolAdapterRegistryReleaseReceipt,
    binding: ExternalPoolAdapterRegistryProviderBindingReceipt,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: String,
}

impl CurrentExternalPoolAdapterRegistryProviderBindingAuthority {
    pub(super) fn new(
        release: ExternalPoolAdapterRegistryReleaseReceipt,
        binding: ExternalPoolAdapterRegistryProviderBindingReceipt,
        prepared: PreparedExternalPoolAdapterInstallation,
        checked_at: String,
    ) -> Self {
        Self {
            release,
            binding,
            prepared,
            checked_at,
        }
    }
    pub(in crate::store) fn release(&self) -> &ExternalPoolAdapterRegistryReleaseReceipt {
        &self.release
    }
    pub(in crate::store) fn binding(&self) -> &ExternalPoolAdapterRegistryProviderBindingReceipt {
        &self.binding
    }
    pub(in crate::store) fn prepared(&self) -> &PreparedExternalPoolAdapterInstallation {
        &self.prepared
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredRegistryRelease {
    pub(super) fn summary(&self) -> ExternalPoolAdapterRegistryReleaseSummary {
        let receipt = &self.receipt;
        let item = &receipt.release;
        ExternalPoolAdapterRegistryReleaseSummary {
            registry_release_id: receipt.registry_release_id.clone(),
            registry_release_digest: receipt.registry_release_digest.clone(),
            registry_release_material_digest: receipt.registry_release_material_digest.clone(),
            admission_id: item.admission_id.clone(),
            admission_digest: item.admission_digest.clone(),
            package_receipt_id: item.package_receipt_id.clone(),
            package_receipt_digest: item.package_receipt_digest.clone(),
            source_receipt_id: item.source_receipt_id.clone(),
            source_receipt_digest: item.source_receipt_digest.clone(),
            adapter_id: item.adapter_id.clone(),
            release_version: item.release_version.clone(),
            route_kind: item.route_kind.clone(),
            supported_provider_kinds: item.supported_provider_kinds.clone(),
            implementation_digest: item.implementation_digest.clone(),
            capability_set_digest: item.capability_set_digest.clone(),
            credential_verifier_digest: item.credential_verifier_digest.clone(),
            archive_sha256: item.archive_sha256.clone(),
            archive_size_bytes: item.archive_size_bytes,
            manifest_digest: item.manifest_digest.clone(),
            entry_inventory_digest: item.entry_inventory_digest.clone(),
            entry_count: item.entry_count,
            total_uncompressed_bytes: item.total_uncompressed_bytes,
            installation_content_digest: item.installation_content_digest.clone(),
            registered_at: item.registered_at.clone(),
            registry_effect: item.registry_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

impl StoredRegistryProviderBinding {
    pub(super) fn summary(&self) -> ExternalPoolAdapterRegistryProviderBindingSummary {
        let receipt = &self.receipt;
        let item = &receipt.binding;
        ExternalPoolAdapterRegistryProviderBindingSummary {
            provider_binding_id: receipt.provider_binding_id.clone(),
            provider_binding_digest: receipt.provider_binding_digest.clone(),
            provider_binding_material_digest: receipt.provider_binding_material_digest.clone(),
            registry_release_id: item.registry_release_id.clone(),
            registry_release_digest: item.registry_release_digest.clone(),
            route_adapter_projection_id: item.route_adapter_projection_id.clone(),
            installation_receipt_id: item.installation_receipt_id.clone(),
            installation_receipt_digest: item.installation_receipt_digest.clone(),
            installation_material_digest: item.installation_material_digest.clone(),
            installation_content_digest: item.installation_content_digest.clone(),
            application_id: item.application_id.clone(),
            application_digest: item.application_digest.clone(),
            adoption_receipt_id: item.adoption_receipt_id.clone(),
            adoption_receipt_digest: item.adoption_receipt_digest.clone(),
            provider_id: item.provider_id.clone(),
            provider_policy_revision: item.provider_policy_revision,
            provider_digest: item.provider_digest.clone(),
            adapter_id: item.adapter_id.clone(),
            release_version: item.release_version.clone(),
            adapter_config_revision: item.adapter_config_revision,
            adapter_config_digest: item.adapter_config_digest.clone(),
            admission_id: item.admission_id.clone(),
            admission_digest: item.admission_digest.clone(),
            package_receipt_id: item.package_receipt_id.clone(),
            package_receipt_digest: item.package_receipt_digest.clone(),
            source_receipt_id: item.source_receipt_id.clone(),
            source_receipt_digest: item.source_receipt_digest.clone(),
            sandbox_conformance_receipt_id: item.sandbox_conformance_receipt_id.clone(),
            sandbox_conformance_receipt_digest: item.sandbox_conformance_receipt_digest.clone(),
            credential_verification_receipt_id: item.credential_verification_receipt_id.clone(),
            credential_verification_receipt_digest: item
                .credential_verification_receipt_digest
                .clone(),
            checked_at: item.checked_at.clone(),
            bound_at: item.bound_at.clone(),
            registry_effect: item.registry_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}
