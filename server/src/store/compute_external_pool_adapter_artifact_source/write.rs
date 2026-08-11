use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::store::{
    compute_external_pool_adapter_release::{
        admission_by_id_on, ExternalPoolAdapterReleaseArtifactSourceAdmission,
    },
    new_id, Store,
};

use super::{
    canonical::{canonical_intake_material_digest, canonical_receipt_json_and_digest},
    read::{receipt_by_admission_on, receipt_by_idempotency_on, validate_digest, validate_exact},
    types::{
        ExternalPoolAdapterArtifactSourceReceipt, RecordExternalPoolAdapterArtifactSource,
        StoredArtifactSource, StoredArtifactSourceEnvelope, StoredArtifactSourceReceipt,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CANONICALIZATION,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE,
        EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND,
        MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_SIZE_BYTES,
    },
};

struct ArtifactEvidence {
    intake_sha256: String,
    reopened_sha256: String,
    artifact_size_bytes: i64,
    content_address_digest: String,
}

struct IdempotencyMaterial {
    admission_id: String,
    admission_digest: String,
    recorded_by_admin_user_id: String,
    intake_confirmation: String,
    intake_sha256: String,
    reopened_sha256: String,
    artifact_size_bytes: i64,
    content_address_digest: String,
    idempotency_scope: String,
    idempotency_key: String,
}

impl Store {
    pub(crate) fn record_external_pool_adapter_artifact_source(
        &self,
        input: RecordExternalPoolAdapterArtifactSource,
    ) -> Result<ExternalPoolAdapterArtifactSourceReceipt> {
        validate_input(&input)?;
        let evidence = artifact_evidence(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(material) = idempotency_material_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_idempotency_material(&material, &input, &evidence)?;
            let stored = receipt_by_idempotency_on(
                &transaction,
                &input.idempotency_scope,
                &input.idempotency_key,
            )?
            .ok_or_else(|| anyhow::anyhow!("artifact source idempotency row disappeared"))?;
            ensure_replay(&stored, &input, &evidence)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let admission = admission_by_id_on(&transaction, &input.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter staged admission is absent"))?;
        ensure_exact_admission(&admission, &input, &evidence)?;
        if receipt_by_admission_on(&transaction, &input.admission_id)?.is_some() {
            bail!(
                "external-pool Adapter artifact source already exists under another idempotency key"
            );
        }

        let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let source = source_material(&admission, &input, &evidence, recorded_at);
        let intake_material_digest = canonical_intake_material_digest(&source)?;
        let mut envelope = StoredArtifactSourceEnvelope {
            schema: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_RECEIPT_SCHEMA.to_string(),
            source_receipt_id: new_id("compute_external_pool_adapter_artifact_source_receipt"),
            source_receipt_digest: String::new(),
            intake_material_digest,
            canonicalization: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CANONICALIZATION.to_string(),
            digest_algorithm: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM.to_string(),
            source,
        };
        let (_, receipt_digest) = canonical_receipt_json_and_digest(&envelope)?;
        envelope.source_receipt_digest = receipt_digest;
        let (receipt_json, receipt_digest) = canonical_receipt_json_and_digest(&envelope)?;
        if receipt_digest != envelope.source_receipt_digest {
            bail!("artifact source receipt digest changed before persistence");
        }

        insert_receipt(&transaction, &envelope, &receipt_json)?;
        let stored =
            receipt_by_admission_on(&transaction, &input.admission_id)?.ok_or_else(|| {
                anyhow::anyhow!("external-pool Adapter artifact source is absent after insert")
            })?;
        if stored.envelope != envelope || stored.source_receipt_json != receipt_json {
            bail!("external-pool Adapter artifact source changed during exact readback");
        }
        let receipt = stored.into_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn idempotency_material_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<IdempotencyMaterial>> {
    let material = conn
        .query_row(
            "SELECT admission_id, admission_digest, recorded_by_admin_user_id,
                intake_confirmation, intake_sha256, reopened_sha256,
                artifact_size_bytes, content_address_digest,
                idempotency_scope, idempotency_key
           FROM compute_external_pool_adapter_artifact_source_receipts
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| {
                Ok(IdempotencyMaterial {
                    admission_id: row.get(0)?,
                    admission_digest: row.get(1)?,
                    recorded_by_admin_user_id: row.get(2)?,
                    intake_confirmation: row.get(3)?,
                    intake_sha256: row.get(4)?,
                    reopened_sha256: row.get(5)?,
                    artifact_size_bytes: row.get(6)?,
                    content_address_digest: row.get(7)?,
                    idempotency_scope: row.get(8)?,
                    idempotency_key: row.get(9)?,
                })
            },
        )
        .optional()?;
    Ok(material)
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    envelope: &StoredArtifactSourceEnvelope,
    receipt_json: &str,
) -> Result<()> {
    let source = &envelope.source;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_source_receipts (
            source_receipt_id, source_receipt_schema, source_receipt_digest,
            source_receipt_json, canonicalization, digest_algorithm,
            admission_id, admission_digest, request_id, request_digest,
            request_material_digest, review_id, review_digest, adapter_id, release_version,
            candidate_artifact_ref, declared_implementation_sha256,
            intake_sha256, reopened_sha256, artifact_size_bytes,
            storage_root_kind, storage_namespace, content_address_algorithm,
            content_address_digest, custody_state, intake_kind, evidence_scope,
            artifact_ref_resolution_effect, adapter_effect, route_effect,
            recorded_by_admin_user_id, intake_confirmation, recorded_at,
            intake_material_digest, idempotency_scope, idempotency_key, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
            ?31, ?32, ?33, ?34, ?35, ?36, ?37
         )",
        params![
            envelope.source_receipt_id,
            envelope.schema,
            envelope.source_receipt_digest,
            receipt_json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            source.admission_id,
            source.admission_digest,
            source.request_id,
            source.request_digest,
            source.request_material_digest,
            source.review_id,
            source.review_digest,
            source.adapter_id,
            source.release_version,
            source.candidate_artifact_ref,
            source.declared_implementation_sha256,
            source.intake_sha256,
            source.reopened_sha256,
            source.artifact_size_bytes,
            source.storage_root_kind,
            source.storage_namespace,
            source.content_address_algorithm,
            source.content_address_digest,
            source.custody_state,
            source.intake_kind,
            source.evidence_scope,
            source.artifact_ref_resolution_effect,
            source.adapter_effect,
            source.route_effect,
            source.recorded_by_admin_user_id,
            source.intake_confirmation,
            source.recorded_at,
            envelope.intake_material_digest,
            source.idempotency_scope,
            source.idempotency_key,
            source.created_at,
        ],
    )?;
    Ok(())
}

fn source_material(
    admission: &ExternalPoolAdapterReleaseArtifactSourceAdmission,
    input: &RecordExternalPoolAdapterArtifactSource,
    evidence: &ArtifactEvidence,
    recorded_at: String,
) -> StoredArtifactSource {
    StoredArtifactSource {
        admission_id: admission.admission_id.clone(),
        admission_digest: admission.admission_digest.clone(),
        request_id: admission.request_id.clone(),
        request_digest: admission.request_digest.clone(),
        request_material_digest: admission.request_material_digest.clone(),
        review_id: admission.review_id.clone(),
        review_digest: admission.review_digest.clone(),
        adapter_id: admission.adapter_id.clone(),
        release_version: admission.release_version.clone(),
        candidate_artifact_ref: admission.candidate_artifact_ref.clone(),
        declared_implementation_sha256: admission.declared_implementation_sha256.clone(),
        intake_sha256: evidence.intake_sha256.clone(),
        reopened_sha256: evidence.reopened_sha256.clone(),
        artifact_size_bytes: evidence.artifact_size_bytes,
        storage_root_kind: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_ROOT_KIND.to_string(),
        storage_namespace: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_STORAGE_NAMESPACE.to_string(),
        content_address_algorithm: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_DIGEST_ALGORITHM
            .to_string(),
        content_address_digest: evidence.content_address_digest.clone(),
        custody_state: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_CUSTODY_STATE.to_string(),
        intake_kind: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_KIND.to_string(),
        evidence_scope: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_EVIDENCE_SCOPE.to_string(),
        artifact_ref_resolution_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT.to_string(),
        adapter_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT.to_string(),
        route_effect: EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_NO_EFFECT.to_string(),
        recorded_by_admin_user_id: input.recorded_by_admin_user_id.clone(),
        intake_confirmation: input.intake_confirmation.clone(),
        recorded_at: recorded_at.clone(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        created_at: recorded_at,
    }
}

fn artifact_evidence(input: &RecordExternalPoolAdapterArtifactSource) -> Result<ArtifactEvidence> {
    let evidence = ArtifactEvidence {
        intake_sha256: input.artifact.intake_sha256().to_string(),
        reopened_sha256: input.artifact.reopened_sha256().to_string(),
        artifact_size_bytes: i64::try_from(input.artifact.artifact_size_bytes())?,
        content_address_digest: input.artifact.content_address_digest().to_string(),
    };
    for (value, label) in [
        (&evidence.intake_sha256, "sealed intake digest"),
        (&evidence.reopened_sha256, "sealed reopened digest"),
        (
            &evidence.content_address_digest,
            "sealed content address digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    if evidence.artifact_size_bytes < 1
        || evidence.artifact_size_bytes > MAX_EXTERNAL_POOL_ADAPTER_ARTIFACT_SIZE_BYTES as i64
        || evidence.intake_sha256 != evidence.reopened_sha256
        || evidence.reopened_sha256 != evidence.content_address_digest
    {
        bail!("sealed quarantine artifact bytes are not exact");
    }
    Ok(evidence)
}

fn ensure_exact_admission(
    admission: &ExternalPoolAdapterReleaseArtifactSourceAdmission,
    input: &RecordExternalPoolAdapterArtifactSource,
    evidence: &ArtifactEvidence,
) -> Result<()> {
    if admission.admission_id != input.admission_id
        || admission.admission_digest != input.expected_admission_digest
        || admission.status != "staged"
        || admission.declared_implementation_sha256 != evidence.intake_sha256
    {
        bail!("only exact staged admission bytes can create an artifact source receipt");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredArtifactSourceReceipt,
    input: &RecordExternalPoolAdapterArtifactSource,
    evidence: &ArtifactEvidence,
) -> Result<()> {
    let source = &stored.envelope.source;
    if source.admission_id != input.admission_id
        || source.admission_digest != input.expected_admission_digest
        || source.recorded_by_admin_user_id != input.recorded_by_admin_user_id
        || source.intake_confirmation != input.intake_confirmation
        || source.idempotency_scope != input.idempotency_scope
        || source.idempotency_key != input.idempotency_key
        || source.intake_sha256 != evidence.intake_sha256
        || source.reopened_sha256 != evidence.reopened_sha256
        || source.artifact_size_bytes != evidence.artifact_size_bytes
        || source.content_address_digest != evidence.content_address_digest
    {
        bail!("external-pool Adapter artifact source replay conflicts with immutable history");
    }
    Ok(())
}

fn ensure_idempotency_material(
    material: &IdempotencyMaterial,
    input: &RecordExternalPoolAdapterArtifactSource,
    evidence: &ArtifactEvidence,
) -> Result<()> {
    if material.admission_id != input.admission_id
        || material.admission_digest != input.expected_admission_digest
        || material.recorded_by_admin_user_id != input.recorded_by_admin_user_id
        || material.intake_confirmation != input.intake_confirmation
        || material.intake_sha256 != evidence.intake_sha256
        || material.reopened_sha256 != evidence.reopened_sha256
        || material.artifact_size_bytes != evidence.artifact_size_bytes
        || material.content_address_digest != evidence.content_address_digest
        || material.idempotency_scope != input.idempotency_scope
        || material.idempotency_key != input.idempotency_key
    {
        bail!("external-pool Adapter artifact source idempotency material conflicts");
    }
    Ok(())
}

fn validate_input(input: &RecordExternalPoolAdapterArtifactSource) -> Result<()> {
    for (value, label, max) in [
        (&input.admission_id, "artifact source admission ID", 160),
        (
            &input.recorded_by_admin_user_id,
            "artifact source administrator",
            160,
        ),
        (
            &input.idempotency_scope,
            "artifact source idempotency scope",
            200,
        ),
        (
            &input.idempotency_key,
            "artifact source idempotency key",
            160,
        ),
    ] {
        validate_exact(value, label, max)?;
    }
    validate_digest(
        &input.expected_admission_digest,
        "artifact source expected admission digest",
    )?;
    if input.intake_confirmation != EXTERNAL_POOL_ADAPTER_ARTIFACT_SOURCE_INTAKE_CONFIRMATION {
        bail!("artifact source intake confirmation is not exact");
    }
    Ok(())
}
