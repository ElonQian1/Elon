use serde::Serialize;

use crate::compute_federation::external_pool_adapter_installation::{
    ExternalPoolAdapterInstallationReceipt, ExternalPoolAdapterInstallationTerminalReceipt,
    InstalledExternalPoolAdapterFile, PreparedExternalPoolAdapterInstallation,
};
use sha2::{Digest, Sha256};

pub(crate) struct InstallExternalPoolAdapter {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub expected_adoption_receipt_digest: String,
    pub expected_package_receipt_digest: String,
    pub expected_source_receipt_digest: String,
    pub installed_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterInstallation {
    pub installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterInstallationSummary {
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_material_digest: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub runtime_kind: String,
    pub entrypoint_path_digest: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub installation_content_digest: String,
    pub storage_namespace: String,
    pub installed_file_count: u64,
    pub installed_total_bytes: u64,
    pub installed_by_admin_user_id: String,
    pub installed_at: String,
    pub installation_effect: String,
    pub credential_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterInstallationWriteReceipt {
    pub installation: ExternalPoolAdapterInstallationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterInstallationTerminalSummary {
    pub terminal_receipt_id: String,
    pub terminal_receipt_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub terminal_kind: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub installation_effect: String,
    pub credential_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterInstallationTerminalWriteReceipt {
    pub installation: ExternalPoolAdapterInstallationSummary,
    pub terminal: ExternalPoolAdapterInstallationTerminalSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterInstallationCurrentness {
    pub schema: &'static str,
    pub installation: ExternalPoolAdapterInstallationSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<ExternalPoolAdapterInstallationTerminalSummary>,
    pub current_status: String,
    pub adoption_status: String,
    pub package_status: String,
    pub source_status: String,
    pub file_inventory_status: String,
    pub terminal_status: String,
}

pub(super) struct StoredExternalPoolAdapterInstallation {
    pub receipt: ExternalPoolAdapterInstallationReceipt,
    pub receipt_json: String,
    pub files: Vec<InstalledExternalPoolAdapterFile>,
}

pub(super) struct StoredExternalPoolAdapterInstallationTerminal {
    pub receipt: ExternalPoolAdapterInstallationTerminalReceipt,
    pub receipt_json: String,
}

/// Sealed, non-Clone/non-Serde proof of one exact current installation.
///
/// The prepared proof pins every audited installed file and directory for this
/// authority's lifetime. Only Store code can construct it.
pub(in crate::store) struct CurrentExternalPoolAdapterInstallationAuthority {
    receipt: ExternalPoolAdapterInstallationReceipt,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: String,
}

pub(in crate::store) struct HistoricalExternalPoolAdapterInstallationAuthority {
    receipt: ExternalPoolAdapterInstallationReceipt,
}

impl CurrentExternalPoolAdapterInstallationAuthority {
    pub(super) fn new(
        receipt: ExternalPoolAdapterInstallationReceipt,
        prepared: PreparedExternalPoolAdapterInstallation,
        checked_at: String,
    ) -> Self {
        Self {
            receipt,
            prepared,
            checked_at,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterInstallationReceipt {
        &self.receipt
    }

    pub(in crate::store) fn prepared(&self) -> &PreparedExternalPoolAdapterInstallation {
        &self.prepared
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl HistoricalExternalPoolAdapterInstallationAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterInstallationReceipt) -> Self {
        Self { receipt }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterInstallationReceipt {
        &self.receipt
    }
}

impl StoredExternalPoolAdapterInstallation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterInstallationSummary {
        let receipt = &self.receipt;
        let item = &receipt.installation;
        let binding = &item.binding;
        ExternalPoolAdapterInstallationSummary {
            installation_receipt_id: receipt.installation_receipt_id.clone(),
            installation_receipt_digest: receipt.installation_receipt_digest.clone(),
            installation_material_digest: receipt.installation_material_digest.clone(),
            adoption_receipt_id: binding.adoption_receipt_id.clone(),
            adoption_receipt_digest: binding.adoption_receipt_digest.clone(),
            application_id: binding.application_id.clone(),
            application_digest: binding.application_digest.clone(),
            provider_id: binding.provider_id.clone(),
            provider_policy_revision: binding.provider_policy_revision,
            provider_digest: binding.provider_digest.clone(),
            admission_id: binding.admission_id.clone(),
            admission_digest: binding.admission_digest.clone(),
            adapter_id: binding.adapter_id.clone(),
            adapter_release_version: binding.adapter_release_version.clone(),
            adapter_config_revision: binding.adapter_config_revision,
            adapter_config_digest: binding.adapter_config_digest.clone(),
            package_receipt_id: binding.package_receipt_id.clone(),
            package_receipt_digest: binding.package_receipt_digest.clone(),
            source_receipt_id: binding.source_receipt_id.clone(),
            source_receipt_digest: binding.source_receipt_digest.clone(),
            archive_sha256: binding.archive_sha256.clone(),
            archive_size_bytes: binding.archive_size_bytes,
            manifest_digest: binding.manifest_digest.clone(),
            entry_inventory_digest: binding.entry_inventory_digest.clone(),
            entry_count: binding.entry_count,
            total_uncompressed_bytes: binding.total_uncompressed_bytes,
            runtime_kind: binding.runtime_kind.clone(),
            entrypoint_path_digest: path_digest(&binding.entrypoint_path),
            entrypoint_sha256: binding.entrypoint_sha256.clone(),
            entrypoint_size_bytes: binding.entrypoint_size_bytes,
            installation_content_digest: binding.installation_content_digest.clone(),
            storage_namespace: binding.storage_namespace.clone(),
            installed_file_count: self.files.len() as u64,
            installed_total_bytes: self.files.iter().map(|file| file.size_bytes).sum(),
            installed_by_admin_user_id: item.installed_by_admin_user_id.clone(),
            installed_at: item.installed_at.clone(),
            installation_effect: item.installation_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

impl StoredExternalPoolAdapterInstallationTerminal {
    pub(super) fn summary(&self) -> ExternalPoolAdapterInstallationTerminalSummary {
        let receipt = &self.receipt;
        let item = &receipt.terminal;
        ExternalPoolAdapterInstallationTerminalSummary {
            terminal_receipt_id: receipt.terminal_receipt_id.clone(),
            terminal_receipt_digest: receipt.terminal_receipt_digest.clone(),
            installation_receipt_id: item.installation_receipt_id.clone(),
            installation_receipt_digest: item.installation_receipt_digest.clone(),
            terminal_kind: item.terminal_kind.clone(),
            revoked_by_admin_user_id: item.revoked_by_admin_user_id.clone(),
            reason: item.reason.clone(),
            revoked_at: item.revoked_at.clone(),
            installation_effect: item.installation_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

fn path_digest(path: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-PATH-V1");
    digest.update([0]);
    digest.update(path.as_bytes());
    hex::encode(digest.finalize())
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
