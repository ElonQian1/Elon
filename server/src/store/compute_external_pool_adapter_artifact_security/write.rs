use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_artifact_package::ARTIFACT_PACKAGE_FORMAT_EFFECT,
    compute_federation::external_pool_adapter_artifact_security::{
        canonical_artifact_security_receipt_json_and_digest, security_material_digest,
        validate_artifact_security_inspection, validate_artifact_security_receipt,
        ExternalPoolAdapterArtifactSecurityReceipt,
        ExternalPoolAdapterArtifactSecurityReceiptMaterial, ARTIFACT_SECURITY_CANONICALIZATION,
        ARTIFACT_SECURITY_CONFIRMATION, ARTIFACT_SECURITY_DIGEST_ALGORITHM,
        ARTIFACT_SECURITY_EFFECT, ARTIFACT_SECURITY_EVIDENCE_SCOPE,
        ARTIFACT_SECURITY_LICENSE_POLICY_ID, ARTIFACT_SECURITY_NO_EFFECT,
        ARTIFACT_SECURITY_RECEIPT_SCHEMA,
    },
    store::{new_id, Store},
};

use super::{
    read::{receipt_by_admission_on, receipt_by_idempotency_on, scan_target_on},
    types::{
        CreateExternalPoolAdapterArtifactSecurityReceipt,
        ExternalPoolAdapterArtifactSecurityWriteReceipt, StoredArtifactSecurityReceipt,
    },
};

impl Store {
    pub(crate) fn create_external_pool_adapter_artifact_security_receipt(
        &self,
        input: CreateExternalPoolAdapterArtifactSecurityReceipt,
    ) -> Result<ExternalPoolAdapterArtifactSecurityWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = receipt_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let result = result(&stored, true);
            transaction.commit()?;
            return Ok(result);
        }
        if receipt_by_admission_on(&transaction, &input.expected.admission_id)?.is_some() {
            bail!("Artifact security receipt already exists for this admission");
        }
        let target = scan_target_on(
            &transaction,
            &input.expected.admission_id,
            &input.expected.admission_digest,
            &input.expected.source_receipt_digest,
            &input.expected.provenance_receipt_digest,
            &input.expected.package_receipt_digest,
        )?;
        if target.package_receipt_id != input.expected.package_receipt_id
            || target.package_inspection_digest != input.expected.package_inspection_digest
            || target.manifest_digest != input.expected.manifest_digest
            || input.scanned.artifact_digest() != target.archive_sha256
            || input.scanned.artifact_size_bytes() != target.archive_size_bytes
            || input.scanned.package_inspection().inspection_digest
                != target.package_inspection_digest
        {
            bail!("scanned CAS/package evidence drifted from exact V232 authority");
        }
        validate_artifact_security_inspection(input.scanned.inspection(), &target)?;

        let scanned_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let scan = input.scanned.inspection();
        let material = ExternalPoolAdapterArtifactSecurityReceiptMaterial {
            admission_id: target.admission_id,
            admission_digest: target.admission_digest,
            source_receipt_digest: target.source_receipt_digest,
            provenance_receipt_digest: target.provenance_receipt_digest,
            package_receipt_id: target.package_receipt_id,
            package_receipt_digest: target.package_receipt_digest,
            archive_sha256: scan.archive_sha256.clone(),
            archive_size_bytes: scan.archive_size_bytes,
            package_inspection_digest: scan.package_inspection_digest.clone(),
            manifest_digest: scan.manifest_digest.clone(),
            sbom_canonical_json: scan.sbom_canonical_json.clone(),
            sbom_digest: scan.sbom_digest.clone(),
            component_inventory_digest: scan.component_inventory_digest.clone(),
            component_count: scan.component_count,
            license_inventory_digest: scan.license_inventory_digest.clone(),
            license_count: scan.license_count,
            scanned_file_inventory_digest: scan.scanned_file_inventory_digest.clone(),
            scanned_file_count: scan.scanned_file_count,
            scanner_rule_set_id: scan.scanner_rule_set_id.clone(),
            scanner_rule_set_digest: scan.scanner_rule_set_digest.clone(),
            license_policy_id: ARTIFACT_SECURITY_LICENSE_POLICY_ID.to_string(),
            finding_count: scan.finding_count,
            inspection_digest: scan.inspection_digest.clone(),
            scanned_by_admin_user_id: input.scanned_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            scanned_at: scanned_at.clone(),
            recorded_at: scanned_at,
            evidence_scope: ARTIFACT_SECURITY_EVIDENCE_SCOPE.to_string(),
            artifact_format_effect: ARTIFACT_PACKAGE_FORMAT_EFFECT.to_string(),
            artifact_security_effect: ARTIFACT_SECURITY_EFFECT.to_string(),
            vulnerability_intelligence_effect: ARTIFACT_SECURITY_NO_EFFECT.to_string(),
            conformance_effect: ARTIFACT_SECURITY_NO_EFFECT.to_string(),
            adapter_effect: ARTIFACT_SECURITY_NO_EFFECT.to_string(),
            route_effect: ARTIFACT_SECURITY_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterArtifactSecurityReceipt {
            schema: ARTIFACT_SECURITY_RECEIPT_SCHEMA.to_string(),
            security_receipt_id: new_id("external_pool_adapter_artifact_security"),
            security_receipt_digest: String::new(),
            security_material_digest: security_material_digest(&material)?,
            canonicalization: ARTIFACT_SECURITY_CANONICALIZATION.to_string(),
            digest_algorithm: ARTIFACT_SECURITY_DIGEST_ALGORITHM.to_string(),
            security: material,
        };
        receipt.security_receipt_digest =
            canonical_artifact_security_receipt_json_and_digest(&receipt)?.1;
        validate_artifact_security_receipt(&receipt)?;
        let (receipt_json, digest) = canonical_artifact_security_receipt_json_and_digest(&receipt)?;
        if digest != receipt.security_receipt_digest {
            bail!("Artifact security receipt digest changed before persistence");
        }
        insert(&transaction, &receipt, &receipt_json)?;
        let stored = receipt_by_admission_on(&transaction, &input.expected.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("Artifact security receipt disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("Artifact security receipt changed during exact readback");
        }
        let result = result(&stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert(
    transaction: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterArtifactSecurityReceipt,
    receipt_json: &str,
) -> Result<()> {
    let item = &receipt.security;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_security_receipts(
            security_receipt_id,security_receipt_schema,security_receipt_digest,security_receipt_json,security_material_digest,
            canonicalization,digest_algorithm,admission_id,admission_digest,source_receipt_digest,provenance_receipt_digest,
            package_receipt_id,package_receipt_digest,archive_sha256,archive_size_bytes,package_inspection_digest,manifest_digest,
            sbom_canonical_json,sbom_digest,component_inventory_digest,component_count,license_inventory_digest,license_count,
            scanned_file_inventory_digest,scanned_file_count,scanner_rule_set_id,scanner_rule_set_digest,license_policy_id,
            finding_count,inspection_digest,scanned_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,scanned_at,
            recorded_at,evidence_scope,artifact_format_effect,artifact_security_effect,vulnerability_intelligence_effect,
            conformance_effect,adapter_effect,route_effect)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,
                 ?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43)",
        params![receipt.security_receipt_id,receipt.schema,receipt.security_receipt_digest,receipt_json,receipt.security_material_digest,
            receipt.canonicalization,receipt.digest_algorithm,item.admission_id,item.admission_digest,item.source_receipt_digest,
            item.provenance_receipt_digest,item.package_receipt_id,item.package_receipt_digest,item.archive_sha256,
            item.archive_size_bytes as i64,item.package_inspection_digest,item.manifest_digest,item.sbom_canonical_json,item.sbom_digest,
            item.component_inventory_digest,item.component_count as i64,item.license_inventory_digest,item.license_count as i64,
            item.scanned_file_inventory_digest,item.scanned_file_count as i64,item.scanner_rule_set_id,item.scanner_rule_set_digest,
            item.license_policy_id,item.finding_count as i64,item.inspection_digest,item.scanned_by_admin_user_id,item.confirmation,
            item.idempotency_scope,item.idempotency_key,item.scanned_at,item.recorded_at,item.evidence_scope,item.artifact_format_effect,
            item.artifact_security_effect,item.vulnerability_intelligence_effect,item.conformance_effect,item.adapter_effect,item.route_effect],
    )?;
    Ok(())
}

fn validate_input(input: &CreateExternalPoolAdapterArtifactSecurityReceipt) -> Result<()> {
    if input.confirmation != ARTIFACT_SECURITY_CONFIRMATION
        || input.scanned_by_admin_user_id.trim().is_empty()
        || input.idempotency_scope.trim().is_empty()
        || input.idempotency_key.trim().is_empty()
    {
        bail!("Artifact security scan input is invalid");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredArtifactSecurityReceipt,
    input: &CreateExternalPoolAdapterArtifactSecurityReceipt,
) -> Result<()> {
    let item = &stored.receipt.security;
    let scan = input.scanned.inspection();
    if item.admission_id != input.expected.admission_id
        || item.admission_digest != input.expected.admission_digest
        || item.source_receipt_digest != input.expected.source_receipt_digest
        || item.provenance_receipt_digest != input.expected.provenance_receipt_digest
        || item.package_receipt_digest != input.expected.package_receipt_digest
        || item.scanned_by_admin_user_id != input.scanned_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
        || item.archive_sha256 != input.scanned.artifact_digest()
        || item.inspection_digest != scan.inspection_digest
    {
        bail!("Artifact security idempotency replay conflicts with original material");
    }
    Ok(())
}

fn result(
    stored: &StoredArtifactSecurityReceipt,
    replayed: bool,
) -> ExternalPoolAdapterArtifactSecurityWriteReceipt {
    ExternalPoolAdapterArtifactSecurityWriteReceipt {
        security: stored.summary(),
        replayed,
    }
}
