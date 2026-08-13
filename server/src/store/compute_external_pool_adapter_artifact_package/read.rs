use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_artifact_package::{
        canonical_artifact_package_receipt_json_and_digest, validate_artifact_package_receipt,
        ARTIFACT_PACKAGE_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_artifact_signed_provenance::{
            current_external_pool_adapter_artifact_signed_provenance_authority_on,
            external_pool_adapter_artifact_signed_provenance_authority_on,
        },
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_release::admission_by_id_on,
        compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
        Store,
    },
};

use super::types::{
    ExternalPoolAdapterArtifactPackageAuthority,
    ExternalPoolAdapterArtifactPackageCurrentnessReceipt,
    ExternalPoolAdapterArtifactPackageInspectionTarget, StoredArtifactPackageReceipt,
};

pub(in crate::store) fn artifact_package_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_package_receipt_digest: &str,
) -> Result<Option<ExternalPoolAdapterArtifactPackageAuthority>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    if stored.receipt.package_receipt_digest != expected_package_receipt_digest {
        return Ok(None);
    }
    Ok(Some(ExternalPoolAdapterArtifactPackageAuthority::new(
        stored.receipt,
    )))
}

pub(in crate::store) fn current_artifact_package_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_package_receipt_digest: &str,
) -> Result<Option<ExternalPoolAdapterArtifactPackageAuthority>> {
    let Some(authority) =
        artifact_package_authority_on(conn, admission_id, expected_package_receipt_digest)?
    else {
        return Ok(None);
    };
    let current: Option<String> = conn
        .query_row(
            "SELECT current_status
               FROM compute_external_pool_adapter_artifact_package_current
              WHERE admission_id=?1 AND package_receipt_digest=?2",
            params![admission_id, expected_package_receipt_digest],
            |row| row.get(0),
        )
        .optional()?;
    if current.as_deref() != Some("verified_current") {
        return Ok(None);
    }
    Ok(Some(authority))
}

pub(in crate::store) fn artifact_package_is_current_exact_on(
    conn: &Connection,
    admission_id: &str,
    expected_package_receipt_digest: &str,
) -> Result<bool> {
    let Some(currentness) = currentness_on(conn, admission_id)? else {
        return Ok(false);
    };
    Ok(currentness.current_status == "verified_current"
        && currentness.package.package_receipt_digest == expected_package_receipt_digest)
}

pub(super) fn receipt_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredArtifactPackageReceipt>> {
    receipt_on(conn, "admission_id=?1", params![admission_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredArtifactPackageReceipt>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredArtifactPackageReceipt>> {
    conn.query_row(
        &format!(
            "SELECT package_receipt_json
               FROM compute_external_pool_adapter_artifact_package_receipts
              WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredArtifactPackageReceipt {
                receipt,
                receipt_json,
            })
        },
    )
    .optional()?
    .map(|stored| audit_receipt(conn, stored))
    .transpose()
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredArtifactPackageReceipt,
) -> Result<StoredArtifactPackageReceipt> {
    validate_artifact_package_receipt(&stored.receipt)?;
    let (json, digest) = canonical_artifact_package_receipt_json_and_digest(&stored.receipt)?;
    let package = &stored.receipt.package;
    let admission = admission_by_id_on(conn, &package.admission_id)?
        .ok_or_else(|| anyhow::anyhow!("Artifact package lost its admission root"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &package.admission_id,
        &package.admission_digest,
        &package.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Artifact package lost its source root"))?;
    let provenance = external_pool_adapter_artifact_signed_provenance_authority_on(
        conn,
        &package.admission_id,
        &package.provenance_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Artifact package lost its provenance root"))?;
    if json != stored.receipt_json
        || digest != stored.receipt.package_receipt_digest
        || admission.admission_digest != package.admission_digest
        || admission.adapter_id != package.manifest.adapter_id
        || admission.release_version != package.manifest.release_version
        || admission.supported_capabilities != package.manifest.supported_capabilities
        || admission.capability_set_digest != package.manifest.capability_set_digest
        || admission.expected_credential_verifier != package.manifest.credential_verifier
        || source.artifact_sha256() != package.archive_sha256
        || source.artifact_size_bytes() != package.archive_size_bytes
        || provenance.provenance_receipt_id() != package.provenance_receipt_id
        || provenance.binding().source_receipt_digest != package.source_receipt_digest
        || !exact_projection(conn, &stored)?
    {
        bail!("Artifact package receipt failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(conn: &Connection, stored: &StoredArtifactPackageReceipt) -> Result<bool> {
    let receipt = &stored.receipt;
    let package = &receipt.package;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_package_receipts
          WHERE package_receipt_id=?1 AND package_receipt_digest=?2
            AND package_receipt_json=?3 AND package_material_digest=?4
            AND admission_id=?5 AND admission_digest=?6
            AND source_receipt_digest=?7 AND provenance_receipt_id=?8
            AND provenance_receipt_digest=?9 AND archive_sha256=?10
            AND archive_size_bytes=?11 AND manifest_canonical_json=?12
            AND manifest_digest=?13 AND entry_inventory_digest=?14
            AND entry_count=?15 AND total_uncompressed_bytes=?16
            AND inspection_digest=?17 AND adapter_id=?18 AND release_version=?19
            AND runtime_kind=?20 AND runtime_entrypoint=?21
            AND capability_set_digest=?22 AND credential_verifier_digest=?23
            AND inspected_by_admin_user_id=?24 AND idempotency_scope=?25
            AND idempotency_key=?26 AND inspected_at=?27",
            params![
                receipt.package_receipt_id,
                receipt.package_receipt_digest,
                stored.receipt_json,
                receipt.package_material_digest,
                package.admission_id,
                package.admission_digest,
                package.source_receipt_digest,
                package.provenance_receipt_id,
                package.provenance_receipt_digest,
                package.archive_sha256,
                package.archive_size_bytes as i64,
                package.manifest_canonical_json,
                package.manifest_digest,
                package.entry_inventory_digest,
                package.entry_count as i64,
                package.total_uncompressed_bytes as i64,
                package.inspection_digest,
                package.manifest.adapter_id,
                package.manifest.release_version,
                package.manifest.runtime.kind,
                package.manifest.runtime.entrypoint,
                package.manifest.capability_set_digest,
                package.manifest.credential_verifier.verifier_digest,
                package.inspected_by_admin_user_id,
                package.idempotency_scope,
                package.idempotency_key,
                package.inspected_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn inspection_target_on(
    conn: &Connection,
    admission_id: &str,
    expected_admission_digest: &str,
    expected_source_receipt_digest: &str,
    expected_provenance_receipt_digest: &str,
) -> Result<ExternalPoolAdapterArtifactPackageInspectionTarget> {
    let admission = current_external_pool_adapter_release_admission_authority_on(
        conn,
        admission_id,
        expected_admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current staged admission was not found"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        admission_id,
        expected_admission_digest,
        expected_source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("exact Artifact source receipt was not found"))?;
    let provenance = current_external_pool_adapter_artifact_signed_provenance_authority_on(
        conn,
        admission_id,
        expected_provenance_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current signed provenance was not found"))?;
    let binding = provenance.binding();
    if source.admission_id() != admission.admission_id()
        || source.admission_digest() != admission.admission_digest()
        || source.source_receipt_digest() != binding.source_receipt_digest
        || source.artifact_sha256() != binding.artifact_sha256
        || source.artifact_size_bytes() != binding.artifact_size_bytes
        || binding.admission_digest != admission.admission_digest()
        || binding.adapter_id != admission.adapter_id()
        || binding.release_version != admission.release_version()
    {
        bail!("Artifact package inspection authorities drifted");
    }
    Ok(ExternalPoolAdapterArtifactPackageInspectionTarget {
        admission_id: admission.admission_id().to_string(),
        admission_digest: admission.admission_digest().to_string(),
        adapter_id: admission.adapter_id().to_string(),
        release_version: admission.release_version().to_string(),
        source_receipt_digest: source.source_receipt_digest().to_string(),
        provenance_receipt_id: provenance.provenance_receipt_id().to_string(),
        provenance_receipt_digest: provenance.provenance_receipt_digest().to_string(),
        artifact_sha256: source.artifact_sha256().to_string(),
        artifact_size_bytes: source.artifact_size_bytes(),
        supported_capabilities: admission.supported_capabilities().to_vec(),
        capability_set_digest: admission.capability_set_digest().to_string(),
        credential_verifier: admission.expected_credential_verifier().clone(),
    })
}

pub(super) fn currentness_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<ExternalPoolAdapterArtifactPackageCurrentnessReceipt>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    let (current_status, admission_status, signer_status): (String, String, String) = conn
        .query_row(
            "SELECT current_status, admission_current_status, signer_current_status
           FROM compute_external_pool_adapter_artifact_package_current
          WHERE admission_id=?1 AND package_receipt_digest=?2",
            params![admission_id, stored.receipt.package_receipt_digest],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    Ok(Some(ExternalPoolAdapterArtifactPackageCurrentnessReceipt {
        schema: ARTIFACT_PACKAGE_CURRENTNESS_SCHEMA,
        package: stored.summary(),
        current_status,
        admission_current_status: admission_status,
        signer_current_status: signer_status,
    }))
}

impl Store {
    pub(crate) fn external_pool_adapter_artifact_package_inspection_target(
        &self,
        admission_id: &str,
        expected_admission_digest: &str,
        expected_source_receipt_digest: &str,
        expected_provenance_receipt_digest: &str,
    ) -> Result<ExternalPoolAdapterArtifactPackageInspectionTarget> {
        let connection = self.conn()?;
        inspection_target_on(
            &connection,
            admission_id,
            expected_admission_digest,
            expected_source_receipt_digest,
            expected_provenance_receipt_digest,
        )
    }

    pub(crate) fn external_pool_adapter_artifact_package_currentness(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactPackageCurrentnessReceipt>> {
        let connection = self.conn()?;
        currentness_on(&connection, admission_id)
    }
}
