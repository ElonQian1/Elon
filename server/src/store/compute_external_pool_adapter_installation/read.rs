use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_installation::{
        canonical_external_pool_adapter_installation_receipt_json_and_digest,
        validate_external_pool_adapter_installation_receipt, InstalledExternalPoolAdapterFile,
    },
    store::{
        compute_external_pool_adapter_adoption::external_pool_adapter_adoption_receipt_authority_on,
        compute_external_pool_adapter_artifact_package::artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        Store,
    },
};

use super::types::*;

const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_installation_currentness.v1";

pub(super) fn receipt_by_id_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterInstallation>> {
    receipt_on(conn, "installation_receipt_id=?1", params![receipt_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterInstallation>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn receipt_by_adoption_on(
    conn: &Connection,
    adoption_receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterInstallation>> {
    receipt_on(conn, "adoption_receipt_id=?1", params![adoption_receipt_id])
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterInstallation>> {
    conn.query_row(
        &format!(
            "SELECT installation_receipt_id,receipt_json
               FROM compute_external_pool_adapter_installation_receipts WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_id: String = row.get(0)?;
            let receipt_json: String = row.get(1)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(error))
            })?;
            Ok((receipt_id, receipt, receipt_json))
        },
    )
    .optional()?
    .map(|(receipt_id, receipt, receipt_json)| {
        let files = files_on(conn, &receipt_id)?;
        audit_receipt(
            conn,
            StoredExternalPoolAdapterInstallation {
                receipt,
                receipt_json,
                files,
            },
        )
    })
    .transpose()
}

fn files_on(conn: &Connection, receipt_id: &str) -> Result<Vec<InstalledExternalPoolAdapterFile>> {
    let mut statement = conn.prepare(
        "SELECT ordinal,path,sha256,size_bytes,role
           FROM compute_external_pool_adapter_installation_files
          WHERE installation_receipt_id=?1 ORDER BY ordinal",
    )?;
    let rows = statement.query_map([receipt_id], |row| {
        let ordinal: i64 = row.get(0)?;
        if ordinal < 0 {
            return Err(rusqlite::Error::IntegralValueOutOfRange(0, ordinal));
        }
        Ok((
            ordinal as usize,
            InstalledExternalPoolAdapterFile {
                path: row.get(1)?,
                sha256: row.get(2)?,
                size_bytes: row.get::<_, i64>(3)? as u64,
                role: row.get(4)?,
            },
        ))
    })?;
    let mut files = Vec::new();
    for row in rows {
        let (ordinal, file) = row?;
        if ordinal != files.len() {
            bail!("Adapter installation file ordinals are not contiguous");
        }
        files.push(file);
    }
    Ok(files)
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredExternalPoolAdapterInstallation,
) -> Result<StoredExternalPoolAdapterInstallation> {
    validate_external_pool_adapter_installation_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_installation_receipt_json_and_digest(&stored.receipt)?;
    let binding = &stored.receipt.installation.binding;
    let adoption = external_pool_adapter_adoption_receipt_authority_on(
        conn,
        &binding.adoption_receipt_id,
        &binding.adoption_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost adoption root"))?;
    let package = artifact_package_authority_on(
        conn,
        &binding.admission_id,
        &binding.package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost package root"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &binding.admission_id,
        &binding.admission_digest,
        &binding.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter installation lost source root"))?;
    let adoption_binding = &adoption.receipt().adoption.binding;
    let package_receipt = package.receipt();
    let package_item = &package_receipt.package;
    let package_files_are_exact = binding.installed_files.len()
        == package_item.manifest.files.len()
        && binding
            .installed_files
            .iter()
            .zip(&package_item.manifest.files)
            .all(|(installed, declared)| {
                installed.path == declared.path
                    && installed.sha256 == declared.sha256
                    && installed.size_bytes == declared.size_bytes
                    && installed.role == declared.role
            });
    if json != stored.receipt_json
        || digest != stored.receipt.installation_receipt_digest
        || stored.files != binding.installed_files
        || adoption.receipt().adoption_material_digest != binding.adoption_material_digest
        || adoption_binding.application_id != binding.application_id
        || adoption_binding.application_digest != binding.application_digest
        || adoption_binding.provider_id != binding.provider_id
        || adoption_binding.provider_owner_account_id != binding.provider_owner_account_id
        || adoption_binding.provider_policy_revision != binding.provider_policy_revision
        || adoption_binding.provider_digest != binding.provider_digest
        || adoption_binding.admission_id != binding.admission_id
        || adoption_binding.admission_digest != binding.admission_digest
        || adoption_binding.adapter_id != binding.adapter_id
        || adoption_binding.adapter_release_version != binding.adapter_release_version
        || adoption_binding.adapter_config_revision != binding.adapter_config_revision
        || adoption_binding.adapter_config_digest != binding.adapter_config_digest
        || adoption_binding.declared_implementation_sha256 != binding.declared_implementation_sha256
        || adoption_binding.capability_set_digest != binding.capability_set_digest
        || adoption_binding.credential_locator_commitment != binding.credential_locator_commitment
        || package_receipt.package_receipt_id != binding.package_receipt_id
        || package_receipt.package_receipt_digest != binding.package_receipt_digest
        || package_receipt.package_material_digest != binding.package_material_digest
        || package_item.admission_id != binding.admission_id
        || package_item.admission_digest != binding.admission_digest
        || package_item.source_receipt_digest != binding.source_receipt_digest
        || package_item.archive_sha256 != binding.archive_sha256
        || package_item.archive_size_bytes != binding.archive_size_bytes
        || package_item.manifest_digest != binding.manifest_digest
        || package_item.entry_inventory_digest != binding.entry_inventory_digest
        || package_item.entry_count != binding.entry_count
        || package_item.total_uncompressed_bytes != binding.total_uncompressed_bytes
        || package_item.manifest.adapter_id != binding.adapter_id
        || package_item.manifest.release_version != binding.adapter_release_version
        || package_item.manifest.capability_set_digest != binding.capability_set_digest
        || package_item.manifest.runtime.kind != binding.runtime_kind
        || package_item.manifest.runtime.entrypoint != binding.entrypoint_path
        || !package_files_are_exact
        || source.source_receipt_id() != binding.source_receipt_id
        || source.source_receipt_digest() != binding.source_receipt_digest
        || source.admission_id() != binding.admission_id
        || source.admission_digest() != binding.admission_digest
        || source.adapter_id() != binding.adapter_id
        || source.release_version() != binding.adapter_release_version
        || source.artifact_sha256() != binding.archive_sha256
        || source.artifact_size_bytes() != binding.archive_size_bytes
        || !exact_projection(conn, &stored)?
    {
        bail!("Adapter installation failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterInstallation,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.installation;
    let binding = &item.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_installation_receipts
          WHERE installation_receipt_id=?1 AND installation_receipt_digest=?2
            AND installation_receipt_schema=?3 AND receipt_json=?4
            AND installation_material_digest=?5 AND canonicalization=?6
            AND digest_algorithm=?7 AND adoption_receipt_id=?8
            AND adoption_receipt_digest=?9 AND adoption_material_digest=?10
            AND application_id=?11 AND application_digest=?12 AND provider_id=?13
            AND provider_owner_account_id=?14 AND provider_policy_revision=?15
            AND provider_digest=?16 AND admission_id=?17 AND admission_digest=?18
            AND adapter_id=?19 AND adapter_release_version=?20
            AND adapter_config_revision=?21 AND adapter_config_digest=?22
            AND declared_implementation_sha256=?23 AND capability_set_digest=?24
            AND credential_locator_commitment=?25 AND package_receipt_id=?26
            AND package_receipt_digest=?27 AND package_material_digest=?28
            AND source_receipt_id=?29 AND source_receipt_digest=?30
            AND archive_sha256=?31 AND archive_size_bytes=?32
            AND manifest_digest=?33 AND entry_inventory_digest=?34
            AND entry_count=?35 AND total_uncompressed_bytes=?36
            AND runtime_kind=?37 AND entrypoint_path=?38 AND entrypoint_sha256=?39
            AND entrypoint_size_bytes=?40 AND installation_content_digest=?41
            AND storage_namespace=?42 AND installed_by_admin_user_id=?43
            AND confirmation=?44 AND idempotency_scope=?45 AND idempotency_key=?46
            AND installed_at=?47 AND recorded_at=?48 AND installation_effect=?49
            AND credential_effect=?50 AND provider_effect=?51 AND route_effect=?52
            AND execution_effect=?53 AND settlement_effect=?54",
            params![
                receipt.installation_receipt_id,
                receipt.installation_receipt_digest,
                receipt.schema,
                stored.receipt_json,
                receipt.installation_material_digest,
                receipt.canonicalization,
                receipt.digest_algorithm,
                binding.adoption_receipt_id,
                binding.adoption_receipt_digest,
                binding.adoption_material_digest,
                binding.application_id,
                binding.application_digest,
                binding.provider_id,
                binding.provider_owner_account_id,
                binding.provider_policy_revision,
                binding.provider_digest,
                binding.admission_id,
                binding.admission_digest,
                binding.adapter_id,
                binding.adapter_release_version,
                binding.adapter_config_revision,
                binding.adapter_config_digest,
                binding.declared_implementation_sha256,
                binding.capability_set_digest,
                binding.credential_locator_commitment,
                binding.package_receipt_id,
                binding.package_receipt_digest,
                binding.package_material_digest,
                binding.source_receipt_id,
                binding.source_receipt_digest,
                binding.archive_sha256,
                binding.archive_size_bytes as i64,
                binding.manifest_digest,
                binding.entry_inventory_digest,
                binding.entry_count as i64,
                binding.total_uncompressed_bytes as i64,
                binding.runtime_kind,
                binding.entrypoint_path,
                binding.entrypoint_sha256,
                binding.entrypoint_size_bytes as i64,
                binding.installation_content_digest,
                binding.storage_namespace,
                item.installed_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.installed_at,
                item.recorded_at,
                item.installation_effect,
                item.credential_effect,
                item.provider_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn currentness_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<ExternalPoolAdapterInstallationCurrentness>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    let statuses: (String, String, String, String, String) = conn.query_row(
        "SELECT current_status,adoption_status,package_status,source_status,file_inventory_status
           FROM compute_external_pool_adapter_installation_current
          WHERE installation_receipt_id=?1 AND installation_receipt_digest=?2",
        params![receipt_id, stored.receipt.installation_receipt_digest],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    Ok(Some(ExternalPoolAdapterInstallationCurrentness {
        schema: CURRENTNESS_SCHEMA,
        installation: stored.summary(),
        current_status: statuses.0,
        adoption_status: statuses.1,
        package_status: statuses.2,
        source_status: statuses.3,
        file_inventory_status: statuses.4,
    }))
}

impl Store {
    pub(crate) fn external_pool_adapter_installation_currentness(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ExternalPoolAdapterInstallationCurrentness>> {
        let connection = self.conn()?;
        currentness_on(&connection, receipt_id)
    }
}
