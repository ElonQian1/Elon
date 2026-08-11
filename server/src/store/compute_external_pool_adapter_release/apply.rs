use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_release::{
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
    },
    store::{new_id, Store},
};

use super::{
    canonical::{
        canonical_admission_json_and_digest, canonical_capabilities_json_and_digest, canonical_json,
    },
    read::{
        admission_by_adapter_release_on, admission_by_idempotency_on, admission_by_request_on,
        request_by_id_on, review_by_request_on,
    },
    review::{now_nanos, validate_digest, validate_exact, validate_optional_note},
    types::{
        ApplyExternalPoolAdapterRelease, ExternalPoolAdapterReleaseAdmissionReceipt,
        StoredAdmissionEnvelope, StoredAdmissionMaterial, ADMISSION_STATUS_STAGED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA,
        EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION, REVIEW_DECISION_APPROVED,
    },
};

impl Store {
    pub(in crate::store) fn apply_external_pool_adapter_release(
        &self,
        input: ApplyExternalPoolAdapterRelease,
    ) -> Result<ExternalPoolAdapterReleaseAdmissionReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = admission_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }

        let request = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release request is absent"))?;
        if let Some(stored) = admission_by_request_on(&transaction, &input.request_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = stored.into_receipt(true);
            transaction.commit()?;
            return Ok(receipt);
        }
        let release = &request.envelope.request.release;
        if admission_by_adapter_release_on(
            &transaction,
            &release.adapter_id,
            &release.release_version,
        )?
        .is_some()
        {
            bail!("external-pool Adapter release is already staged by another request");
        }
        if request.status != REVIEW_DECISION_APPROVED
            || request.envelope.request_digest != input.expected_request_digest
            || request.envelope.request_material_digest != input.expected_request_material_digest
        {
            bail!("only the exact approved Adapter release request can be staged");
        }
        let review = review_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool Adapter release review is absent"))?;
        if review.envelope.review.decision != REVIEW_DECISION_APPROVED
            || review.envelope.review_digest != input.expected_review_digest
            || review.envelope.review.request_digest != input.expected_request_digest
            || review.envelope.review.request_material_digest
                != input.expected_request_material_digest
            || review.envelope.review.reviewed_by_admin_user_id
                == request.envelope.request.submitted_by_admin_user_id
        {
            bail!("external-pool Adapter release approval is stale or not four-eyes");
        }

        let (capabilities_json, capability_set_digest) =
            canonical_capabilities_json_and_digest(&release.supported_capabilities)?;
        if capability_set_digest != release.capability_set_digest {
            bail!("external-pool Adapter release capability digest changed before staging");
        }
        let supported_provider_kinds_json = canonical_json(&release.supported_provider_kinds)?;
        let verifier = &release.expected_credential_verifier;
        let mut envelope = StoredAdmissionEnvelope {
            schema: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA.to_string(),
            admission_id: new_id("compute_external_pool_adapter_release_admission"),
            admission_digest: String::new(),
            canonicalization: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM.to_string(),
            admission: StoredAdmissionMaterial {
                request_id: request.envelope.request_id.clone(),
                request_digest: request.envelope.request_digest.clone(),
                request_material_digest: request.envelope.request_material_digest.clone(),
                review_id: review.envelope.review_id.clone(),
                review_digest: review.envelope.review_digest.clone(),
                adapter_id: release.adapter_id.clone(),
                release_version: release.release_version.clone(),
                route_kind: release.route_kind.clone(),
                supported_provider_kinds: release.supported_provider_kinds.clone(),
                candidate_artifact_ref: release.candidate_artifact_ref.clone(),
                declared_implementation_sha256: release.declared_implementation_sha256.clone(),
                supported_capabilities: release.supported_capabilities.clone(),
                capability_set_digest: release.capability_set_digest.clone(),
                expected_credential_verifier: verifier.clone(),
                submitted_by_admin_user_id: request
                    .envelope
                    .request
                    .submitted_by_admin_user_id
                    .clone(),
                reviewed_by_admin_user_id: review.envelope.review.reviewed_by_admin_user_id.clone(),
                applied_by_admin_user_id: input.applied_by_admin_user_id.clone(),
                apply_confirmation: input.apply_confirmation.clone(),
                apply_note: input.apply_note.clone(),
                applied_at: now_nanos(),
                status: ADMISSION_STATUS_STAGED.to_string(),
            },
        };
        let (_, digest) = canonical_admission_json_and_digest(&envelope)?;
        envelope.admission_digest = digest;
        let (admission_json, _) = canonical_admission_json_and_digest(&envelope)?;
        let admission = &envelope.admission;
        transaction.execute(
            "INSERT INTO compute_external_pool_adapter_release_admissions (
                admission_id, admission_schema, admission_digest, admission_json,
                canonicalization, digest_algorithm, request_id, request_digest,
                request_material_digest, review_id, review_digest, adapter_id,
                release_version, route_kind, supported_provider_kinds_json,
                candidate_artifact_ref, declared_implementation_sha256, capabilities_json,
                capability_set_digest, verifier_verification_kind, verifier_id,
                verifier_revision, verifier_digest, submitted_by_admin_user_id,
                reviewed_by_admin_user_id, applied_by_admin_user_id, apply_confirmation,
                apply_note, applied_at, status, idempotency_scope, idempotency_key, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24,
                ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?29
             )",
            params![
                envelope.admission_id,
                envelope.schema,
                envelope.admission_digest,
                admission_json,
                envelope.canonicalization,
                envelope.digest_algorithm,
                admission.request_id,
                admission.request_digest,
                admission.request_material_digest,
                admission.review_id,
                admission.review_digest,
                admission.adapter_id,
                admission.release_version,
                admission.route_kind,
                supported_provider_kinds_json,
                admission.candidate_artifact_ref,
                admission.declared_implementation_sha256,
                capabilities_json,
                admission.capability_set_digest,
                admission.expected_credential_verifier.verification_kind,
                admission.expected_credential_verifier.verifier_id,
                admission.expected_credential_verifier.verifier_revision,
                admission.expected_credential_verifier.verifier_digest,
                admission.submitted_by_admin_user_id,
                admission.reviewed_by_admin_user_id,
                admission.applied_by_admin_user_id,
                admission.apply_confirmation,
                admission.apply_note,
                admission.applied_at,
                admission.status,
                input.idempotency_scope,
                input.idempotency_key,
            ],
        )?;
        let stored = admission_by_request_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter release admission is absent after insert"))?;
        let receipt = stored.into_receipt(false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn validate_input(input: &ApplyExternalPoolAdapterRelease) -> Result<()> {
    validate_exact(&input.request_id, "apply request ID", 160)?;
    validate_digest(&input.expected_request_digest, "apply request digest")?;
    validate_digest(
        &input.expected_request_material_digest,
        "apply request material digest",
    )?;
    validate_digest(&input.expected_review_digest, "apply review digest")?;
    validate_exact(
        &input.applied_by_admin_user_id,
        "applying administrator",
        160,
    )?;
    validate_exact(&input.idempotency_scope, "apply idempotency scope", 200)?;
    validate_exact(&input.idempotency_key, "apply idempotency key", 160)?;
    validate_optional_note(&input.apply_note, "apply note", 2_000)?;
    if input.apply_confirmation != EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION {
        bail!("external-pool Adapter release apply confirmation is not exact");
    }
    Ok(())
}

fn ensure_replay(
    stored: &super::types::StoredAdmission,
    input: &ApplyExternalPoolAdapterRelease,
) -> Result<()> {
    let admission = &stored.envelope.admission;
    if admission.request_id != input.request_id
        || admission.request_digest != input.expected_request_digest
        || admission.request_material_digest != input.expected_request_material_digest
        || admission.review_digest != input.expected_review_digest
        || admission.applied_by_admin_user_id != input.applied_by_admin_user_id
        || admission.apply_confirmation != input.apply_confirmation
        || admission.apply_note != input.apply_note
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("external-pool Adapter release apply replay conflicts with immutable history");
    }
    Ok(())
}
