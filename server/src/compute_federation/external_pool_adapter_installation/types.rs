use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    external_pool_adapter_adoption::ExternalPoolAdapterAdoptionReceipt,
    external_pool_adapter_artifact_package::ExternalPoolAdapterArtifactPackageReceipt,
};

pub(crate) const INSTALLATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_installation_receipt.v1";
pub(crate) const INSTALLATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const INSTALLATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const INSTALLATION_CONFIRMATION: &str = "confirm_external_pool_adapter_installation";
pub(crate) const INSTALLATION_STORAGE_NAMESPACE: &str =
    "compute-federation/external-pool-adapter-artifacts/v1/installed-inert/sha256";
pub(crate) const INSTALLATION_EFFECT: &str = "adapter_bytes_installed_inert";
pub(crate) const INSTALLATION_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstalledExternalPoolAdapterFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterInstallationBinding {
    pub application_id: String,
    pub application_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub declared_implementation_sha256: String,
    pub capability_set_digest: String,
    pub credential_locator_commitment: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub adoption_material_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub runtime_kind: String,
    pub entrypoint_path: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub installation_content_digest: String,
    pub storage_namespace: String,
    pub installed_files: Vec<InstalledExternalPoolAdapterFile>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterInstallationMaterial {
    pub binding: ExternalPoolAdapterInstallationBinding,
    pub installed_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub installed_at: String,
    pub recorded_at: String,
    pub installation_effect: String,
    pub credential_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterInstallationReceipt {
    pub schema: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub installation: ExternalPoolAdapterInstallationMaterial,
}

pub(crate) struct ExternalPoolAdapterInstallationTarget {
    pub adoption_receipt: ExternalPoolAdapterAdoptionReceipt,
    pub package_receipt: ExternalPoolAdapterArtifactPackageReceipt,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
}

/// Non-forgeable proof that exact installed files were reopened and fully rehashed.
///
/// It is intentionally non-Clone/non-Serde and never exposes the local install path.
pub(crate) struct PreparedExternalPoolAdapterInstallation {
    pub(super) binding: ExternalPoolAdapterInstallationBinding,
    pub(super) _reopened_files: Vec<File>,
    pub(super) _pinned_directories: Vec<File>,
    pub(super) _final_root: PathBuf,
}

impl PreparedExternalPoolAdapterInstallation {
    pub(crate) fn binding(&self) -> &ExternalPoolAdapterInstallationBinding {
        &self.binding
    }

    pub(crate) fn installation_content_digest(&self) -> &str {
        &self.binding.installation_content_digest
    }

    pub(crate) fn installed_files(&self) -> &[InstalledExternalPoolAdapterFile] {
        &self.binding.installed_files
    }

    pub(crate) fn storage_namespace(&self) -> &str {
        &self.binding.storage_namespace
    }
}
