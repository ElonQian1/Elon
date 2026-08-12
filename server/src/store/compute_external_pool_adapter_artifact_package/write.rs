use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_artifact_package::{
        canonical_artifact_package_receipt_json_and_digest, package_material_digest,
        validate_artifact_package_inspection, validate_artifact_package_receipt,
        ExternalPoolAdapterArtifactPackageReceipt,
        ExternalPoolAdapterArtifactPackageReceiptMaterial, ARTIFACT_PACKAGE_CANONICALIZATION,
        ARTIFACT_PACKAGE_CONFIRMATION, ARTIFACT_PACKAGE_DIGEST_ALGORITHM,
        ARTIFACT_PACKAGE_EVIDENCE_SCOPE, ARTIFACT_PACKAGE_FORMAT_EFFECT,
        ARTIFACT_PACKAGE_NO_EFFECT, ARTIFACT_PACKAGE_RECEIPT_SCHEMA,
    },
    store::{new_id, Store},
};

use super::{
    read::{inspection_target_on, receipt_by_admission_on, receipt_by_idempotency_on},
    types::{
        CreateExternalPoolAdapterArtifactPackageReceipt,
        ExternalPoolAdapterArtifactPackageWriteReceipt, StoredArtifactPackageReceipt,
    },
};

impl Store {
    pub(crate) fn create_external_pool_adapter_artifact_package_receipt(
        &self,
        input: CreateExternalPoolAdapterArtifactPackageReceipt,
    ) -> Result<ExternalPoolAdapterArtifactPackageWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = receipt_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let result = write_receipt(&stored, true);
            transaction.commit()?;
            return Ok(result);
        }
        if receipt_by_admission_on(&transaction, &input.expected_admission_id)?.is_some() {
            bail!("Artifact package receipt already exists for this admission");
        }

        let target = inspection_target_on(
            &transaction,
            &input.expected_admission_id,
            &input.expected_admission_digest,
            &input.expected_source_receipt_digest,
            &input.expected_provenance_receipt_digest,
        )?;
        if input.inspected.artifact_digest() != target.artifact_sha256
            || input.inspected.artifact_size_bytes() != target.artifact_size_bytes
        {
            bail!("inspected CAS handle drifted from exact authorities");
        }
        validate_artifact_package_inspection(input.inspected.inspection(), &target.expected())?;

        let inspected_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let inspection = input.inspected.inspection();
        let material = ExternalPoolAdapterArtifactPackageReceiptMaterial {
            admission_id: target.admission_id,
            admission_digest: target.admission_digest,
            source_receipt_digest: target.source_receipt_digest,
            provenance_receipt_id: target.provenance_receipt_id,
            provenance_receipt_digest: target.provenance_receipt_digest,
            archive_sha256: inspection.archive_sha256.clone(),
            archive_size_bytes: inspection.archive_size_bytes,
            manifest: inspection.manifest.clone(),
            manifest_canonical_json: inspection.manifest_canonical_json.clone(),
            manifest_digest: inspection.manifest_digest.clone(),
            entry_inventory_digest: inspection.entry_inventory_digest.clone(),
            entry_count: inspection.entry_count,
            total_uncompressed_bytes: inspection.total_uncompressed_bytes,
            inspection_digest: inspection.inspection_digest.clone(),
            inspected_by_admin_user_id: input.inspected_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            inspected_at: inspected_at.clone(),
            recorded_at: inspected_at,
            evidence_scope: ARTIFACT_PACKAGE_EVIDENCE_SCOPE.to_string(),
            artifact_format_effect: ARTIFACT_PACKAGE_FORMAT_EFFECT.to_string(),
            artifact_security_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
            conformance_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
            adapter_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
            route_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterArtifactPackageReceipt {
            schema: ARTIFACT_PACKAGE_RECEIPT_SCHEMA.to_string(),
            package_receipt_id: new_id("external_pool_adapter_artifact_package"),
            package_receipt_digest: String::new(),
            package_material_digest: package_material_digest(&material)?,
            canonicalization: ARTIFACT_PACKAGE_CANONICALIZATION.to_string(),
            digest_algorithm: ARTIFACT_PACKAGE_DIGEST_ALGORITHM.to_string(),
            package: material,
        };
        receipt.package_receipt_digest =
            canonical_artifact_package_receipt_json_and_digest(&receipt)?.1;
        validate_artifact_package_receipt(&receipt)?;
        let (receipt_json, digest) = canonical_artifact_package_receipt_json_and_digest(&receipt)?;
        if digest != receipt.package_receipt_digest {
            bail!("Artifact package receipt digest changed before persistence");
        }
        insert_receipt(&transaction, &receipt, &receipt_json)?;
        let stored = receipt_by_admission_on(&transaction, &input.expected_admission_id)?
            .ok_or_else(|| anyhow::anyhow!("Artifact package receipt disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("Artifact package receipt changed during exact readback");
        }
        let result = write_receipt(&stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

fn insert_receipt(
    transaction: &rusqlite::Transaction<'_>,
    receipt: &ExternalPoolAdapterArtifactPackageReceipt,
    receipt_json: &str,
) -> Result<()> {
    let package = &receipt.package;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_package_receipts(
            package_receipt_id, package_receipt_schema, package_receipt_digest,
            package_receipt_json, package_material_digest, canonicalization, digest_algorithm,
            admission_id, admission_digest, source_receipt_digest, provenance_receipt_id,
            provenance_receipt_digest, archive_sha256, archive_size_bytes,
            manifest_canonical_json, manifest_digest, entry_inventory_digest, entry_count,
            total_uncompressed_bytes, inspection_digest, adapter_id, release_version,
            runtime_kind, runtime_entrypoint, supported_capabilities_json,
            capability_set_digest, credential_verifier_json, credential_verifier_digest,
            inspected_by_admin_user_id, confirmation, idempotency_scope, idempotency_key,
            inspected_at, recorded_at, evidence_scope, artifact_format_effect,
            artifact_security_effect, conformance_effect, adapter_effect, route_effect)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,
                 ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,
                 ?33,?34,?35,?36,?37,?38,?39,?40)",
        params![
            receipt.package_receipt_id,
            receipt.schema,
            receipt.package_receipt_digest,
            receipt_json,
            receipt.package_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
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
            serde_json::to_string(&package.manifest.supported_capabilities)?,
            package.manifest.capability_set_digest,
            serde_json::to_string(&package.manifest.credential_verifier)?,
            package.manifest.credential_verifier.verifier_digest,
            package.inspected_by_admin_user_id,
            package.confirmation,
            package.idempotency_scope,
            package.idempotency_key,
            package.inspected_at,
            package.recorded_at,
            package.evidence_scope,
            package.artifact_format_effect,
            package.artifact_security_effect,
            package.conformance_effect,
            package.adapter_effect,
            package.route_effect,
        ],
    )?;
    Ok(())
}

fn validate_input(input: &CreateExternalPoolAdapterArtifactPackageReceipt) -> Result<()> {
    for (value, label, max) in [
        (&input.expected_admission_id, "package admission ID", 160),
        (&input.inspected_by_admin_user_id, "package inspector", 160),
        (&input.idempotency_scope, "package idempotency scope", 200),
        (&input.idempotency_key, "package idempotency key", 160),
    ] {
        crate::compute_federation::external_pool_adapter_artifact_package::validate_identifier(
            value, label, max,
        )?;
    }
    for digest in [
        &input.expected_admission_digest,
        &input.expected_source_receipt_digest,
        &input.expected_provenance_receipt_digest,
    ] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("Artifact package expected digest is invalid");
        }
    }
    if input.confirmation != ARTIFACT_PACKAGE_CONFIRMATION {
        bail!("Artifact package confirmation is invalid");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredArtifactPackageReceipt,
    input: &CreateExternalPoolAdapterArtifactPackageReceipt,
) -> Result<()> {
    let package = &stored.receipt.package;
    let inspection = input.inspected.inspection();
    if package.admission_id != input.expected_admission_id
        || package.admission_digest != input.expected_admission_digest
        || package.source_receipt_digest != input.expected_source_receipt_digest
        || package.provenance_receipt_digest != input.expected_provenance_receipt_digest
        || package.inspected_by_admin_user_id != input.inspected_by_admin_user_id
        || package.confirmation != input.confirmation
        || package.idempotency_scope != input.idempotency_scope
        || package.idempotency_key != input.idempotency_key
        || package.archive_sha256 != input.inspected.artifact_digest()
        || package.archive_size_bytes != input.inspected.artifact_size_bytes()
        || package.inspection_digest != inspection.inspection_digest
        || package.manifest_digest != inspection.manifest_digest
        || package.entry_inventory_digest != inspection.entry_inventory_digest
    {
        bail!("Artifact package idempotency replay conflicts with original material");
    }
    Ok(())
}

fn write_receipt(
    stored: &StoredArtifactPackageReceipt,
    replayed: bool,
) -> ExternalPoolAdapterArtifactPackageWriteReceipt {
    ExternalPoolAdapterArtifactPackageWriteReceipt {
        package: stored.summary(),
        replayed,
    }
}
