use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_artifact_signed_provenance::{
        canonical_signed_provenance_receipt_json_and_digest, validate_signed_provenance_receipt,
        ExternalPoolAdapterArtifactSignedProvenanceReceipt,
        ARTIFACT_SIGNED_PROVENANCE_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_artifact_signing_key::external_pool_adapter_artifact_signing_key_record_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        Store,
    },
};

use super::types::{
    ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt, StoredSignedProvenanceReceipt,
};

pub(super) fn receipt_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredSignedProvenanceReceipt>> {
    receipt_on(conn, "admission_id=?1", params![admission_id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSignedProvenanceReceipt>> {
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
) -> Result<Option<StoredSignedProvenanceReceipt>> {
    conn.query_row(
        &format!(
            "SELECT provenance_receipt_json
               FROM compute_external_pool_adapter_artifact_signed_provenance_receipts
              WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredSignedProvenanceReceipt {
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
    stored: StoredSignedProvenanceReceipt,
) -> Result<StoredSignedProvenanceReceipt> {
    validate_signed_provenance_receipt(&stored.receipt)?;
    let (json, digest) = canonical_signed_provenance_receipt_json_and_digest(&stored.receipt)?;
    let material = &stored.receipt.provenance;
    let binding = &material.binding;
    let source = external_pool_adapter_artifact_source_authority_on(
        conn,
        &binding.admission_id,
        &binding.admission_digest,
        &binding.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("signed provenance lost its source receipt"))?;
    let key = external_pool_adapter_artifact_signing_key_record_authority_on(
        conn,
        &binding.key_record_id,
        &binding.key_record_digest,
        &binding.key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("signed provenance lost its key root"))?;
    if json != stored.receipt_json
        || digest != stored.receipt.provenance_receipt_digest
        || source.source_receipt_id() != binding.source_receipt_id
        || source.adapter_id() != binding.adapter_id
        || source.release_version() != binding.release_version
        || source.artifact_sha256() != binding.artifact_sha256
        || source.artifact_size_bytes() != binding.artifact_size_bytes
        || key.source_operator() != binding.source_operator
        || !exact_projection(conn, &stored)?
    {
        bail!("signed-provenance receipt failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(conn: &Connection, stored: &StoredSignedProvenanceReceipt) -> Result<bool> {
    let receipt = &stored.receipt;
    let material = &receipt.provenance;
    let binding = &material.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_signed_provenance_receipts
          WHERE provenance_receipt_id=?1 AND provenance_receipt_digest=?2
            AND provenance_receipt_json=?3 AND verification_material_digest=?4
            AND admission_id=?5 AND admission_digest=?6 AND adapter_id=?7
            AND release_version=?8 AND candidate_artifact_ref_digest=?9
            AND source_receipt_id=?10 AND source_receipt_digest=?11
            AND artifact_sha256=?12 AND artifact_size_bytes=?13
            AND key_record_id=?14 AND key_record_digest=?15 AND key_id=?16
            AND source_operator=?17 AND signature_message_digest=?18
            AND signature_base64=?19 AND signature_digest=?20
            AND verified_by_admin_user_id=?21 AND idempotency_scope=?22
            AND idempotency_key=?23 AND verified_at=?24",
            params![
                receipt.provenance_receipt_id,
                receipt.provenance_receipt_digest,
                stored.receipt_json,
                receipt.verification_material_digest,
                binding.admission_id,
                binding.admission_digest,
                binding.adapter_id,
                binding.release_version,
                binding.candidate_artifact_ref_digest,
                binding.source_receipt_id,
                binding.source_receipt_digest,
                binding.artifact_sha256,
                binding.artifact_size_bytes as i64,
                binding.key_record_id,
                binding.key_record_digest,
                binding.key_id,
                binding.source_operator,
                material.signature_message_digest,
                material.signature_base64,
                material.signature_digest,
                material.verified_by_admin_user_id,
                material.idempotency_scope,
                material.idempotency_key,
                material.verified_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn currentness_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt>> {
    let Some(stored) = receipt_by_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    let (status, admission_status, signer_status): (String, String, String) = conn.query_row(
        "SELECT current_status, admission_current_status, signer_current_status
           FROM compute_external_pool_adapter_artifact_signed_provenance_current
          WHERE admission_id=?1 AND provenance_receipt_digest=?2",
        params![admission_id, stored.receipt.provenance_receipt_digest],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    Ok(Some(
        ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt {
            schema: ARTIFACT_SIGNED_PROVENANCE_CURRENTNESS_SCHEMA,
            provenance: stored.summary(),
            current_status: status,
            admission_current_status: admission_status,
            signer_current_status: signer_status,
        },
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_artifact_signed_provenance_currentness(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt>> {
        crate::compute_federation::external_pool_adapter_artifact_signed_provenance::validate_exact(
            admission_id,
            "signed-provenance admission ID",
            160,
        )?;
        let connection = self.conn()?;
        currentness_on(&connection, admission_id)
    }
}
