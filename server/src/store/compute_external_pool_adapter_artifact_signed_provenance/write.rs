use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{SecondsFormat, Utc};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{params, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_artifact_signed_provenance::{
        candidate_artifact_ref_digest, canonical_signed_provenance_receipt_json_and_digest,
        signature_challenge, validate_digest, validate_exact, validate_signed_provenance_receipt,
        verification_material_digest, ExternalPoolAdapterArtifactSignatureBinding,
        ExternalPoolAdapterArtifactSignedProvenanceMaterial,
        ExternalPoolAdapterArtifactSignedProvenanceReceipt, ARTIFACT_SIGNATURE_BINDING_SCHEMA,
        ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION, ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION,
        ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM, ARTIFACT_SIGNED_PROVENANCE_EVIDENCE_SCOPE,
        ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT, ARTIFACT_SIGNED_PROVENANCE_RECEIPT_SCHEMA,
        ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM,
    },
    store::{
        compute_external_pool_adapter_artifact_signing_key::current_external_pool_adapter_artifact_signing_key_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
        new_id, Store,
    },
};

use super::{
    read::{receipt_by_admission_on, receipt_by_idempotency_on},
    types::{
        CreateExternalPoolAdapterArtifactSignedProvenance,
        ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt,
        GetExternalPoolAdapterArtifactSignatureChallenge, StoredSignedProvenanceReceipt,
    },
};

impl Store {
    pub(crate) fn external_pool_adapter_artifact_signature_challenge(
        &self,
        input: GetExternalPoolAdapterArtifactSignatureChallenge,
    ) -> Result<crate::compute_federation::external_pool_adapter_artifact_signed_provenance::ExternalPoolAdapterArtifactSignatureChallengeReceipt>{
        validate_challenge_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let challenge = signature_challenge(exact_binding(&transaction, &input)?)?;
        transaction.commit()?;
        Ok(challenge)
    }

    pub(crate) fn create_external_pool_adapter_artifact_signed_provenance(
        &self,
        input: CreateExternalPoolAdapterArtifactSignedProvenance,
    ) -> Result<ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = receipt_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            // Historical exact replay remains readable even after admission/key currentness ends.
            let result = write_receipt(&stored, true);
            transaction.commit()?;
            return Ok(result);
        }

        if receipt_by_admission_on(&transaction, &input.admission_id)?.is_some() {
            bail!("signed provenance already exists for this admission");
        }
        let challenge_input = GetExternalPoolAdapterArtifactSignatureChallenge {
            admission_id: input.admission_id.clone(),
            expected_admission_digest: input.expected_admission_digest.clone(),
            expected_source_receipt_digest: input.expected_source_receipt_digest.clone(),
            key_record_id: input.key_record_id.clone(),
            expected_key_record_digest: input.expected_key_record_digest.clone(),
            expected_key_id: input.expected_key_id.clone(),
        };
        let binding = exact_binding(&transaction, &challenge_input)?;
        let challenge = signature_challenge(binding.clone())?;
        if challenge.signature_message_digest != input.expected_signature_message_digest {
            bail!("signature challenge is stale");
        }
        verify_signature(
            &challenge.signature_message_base64,
            &input.signature_base64,
            &transaction,
            &input,
        )?;

        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let signature_bytes = STANDARD.decode(&input.signature_base64)?;
        let material = ExternalPoolAdapterArtifactSignedProvenanceMaterial {
            binding,
            signature_message_digest: challenge.signature_message_digest,
            signature_base64: input.signature_base64,
            signature_digest: hex::encode(Sha256::digest(&signature_bytes)),
            verified_by_admin_user_id: input.verified_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            verified_at: timestamp.clone(),
            recorded_at: timestamp,
            evidence_scope: ARTIFACT_SIGNED_PROVENANCE_EVIDENCE_SCOPE.to_string(),
            artifact_ref_resolution_effect: ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT.to_string(),
            artifact_format_effect: ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT.to_string(),
            conformance_effect: ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT.to_string(),
            adapter_effect: ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT.to_string(),
            route_effect: ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterArtifactSignedProvenanceReceipt {
            schema: ARTIFACT_SIGNED_PROVENANCE_RECEIPT_SCHEMA.to_string(),
            provenance_receipt_id: new_id("external_pool_adapter_artifact_signed_provenance"),
            provenance_receipt_digest: String::new(),
            verification_material_digest: verification_material_digest(&material)?,
            canonicalization: ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION.to_string(),
            digest_algorithm: ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM.to_string(),
            provenance: material,
        };
        receipt.provenance_receipt_digest =
            canonical_signed_provenance_receipt_json_and_digest(&receipt)?.1;
        validate_signed_provenance_receipt(&receipt)?;
        let (receipt_json, digest) = canonical_signed_provenance_receipt_json_and_digest(&receipt)?;
        if digest != receipt.provenance_receipt_digest {
            bail!("signed-provenance digest changed before persistence");
        }
        insert_receipt(&transaction, &receipt, &receipt_json)?;
        let stored = receipt_by_admission_on(&transaction, &input.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("signed provenance disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("signed provenance changed during exact readback");
        }
        let result = write_receipt(&stored, false);
        transaction.commit()?;
        Ok(result)
    }
}

pub(super) fn exact_binding(
    transaction: &Transaction<'_>,
    input: &GetExternalPoolAdapterArtifactSignatureChallenge,
) -> Result<ExternalPoolAdapterArtifactSignatureBinding> {
    let admission = current_external_pool_adapter_release_admission_authority_on(
        transaction,
        &input.admission_id,
        &input.expected_admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current staged admission was not found"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        transaction,
        &input.admission_id,
        &input.expected_admission_digest,
        &input.expected_source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("exact Artifact source receipt was not found"))?;
    let key = current_external_pool_adapter_artifact_signing_key_authority_on(
        transaction,
        &input.key_record_id,
        &input.expected_key_record_digest,
        &input.expected_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active Artifact signer key was not found"))?;
    if source.admission_id() != admission.admission_id()
        || source.admission_digest() != admission.admission_digest()
        || source.adapter_id() != admission.adapter_id()
        || source.release_version() != admission.release_version()
        || source.artifact_sha256() != admission.declared_implementation_sha256()
        || key.key_record_id() != input.key_record_id
        || key.key_record_digest() != input.expected_key_record_digest
        || key.key_id() != input.expected_key_id
    {
        bail!("Artifact signature binding authorities drifted");
    }
    Ok(ExternalPoolAdapterArtifactSignatureBinding {
        schema: ARTIFACT_SIGNATURE_BINDING_SCHEMA.to_string(),
        admission_id: admission.admission_id().to_string(),
        admission_digest: admission.admission_digest().to_string(),
        adapter_id: admission.adapter_id().to_string(),
        release_version: admission.release_version().to_string(),
        candidate_artifact_ref_digest: candidate_artifact_ref_digest(
            source.candidate_artifact_ref(),
        ),
        source_receipt_id: source.source_receipt_id().to_string(),
        source_receipt_digest: source.source_receipt_digest().to_string(),
        artifact_sha256: source.artifact_sha256().to_string(),
        artifact_size_bytes: source.artifact_size_bytes(),
        key_record_id: key.key_record_id().to_string(),
        key_record_digest: key.key_record_digest().to_string(),
        key_id: key.key_id().to_string(),
        source_operator: key.source_operator().to_string(),
        signature_algorithm: ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM.to_string(),
    })
}

fn verify_signature(
    message_base64: &str,
    signature_base64: &str,
    transaction: &Transaction<'_>,
    input: &CreateExternalPoolAdapterArtifactSignedProvenance,
) -> Result<()> {
    let key = current_external_pool_adapter_artifact_signing_key_authority_on(
        transaction,
        &input.key_record_id,
        &input.expected_key_record_digest,
        &input.expected_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active signer disappeared before verification"))?;
    let message = STANDARD.decode(message_base64)?;
    let signature = STANDARD.decode(signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != signature_base64
    {
        bail!("Artifact signature Base64 is not canonical");
    }
    let public_key = RsaPublicKey::from_public_key_pem(key.public_key_pem())
        .context("decode registered Artifact signer public key")?;
    let signature =
        RsaSignature::try_from(signature.as_slice()).context("decode RSA Artifact signature")?;
    VerifyingKey::<Sha256>::new(public_key)
        .verify(&message, &signature)
        .context("Artifact signature verification failed")
}

fn validate_input(input: &CreateExternalPoolAdapterArtifactSignedProvenance) -> Result<()> {
    validate_challenge_input(&GetExternalPoolAdapterArtifactSignatureChallenge {
        admission_id: input.admission_id.clone(),
        expected_admission_digest: input.expected_admission_digest.clone(),
        expected_source_receipt_digest: input.expected_source_receipt_digest.clone(),
        key_record_id: input.key_record_id.clone(),
        expected_key_record_digest: input.expected_key_record_digest.clone(),
        expected_key_id: input.expected_key_id.clone(),
    })?;
    for (value, label, max) in [
        (
            &input.verified_by_admin_user_id,
            "verifying administrator",
            160,
        ),
        (&input.idempotency_scope, "idempotency scope", 200),
        (&input.idempotency_key, "idempotency key", 160),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [(
        &input.expected_signature_message_digest,
        "expected signature message digest",
    )] {
        validate_digest(value, label)?;
    }
    if input.confirmation != ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION {
        bail!("signed-provenance confirmation is invalid");
    }
    let decoded = STANDARD.decode(&input.signature_base64)?;
    if decoded.is_empty()
        || decoded.len() > 1024
        || STANDARD.encode(decoded) != input.signature_base64
    {
        bail!("Artifact signature Base64 is invalid");
    }
    Ok(())
}

fn validate_challenge_input(
    input: &GetExternalPoolAdapterArtifactSignatureChallenge,
) -> Result<()> {
    validate_exact(&input.admission_id, "admission ID", 160)?;
    validate_exact(&input.key_record_id, "key record ID", 160)?;
    for (value, label) in [
        (
            &input.expected_admission_digest,
            "expected admission digest",
        ),
        (
            &input.expected_source_receipt_digest,
            "expected source receipt digest",
        ),
        (
            &input.expected_key_record_digest,
            "expected key record digest",
        ),
        (&input.expected_key_id, "expected key ID"),
    ] {
        validate_digest(value, label)?;
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredSignedProvenanceReceipt,
    input: &CreateExternalPoolAdapterArtifactSignedProvenance,
) -> Result<()> {
    let material = &stored.receipt.provenance;
    let binding = &material.binding;
    if binding.admission_id != input.admission_id
        || binding.admission_digest != input.expected_admission_digest
        || binding.source_receipt_digest != input.expected_source_receipt_digest
        || binding.key_record_id != input.key_record_id
        || binding.key_record_digest != input.expected_key_record_digest
        || binding.key_id != input.expected_key_id
        || material.signature_message_digest != input.expected_signature_message_digest
        || material.signature_base64 != input.signature_base64
        || material.verified_by_admin_user_id != input.verified_by_admin_user_id
        || material.confirmation != input.confirmation
    {
        bail!("signed-provenance idempotency replay changed immutable material");
    }
    Ok(())
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    receipt: &ExternalPoolAdapterArtifactSignedProvenanceReceipt,
    receipt_json: &str,
) -> Result<()> {
    let material = &receipt.provenance;
    let binding = &material.binding;
    transaction.execute(
        "INSERT INTO compute_external_pool_adapter_artifact_signed_provenance_receipts (
            provenance_receipt_id, provenance_receipt_schema, provenance_receipt_digest,
            provenance_receipt_json, verification_material_digest, canonicalization,
            digest_algorithm, admission_id, admission_digest, adapter_id, release_version,
            candidate_artifact_ref_digest, source_receipt_id, source_receipt_digest,
            artifact_sha256, artifact_size_bytes, key_record_id, key_record_digest, key_id,
            source_operator, signature_algorithm, signature_message_digest, signature_base64,
            signature_digest, verified_by_admin_user_id, confirmation, idempotency_scope,
            idempotency_key, verified_at, recorded_at, evidence_scope,
            artifact_ref_resolution_effect, artifact_format_effect, conformance_effect,
            adapter_effect, route_effect
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                   ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,
                   ?32,?33,?34,?35,?36)",
        params![
            receipt.provenance_receipt_id,
            receipt.schema,
            receipt.provenance_receipt_digest,
            receipt_json,
            receipt.verification_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
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
            binding.signature_algorithm,
            material.signature_message_digest,
            material.signature_base64,
            material.signature_digest,
            material.verified_by_admin_user_id,
            material.confirmation,
            material.idempotency_scope,
            material.idempotency_key,
            material.verified_at,
            material.recorded_at,
            material.evidence_scope,
            material.artifact_ref_resolution_effect,
            material.artifact_format_effect,
            material.conformance_effect,
            material.adapter_effect,
            material.route_effect,
        ],
    )?;
    Ok(())
}

fn write_receipt(
    stored: &StoredSignedProvenanceReceipt,
    replayed: bool,
) -> ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt {
    ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt {
        provenance: stored.summary(),
        replayed,
    }
}
