use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_release::{
        canonical_external_pool_adapter_release_request_json_and_digest,
        canonical_external_pool_adapter_release_request_material_digest,
        validate_external_pool_adapter_release_request_envelope,
        validate_external_pool_adapter_release_request_material,
        ComputeExternalPoolAdapterReleaseRequest, ComputeExternalPoolAdapterReleaseRequestEnvelope,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA,
    },
    store::{new_id, Store},
};

use super::{
    canonical::{canonical_capabilities_json_and_digest, canonical_json},
    read::{admission_by_adapter_release_on, request_by_id_on, request_by_idempotency_on},
    review::{now_nanos, validate_exact},
    types::{ExternalPoolAdapterReleaseRequestReceipt, SubmitExternalPoolAdapterReleaseRequest},
};

const MATERIAL_VALIDATION_TIMESTAMP: &str = "1970-01-01T00:00:00.000000000Z";

impl Store {
    pub(in crate::store) fn submit_external_pool_adapter_release_request(
        &self,
        input: SubmitExternalPoolAdapterReleaseRequest,
    ) -> Result<ExternalPoolAdapterReleaseRequestReceipt> {
        validate_exact(&input.idempotency_scope, "request idempotency scope", 200)?;
        let material = request_material(&input, MATERIAL_VALIDATION_TIMESTAMP.to_string());
        validate_external_pool_adapter_release_request_material(&material)?;
        let material_digest =
            canonical_external_pool_adapter_release_request_material_digest(&material)?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = request_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input, &material_digest)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if admission_by_adapter_release_on(
            &transaction,
            &input.release.adapter_id,
            &input.release.release_version,
        )?
        .is_some()
        {
            bail!("external-pool Adapter release is already staged");
        }

        let request_id = new_id("compute_external_pool_adapter_release_request");
        let submitted_at = now_nanos();
        let request = request_material(&input, submitted_at.clone());
        let mut envelope = ComputeExternalPoolAdapterReleaseRequestEnvelope {
            schema: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA.to_string(),
            request_id,
            request_digest: String::new(),
            request_material_digest: material_digest,
            canonicalization: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM.to_string(),
            request,
        };
        let (_, request_digest) =
            canonical_external_pool_adapter_release_request_json_and_digest(&envelope)?;
        envelope.request_digest = request_digest;
        validate_external_pool_adapter_release_request_envelope(&envelope)?;
        let (request_json, digest) =
            canonical_external_pool_adapter_release_request_json_and_digest(&envelope)?;
        if digest != envelope.request_digest {
            bail!("external-pool Adapter release request digest changed before persistence");
        }

        let release = &envelope.request.release;
        let supported_provider_kinds_json = canonical_json(&release.supported_provider_kinds)?;
        let (capabilities_json, capability_set_digest) =
            canonical_capabilities_json_and_digest(&release.supported_capabilities)?;
        if capability_set_digest != release.capability_set_digest {
            bail!("external-pool Adapter release capability digest changed before persistence");
        }
        let verifier = &release.expected_credential_verifier;
        transaction.execute(
            "INSERT INTO compute_external_pool_adapter_release_requests (
                request_id, request_schema, request_digest, request_json,
                canonicalization, digest_algorithm, request_material_digest,
                adapter_id, release_version, route_kind, supported_provider_kinds_json,
                candidate_artifact_ref, declared_implementation_sha256, capabilities_json,
                capability_set_digest, verifier_verification_kind, verifier_id,
                verifier_revision, verifier_digest, submit_confirmation, submit_note,
                submitted_by_admin_user_id, submitted_at, status,
                reviewed_by_admin_user_id, reviewed_at, applied_by_admin_user_id, applied_at,
                idempotency_scope, idempotency_key, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, 'submitted',
                NULL, NULL, NULL, NULL, ?24, ?25, ?23, ?23
             )",
            params![
                envelope.request_id,
                envelope.schema,
                envelope.request_digest,
                request_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
                envelope.request_material_digest,
                release.adapter_id,
                release.release_version,
                release.route_kind,
                supported_provider_kinds_json,
                release.candidate_artifact_ref,
                release.declared_implementation_sha256,
                capabilities_json,
                release.capability_set_digest,
                verifier.verification_kind,
                verifier.verifier_id,
                verifier.verifier_revision,
                verifier.verifier_digest,
                envelope.request.confirmation,
                envelope.request.submission_note,
                envelope.request.submitted_by_admin_user_id,
                envelope.request.submitted_at,
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = request_by_id_on(&transaction, &envelope.request_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter release request is absent after insert"))?;
        let receipt = stored.into_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn request_material(
    input: &SubmitExternalPoolAdapterReleaseRequest,
    submitted_at: String,
) -> ComputeExternalPoolAdapterReleaseRequest {
    ComputeExternalPoolAdapterReleaseRequest {
        submitted_by_admin_user_id: input.submitted_by_admin_user_id.clone(),
        release: input.release.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        submission_note: input.submission_note.clone(),
        submitted_at,
    }
}

fn ensure_replay(
    stored: &super::types::StoredRequest,
    input: &SubmitExternalPoolAdapterReleaseRequest,
    material_digest: &str,
) -> Result<()> {
    let request = &stored.envelope.request;
    if stored.envelope.request_material_digest != material_digest
        || request.submitted_by_admin_user_id != input.submitted_by_admin_user_id
        || request.release != input.release
        || request.idempotency_key != input.idempotency_key
        || request.confirmation != input.confirmation
        || request.submission_note != input.submission_note
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool Adapter release request replay conflicts with immutable history");
    }
    Ok(())
}
