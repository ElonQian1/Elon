use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_artifact_security::{
        canonical_artifact_security_receipt_json_and_digest, canonical_sbom,
        validate_artifact_security_receipt, validate_sbom, ExternalPoolAdapterArtifactSbom,
        ARTIFACT_SECURITY_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_artifact_package::{
            artifact_package_authority_on, current_artifact_package_authority_on,
        },
        Store,
    },
};

use super::types::{
    ExternalPoolAdapterArtifactSecurityAuthority,
    ExternalPoolAdapterArtifactSecurityCurrentnessReceipt,
    ExternalPoolAdapterArtifactSecurityScanTarget, StoredArtifactSecurityReceipt,
};

pub(in crate::store) fn current_artifact_security_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_security_receipt_digest: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSecurityAuthority>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    if stored.receipt.security_receipt_digest != expected_security_receipt_digest {
        bail!("Artifact security receipt digest is stale");
    }
    let status: String = conn.query_row(
        "SELECT current_status FROM compute_external_pool_adapter_artifact_security_current
          WHERE admission_id=?1 AND security_receipt_digest=?2",
        params![admission_id, expected_security_receipt_digest],
        |row| row.get(0),
    )?;
    if status != "verified_current" {
        bail!("Artifact security receipt is historical and cannot be consumed");
    }
    Ok(Some(ExternalPoolAdapterArtifactSecurityAuthority::new(
        stored.receipt,
    )))
}

pub(in crate::store) fn historical_artifact_security_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_security_receipt_digest: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSecurityAuthority>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    if stored.receipt.security_receipt_digest != expected_security_receipt_digest {
        bail!("Artifact security receipt digest is stale");
    }
    Ok(Some(ExternalPoolAdapterArtifactSecurityAuthority::new(
        stored.receipt,
    )))
}

pub(super) fn scan_target_on(
    conn: &Connection,
    admission_id: &str,
    admission_digest: &str,
    source_receipt_digest: &str,
    provenance_receipt_digest: &str,
    package_receipt_digest: &str,
) -> Result<ExternalPoolAdapterArtifactSecurityScanTarget> {
    let authority =
        current_artifact_package_authority_on(conn, admission_id, package_receipt_digest)?
            .ok_or_else(|| anyhow::anyhow!("current Artifact package receipt was not found"))?;
    let receipt = authority.receipt();
    let package = &receipt.package;
    if package.admission_digest != admission_digest
        || package.source_receipt_digest != source_receipt_digest
        || package.provenance_receipt_digest != provenance_receipt_digest
    {
        bail!("Artifact security scan lineage conflicts with V232 authority");
    }
    Ok(ExternalPoolAdapterArtifactSecurityScanTarget {
        admission_id: package.admission_id.clone(),
        admission_digest: package.admission_digest.clone(),
        source_receipt_digest: package.source_receipt_digest.clone(),
        provenance_receipt_digest: package.provenance_receipt_digest.clone(),
        package_receipt_id: receipt.package_receipt_id.clone(),
        package_receipt_digest: receipt.package_receipt_digest.clone(),
        archive_sha256: package.archive_sha256.clone(),
        archive_size_bytes: package.archive_size_bytes,
        manifest: package.manifest.clone(),
        manifest_digest: package.manifest_digest.clone(),
        package_inspection_digest: package.inspection_digest.clone(),
    })
}

pub(super) fn receipt_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredArtifactSecurityReceipt>> {
    receipt_on(conn, "admission_id=?1", params![admission_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredArtifactSecurityReceipt>> {
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
) -> Result<Option<StoredArtifactSecurityReceipt>> {
    conn.query_row(
        &format!("SELECT security_receipt_json FROM compute_external_pool_adapter_artifact_security_receipts WHERE {filter}"),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))?;
            Ok(StoredArtifactSecurityReceipt { receipt, receipt_json })
        },
    ).optional()?.map(|stored| audit_receipt(conn, stored)).transpose()
}

fn audit_receipt(
    conn: &Connection,
    stored: StoredArtifactSecurityReceipt,
) -> Result<StoredArtifactSecurityReceipt> {
    validate_artifact_security_receipt(&stored.receipt)?;
    let (json, digest) = canonical_artifact_security_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.security;
    let package =
        artifact_package_authority_on(conn, &item.admission_id, &item.package_receipt_digest)?
            .ok_or_else(|| anyhow::anyhow!("Artifact security receipt lost its package root"))?;
    let root = &package.receipt().package;
    let sbom: ExternalPoolAdapterArtifactSbom = serde_json::from_str(&item.sbom_canonical_json)?;
    validate_sbom(&sbom, &root.manifest)?;
    let (sbom_json, sbom_digest) = canonical_sbom(&sbom)?;
    if json != stored.receipt_json
        || digest != stored.receipt.security_receipt_digest
        || sbom_json != item.sbom_canonical_json
        || sbom_digest != item.sbom_digest
        || root.admission_digest != item.admission_digest
        || root.source_receipt_digest != item.source_receipt_digest
        || root.provenance_receipt_digest != item.provenance_receipt_digest
        || package.receipt().package_receipt_id != item.package_receipt_id
        || root.archive_sha256 != item.archive_sha256
        || root.archive_size_bytes != item.archive_size_bytes
        || root.manifest_digest != item.manifest_digest
        || root.inspection_digest != item.package_inspection_digest
        || !exact_projection(conn, &stored)?
    {
        bail!("Artifact security receipt failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(conn: &Connection, stored: &StoredArtifactSecurityReceipt) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.security;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_security_receipts
          WHERE security_receipt_id=?1 AND security_receipt_digest=?2 AND security_receipt_json=?3
            AND security_material_digest=?4 AND admission_id=?5 AND admission_digest=?6
            AND source_receipt_digest=?7 AND provenance_receipt_digest=?8
            AND package_receipt_id=?9 AND package_receipt_digest=?10 AND archive_sha256=?11
            AND archive_size_bytes=?12 AND package_inspection_digest=?13 AND manifest_digest=?14
            AND sbom_canonical_json=?15 AND sbom_digest=?16
            AND component_inventory_digest=?17 AND component_count=?18
            AND license_inventory_digest=?19 AND license_count=?20
            AND scanned_file_inventory_digest=?21 AND scanned_file_count=?22
            AND scanner_rule_set_id=?23 AND scanner_rule_set_digest=?24
            AND license_policy_id=?25 AND finding_count=?26 AND inspection_digest=?27
            AND scanned_by_admin_user_id=?28 AND confirmation=?29
            AND idempotency_scope=?30 AND idempotency_key=?31
            AND scanned_at=?32 AND recorded_at=?33 AND evidence_scope=?34
            AND artifact_format_effect=?35 AND artifact_security_effect=?36
            AND vulnerability_intelligence_effect=?37 AND conformance_effect=?38
            AND adapter_effect=?39 AND route_effect=?40",
            params![
                receipt.security_receipt_id,
                receipt.security_receipt_digest,
                stored.receipt_json,
                receipt.security_material_digest,
                item.admission_id,
                item.admission_digest,
                item.source_receipt_digest,
                item.provenance_receipt_digest,
                item.package_receipt_id,
                item.package_receipt_digest,
                item.archive_sha256,
                item.archive_size_bytes as i64,
                item.package_inspection_digest,
                item.manifest_digest,
                item.sbom_canonical_json,
                item.sbom_digest,
                item.component_inventory_digest,
                item.component_count as i64,
                item.license_inventory_digest,
                item.license_count as i64,
                item.scanned_file_inventory_digest,
                item.scanned_file_count as i64,
                item.scanner_rule_set_id,
                item.scanner_rule_set_digest,
                item.license_policy_id,
                item.finding_count as i64,
                item.inspection_digest,
                item.scanned_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.scanned_at,
                item.recorded_at,
                item.evidence_scope,
                item.artifact_format_effect,
                item.artifact_security_effect,
                item.vulnerability_intelligence_effect,
                item.conformance_effect,
                item.adapter_effect,
                item.route_effect
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn currentness_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSecurityCurrentnessReceipt>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    let (status, admission, signer): (String, String, String) = conn.query_row(
        "SELECT current_status,admission_current_status,signer_current_status
           FROM compute_external_pool_adapter_artifact_security_current
          WHERE admission_id=?1 AND security_receipt_digest=?2",
        params![admission_id, stored.receipt.security_receipt_digest],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    Ok(Some(
        ExternalPoolAdapterArtifactSecurityCurrentnessReceipt {
            schema: ARTIFACT_SECURITY_CURRENTNESS_SCHEMA,
            security: stored.summary(),
            current_status: status,
            admission_current_status: admission,
            signer_current_status: signer,
        },
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_artifact_security_scan_target(
        &self,
        admission_id: &str,
        admission_digest: &str,
        source_receipt_digest: &str,
        provenance_receipt_digest: &str,
        package_receipt_digest: &str,
    ) -> Result<ExternalPoolAdapterArtifactSecurityScanTarget> {
        let connection = self.conn()?;
        scan_target_on(
            &connection,
            admission_id,
            admission_digest,
            source_receipt_digest,
            provenance_receipt_digest,
            package_receipt_digest,
        )
    }

    pub(crate) fn external_pool_adapter_artifact_security_currentness(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactSecurityCurrentnessReceipt>> {
        let connection = self.conn()?;
        currentness_on(&connection, admission_id)
    }
}
