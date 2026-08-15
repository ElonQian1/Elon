use std::{collections::BTreeSet, fs::File, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    external_pool_adapter_adoption::ExternalPoolAdapterAdoptionReceipt,
    external_pool_adapter_artifact_package::{
        ExternalPoolAdapterArtifactPackageReceipt, ARTIFACT_PACKAGE_ENTRYPOINT_ROLE,
        ARTIFACT_PACKAGE_RESOURCE_ROLE,
    },
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
pub(crate) const INSTALLATION_TERMINAL_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_installation_terminal_receipt.v1";
pub(crate) const INSTALLATION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_installation_revocation";
pub(crate) const INSTALLATION_TERMINAL_KIND_REVOKED: &str = "revoked";
pub(crate) const INSTALLATION_REVOKED_EFFECT: &str = "installed_instance_revoked";

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterInstallationTerminalMaterial {
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub terminal_kind: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub revoked_at: String,
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
pub(crate) struct ExternalPoolAdapterInstallationTerminalReceipt {
    pub schema: String,
    pub terminal_receipt_id: String,
    pub terminal_receipt_digest: String,
    pub terminal_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub terminal: ExternalPoolAdapterInstallationTerminalMaterial,
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
    pub(super) entrypoint_index: usize,
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

    /// Borrows the exact entrypoint handle retained by the filesystem audit.
    ///
    /// The local installation path is intentionally not part of this seam.
    pub(crate) fn retained_entrypoint(&self) -> anyhow::Result<(&File, &str, u64)> {
        let expected = self
            .binding
            .installed_files
            .get(self.entrypoint_index)
            .ok_or_else(|| anyhow::anyhow!("retained entrypoint inventory is no longer exact"))?;
        let retained = self
            ._reopened_files
            .get(self.entrypoint_index)
            .ok_or_else(|| anyhow::anyhow!("retained entrypoint handle is unavailable"))?;
        if self._reopened_files.len() != self.binding.installed_files.len()
            || expected.role != ARTIFACT_PACKAGE_ENTRYPOINT_ROLE
            || expected.path != self.binding.entrypoint_path
            || expected.sha256 != self.binding.entrypoint_sha256
            || expected.size_bytes != self.binding.entrypoint_size_bytes
        {
            anyhow::bail!("retained entrypoint authority is no longer exact");
        }
        Ok((
            retained,
            &self.binding.entrypoint_sha256,
            self.binding.entrypoint_size_bytes,
        ))
    }

    /// Borrows one exact public resource retained by the installation audit.
    ///
    /// Every call revalidates the full ordered inventory and every retained handle by path, role,
    /// SHA-256, and size. No local path or raw descriptor crosses this seam.
    pub(crate) fn retained_resource(&self, path: &str) -> anyhow::Result<(&File, &str, u64)> {
        if self._reopened_files.len() != self.binding.installed_files.len()
            || path.trim() != path
            || path.is_empty()
        {
            anyhow::bail!("retained resource inventory is no longer exact");
        }
        let mut paths = BTreeSet::new();
        let mut selected = None;
        for (index, (expected, retained)) in self
            .binding
            .installed_files
            .iter()
            .zip(&self._reopened_files)
            .enumerate()
        {
            if !valid_retained_inventory_path(&expected.path)
                || !paths.insert(expected.path.as_str())
                || !matches!(
                    expected.role.as_str(),
                    ARTIFACT_PACKAGE_ENTRYPOINT_ROLE | ARTIFACT_PACKAGE_RESOURCE_ROLE
                )
                || (index == self.entrypoint_index)
                    != (expected.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE)
                || expected.size_bytes == 0
                || (expected.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE
                    && (index != self.entrypoint_index
                        || expected.path != self.binding.entrypoint_path
                        || expected.sha256 != self.binding.entrypoint_sha256
                        || expected.size_bytes != self.binding.entrypoint_size_bytes))
                || retained.metadata()?.len() != expected.size_bytes
                || retained_file_sha256(retained, expected.size_bytes)? != expected.sha256
            {
                anyhow::bail!("retained resource authority is no longer exact");
            }
            if expected.path == path {
                if selected.is_some() || expected.role != ARTIFACT_PACKAGE_RESOURCE_ROLE {
                    anyhow::bail!("retained resource selection is not exact");
                }
                selected = Some((retained, expected.sha256.as_str(), expected.size_bytes));
            }
        }
        selected.ok_or_else(|| anyhow::anyhow!("retained resource is unavailable"))
    }
}

fn valid_retained_inventory_path(path: &str) -> bool {
    path.trim() == path
        && !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|component| !matches!(component, "" | "." | ".."))
}

fn retained_file_sha256(file: &File, expected_size: u64) -> anyhow::Result<String> {
    let before = file.metadata()?;
    if !before.is_file() || before.len() != expected_size {
        anyhow::bail!("retained resource size changed");
    }
    let mut digest = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < expected_size {
        let wanted = usize::try_from((expected_size - offset).min(buffer.len() as u64))?;
        let read = read_retained_at(file, &mut buffer[..wanted], offset)?;
        if read == 0 {
            anyhow::bail!("retained resource ended early");
        }
        digest.update(&buffer[..read]);
        offset = offset
            .checked_add(read as u64)
            .ok_or_else(|| anyhow::anyhow!("retained resource size overflow"))?;
    }
    if read_retained_at(file, &mut buffer[..1], expected_size)? != 0
        || file.metadata()?.len() != before.len()
    {
        anyhow::bail!("retained resource changed during rehash");
    }
    Ok(hex::encode(digest.finalize()))
}

#[cfg(unix)]
fn read_retained_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buffer, offset)
}

#[cfg(windows)]
fn read_retained_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buffer, offset)
}
