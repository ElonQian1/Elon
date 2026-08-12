use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{SecondsFormat, Utc};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_sandbox_reattestation::*,
    store::{
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_release_authority_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        compute_external_pool_adapter_vulnerability_reattestation::current_external_pool_adapter_vulnerability_reattestation_authority_on,
        new_id, Store,
    },
};

use super::{challenge_audit::challenge_by_id_on, persistence::*, read::*, types::*};

impl Store {
    pub(crate) fn create_external_pool_adapter_sandbox_reattestation(
        &self,
        input: CreateExternalPoolAdapterSandboxReattestation,
    ) -> Result<ExternalPoolAdapterSandboxReattestationWriteReceipt> {
        validate_create_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            receipt_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_replay(&stored, &input)?;
            let output = ExternalPoolAdapterSandboxReattestationWriteReceipt {
                reattestation: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        let challenge = challenge_by_id_on(&tx, &input.challenge_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation challenge was not found"))?;
        let now = Utc::now();
        let challenge_expires =
            chrono::DateTime::parse_from_rfc3339(&challenge.binding.challenge_expires_at)?
                .with_timezone(&Utc);
        let report_expires =
            chrono::DateTime::parse_from_rfc3339(&challenge.binding.report_expires_at)?
                .with_timezone(&Utc);
        if now >= challenge_expires
            || now >= report_expires
            || challenge.signature_message_digest != input.expected_signature_message_digest
            || receipt_by_challenge_on(&tx, &input.challenge_id)?.is_some()
        {
            bail!("sandbox re-attestation challenge is stale or already consumed");
        }
        ensure_current_roots(&tx, &challenge, now)?;
        ensure_current_head(&tx, &challenge.binding)?;
        verify_signature(&tx, &challenge, &input.signature_base64)?;
        let timestamp = now.to_rfc3339_opts(SecondsFormat::Nanos, true);
        let signature = STANDARD.decode(&input.signature_base64)?;
        let material = ExternalPoolAdapterSandboxReattestationMaterial {
            binding: challenge.binding,
            signature_message_digest: challenge.signature_message_digest,
            signature_base64: input.signature_base64,
            signature_digest: hex::encode(Sha256::digest(signature)),
            recorded_by_admin_user_id: input.recorded_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            verified_at: timestamp.clone(),
            recorded_at: timestamp,
            evidence_scope: SANDBOX_REATTESTATION_EVIDENCE_SCOPE.into(),
            sandbox_reattestation_effect: SANDBOX_REATTESTATION_EFFECT.into(),
            adapter_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            provider_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            credential_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            route_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            execution_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
            settlement_effect: SANDBOX_REATTESTATION_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterSandboxReattestationReceipt {
            schema: SANDBOX_REATTESTATION_RECEIPT_SCHEMA.into(),
            reattestation_receipt_id: new_id("external_pool_adapter_sandbox_reattestation"),
            reattestation_receipt_digest: String::new(),
            reattestation_material_digest: sandbox_reattestation_material_digest(&material)?,
            canonicalization: SANDBOX_REATTESTATION_CANONICALIZATION.into(),
            digest_algorithm: SANDBOX_REATTESTATION_DIGEST_ALGORITHM.into(),
            reattestation: material,
        };
        receipt.reattestation_receipt_digest =
            sandbox_reattestation_receipt_json_and_digest(&receipt)?.1;
        validate_sandbox_reattestation_receipt(&receipt)?;
        let (json, _) = sandbox_reattestation_receipt_json_and_digest(&receipt)?;
        insert_receipt(&tx, &receipt, &json)?;
        let stored = receipt_by_id_on(&tx, &receipt.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation disappeared"))?;
        let output = ExternalPoolAdapterSandboxReattestationWriteReceipt {
            reattestation: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

fn ensure_current_roots(
    tx: &Transaction<'_>,
    challenge: &ExternalPoolAdapterSandboxReattestationChallenge,
    now: chrono::DateTime<Utc>,
) -> Result<()> {
    let b = &challenge.binding;
    let checked_at = now.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let release = current_external_pool_adapter_registry_release_authority_on(
        tx,
        &b.registry_release_id,
        &b.registry_release_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V249 registry release was not found"))?;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        tx,
        &b.registry_release_id,
        &b.vulnerability_reattestation_receipt_id,
        &b.vulnerability_reattestation_receipt_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V250 vulnerability authority was not found"))?;
    if release.checked_at() != checked_at || vulnerability.checked_at() != checked_at {
        bail!("sandbox re-attestation roots used different checked_at anchors");
    }
    audit_root_receipts(release.release(), vulnerability.receipt(), b)?;
    let verifier = current_sandbox_verifier_key_authority_on(
        tx,
        &b.sandbox_verifier_key_record_id,
        &b.sandbox_verifier_key_record_digest,
        &b.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active V237 verifier was not found"))?;
    if verifier.verifier_operator() != b.sandbox_verifier_operator
        || verifier.verifier_product() != b.sandbox_verifier_product
    {
        bail!("sandbox re-attestation verifier root drifted");
    }
    Ok(())
}

fn ensure_current_head(
    tx: &Transaction<'_>,
    b: &ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    let head = head_by_release_on(tx, &b.registry_release_id)?;
    let expected = head
        .as_ref()
        .map(|stored| {
            Ok::<_, anyhow::Error>((
                stored.receipt.reattestation_receipt_id.as_str(),
                stored.receipt.reattestation_receipt_digest.as_str(),
                stored
                    .receipt
                    .reattestation
                    .binding
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation sequence overflow"))?,
            ))
        })
        .transpose()?;
    let drifted = match expected {
        None => {
            b.sequence != 1
                || b.predecessor_receipt_id.is_some()
                || b.predecessor_receipt_digest.is_some()
        }
        Some((id, digest, sequence)) => {
            b.sequence != sequence
                || b.predecessor_receipt_id.as_deref() != Some(id)
                || b.predecessor_receipt_digest.as_deref() != Some(digest)
        }
    };
    if drifted {
        bail!("sandbox re-attestation predecessor head changed after challenge");
    }
    Ok(())
}

fn audit_root_receipts(
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    vulnerability: &crate::compute_federation::external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationReceipt,
    b: &ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    let r = &release.release;
    let v = &vulnerability.reattestation.binding;
    if release.registry_release_id != b.registry_release_id
        || release.registry_release_digest != b.registry_release_digest
        || release.registry_release_material_digest != b.registry_release_material_digest
        || r.admission_id != b.admission_id
        || r.admission_digest != b.admission_digest
        || r.package_receipt_id != b.package_receipt_id
        || r.package_receipt_digest != b.package_receipt_digest
        || r.source_receipt_id != b.source_receipt_id
        || r.source_receipt_digest != b.source_receipt_digest
        || r.adapter_id != b.adapter_id
        || r.release_version != b.release_version
        || r.route_kind != b.route_kind
        || r.supported_provider_kinds != b.supported_provider_kinds
        || r.implementation_digest != b.implementation_digest
        || r.declared_implementation_sha256 != b.declared_implementation_sha256
        || r.supported_capabilities != b.supported_capabilities
        || r.capability_set_digest != b.capability_set_digest
        || r.credential_verifier != b.expected_credential_verifier
        || r.credential_verifier_digest != b.credential_verifier_digest
        || r.archive_sha256 != b.archive_sha256
        || r.archive_size_bytes != b.archive_size_bytes
        || r.manifest_digest != b.manifest_digest
        || r.entry_inventory_digest != b.entry_inventory_digest
        || r.entry_count != b.entry_count
        || r.total_uncompressed_bytes != b.total_uncompressed_bytes
        || r.installation_content_digest != b.installation_content_digest
        || vulnerability.reattestation_receipt_id != b.vulnerability_reattestation_receipt_id
        || vulnerability.reattestation_receipt_digest
            != b.vulnerability_reattestation_receipt_digest
        || vulnerability.reattestation_material_digest
            != b.vulnerability_reattestation_material_digest
        || v.registry_release_id != b.registry_release_id
        || v.registry_release_digest != b.registry_release_digest
        || v.sequence != b.vulnerability_reattestation_sequence
        || vulnerability.reattestation.verified_at != b.vulnerability_reattestation_verified_at
        || v.intelligence.snapshot_digest != b.vulnerability_intelligence_snapshot_digest
        || v.intelligence.expires_at != b.vulnerability_intelligence_expires_at
        || v.security_receipt_id != b.security_receipt_id
        || v.security_receipt_digest != b.security_receipt_digest
        || v.security_material_digest != b.security_material_digest
        || v.sbom_digest != b.sbom_digest
        || v.component_inventory_digest != b.component_inventory_digest
        || v.component_count != b.component_count
        || v.dependency_inventory_digest != b.dependency_inventory_digest
    {
        bail!("sandbox re-attestation challenge roots drifted before record");
    }
    Ok(())
}

fn verify_signature(
    tx: &Transaction<'_>,
    challenge: &ExternalPoolAdapterSandboxReattestationChallenge,
    signature_base64: &str,
) -> Result<()> {
    let b = &challenge.binding;
    let verifier = current_sandbox_verifier_key_authority_on(
        tx,
        &b.sandbox_verifier_key_record_id,
        &b.sandbox_verifier_key_record_digest,
        &b.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active V237 verifier was not found"))?;
    let public = RsaPublicKey::from_public_key_pem(verifier.public_key_pem())
        .context("decode current V237 sandbox verifier public key")?;
    let message = STANDARD.decode(&challenge.signature_message_base64)?;
    let signature = STANDARD.decode(signature_base64)?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("sandbox re-attestation RSA signature verification failed")
}

fn validate_create_input(input: &CreateExternalPoolAdapterSandboxReattestation) -> Result<()> {
    for value in [
        &input.challenge_id,
        &input.recorded_by_admin_user_id,
        &input.idempotency_scope,
        &input.idempotency_key,
    ] {
        if value.trim() != value || value.is_empty() || value.chars().count() > 240 {
            bail!("sandbox re-attestation create identifier is invalid");
        }
    }
    if input.expected_signature_message_digest.len() != 64
        || input.confirmation != SANDBOX_REATTESTATION_CONFIRMATION
    {
        bail!("sandbox re-attestation create request is invalid");
    }
    let signature = STANDARD.decode(&input.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != input.signature_base64
    {
        bail!("sandbox re-attestation signature is invalid");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredSandboxReattestation,
    input: &CreateExternalPoolAdapterSandboxReattestation,
) -> Result<()> {
    let item = &stored.receipt.reattestation;
    if item.binding.challenge_id != input.challenge_id
        || item.signature_message_digest != input.expected_signature_message_digest
        || item.signature_base64 != input.signature_base64
        || item.recorded_by_admin_user_id != input.recorded_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("sandbox re-attestation idempotency conflicts with immutable history");
    }
    Ok(())
}
