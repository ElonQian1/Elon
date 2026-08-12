use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, SecondsFormat};
use sha2::{Digest, Sha256};

use super::{
    canonical::{
        canonical_signed_provenance_receipt_json_and_digest, signature_challenge,
        verification_material_digest,
    },
    types::*,
};

pub(crate) fn validate_signature_binding(
    binding: &ExternalPoolAdapterArtifactSignatureBinding,
) -> Result<()> {
    if binding.schema != ARTIFACT_SIGNATURE_BINDING_SCHEMA
        || binding.signature_algorithm != ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM
        || binding.artifact_size_bytes == 0
        || binding.artifact_size_bytes > 32 * 1024 * 1024
    {
        bail!("Artifact signature binding metadata is unsupported");
    }
    for (value, label, max) in [
        (&binding.admission_id, "admission ID", 160),
        (&binding.adapter_id, "Adapter ID", 160),
        (&binding.release_version, "release version", 80),
        (&binding.source_receipt_id, "source receipt ID", 160),
        (&binding.key_record_id, "key record ID", 160),
        (&binding.source_operator, "source operator", 160),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [
        (&binding.admission_digest, "admission digest"),
        (
            &binding.candidate_artifact_ref_digest,
            "candidate Artifact ref digest",
        ),
        (&binding.source_receipt_digest, "source receipt digest"),
        (&binding.artifact_sha256, "Artifact digest"),
        (&binding.key_record_digest, "key record digest"),
        (&binding.key_id, "key ID"),
    ] {
        validate_digest(value, label)?;
    }
    Ok(())
}

pub(crate) fn validate_signed_provenance_receipt(
    receipt: &ExternalPoolAdapterArtifactSignedProvenanceReceipt,
) -> Result<()> {
    if receipt.schema != ARTIFACT_SIGNED_PROVENANCE_RECEIPT_SCHEMA
        || receipt.canonicalization != ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION
        || receipt.digest_algorithm != ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM
    {
        bail!("signed-provenance receipt metadata is unsupported");
    }
    validate_exact(&receipt.provenance_receipt_id, "provenance receipt ID", 160)?;
    validate_digest(
        &receipt.provenance_receipt_digest,
        "provenance receipt digest",
    )?;
    validate_digest(
        &receipt.verification_material_digest,
        "verification material digest",
    )?;
    let material = &receipt.provenance;
    validate_signature_binding(&material.binding)?;
    validate_digest(
        &material.signature_message_digest,
        "signature message digest",
    )?;
    validate_digest(&material.signature_digest, "signature digest")?;
    validate_exact(
        &material.verified_by_admin_user_id,
        "verifying administrator",
        160,
    )?;
    validate_exact(&material.idempotency_scope, "idempotency scope", 200)?;
    validate_exact(&material.idempotency_key, "idempotency key", 160)?;
    canonical_nanos(&material.verified_at)?;
    if material.recorded_at != material.verified_at
        || material.confirmation != ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION
        || material.evidence_scope != ARTIFACT_SIGNED_PROVENANCE_EVIDENCE_SCOPE
        || material.artifact_ref_resolution_effect != ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT
        || material.artifact_format_effect != ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT
        || material.conformance_effect != ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT
        || material.adapter_effect != ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT
        || material.route_effect != ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT
    {
        bail!("signed-provenance effects are not exact");
    }
    let signature = STANDARD.decode(&material.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != material.signature_base64
        || hex::encode(Sha256::digest(&signature)) != material.signature_digest
        || signature_challenge(material.binding.clone())?.signature_message_digest
            != material.signature_message_digest
        || verification_material_digest(material)? != receipt.verification_material_digest
        || canonical_signed_provenance_receipt_json_and_digest(receipt)?.1
            != receipt.provenance_receipt_digest
    {
        bail!("signed-provenance cryptographic material is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_exact(value: &str, label: &str, maximum: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

pub(crate) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("signed-provenance timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
