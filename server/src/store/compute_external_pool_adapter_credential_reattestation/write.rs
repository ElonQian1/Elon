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
    compute_federation::{
        external_pool_adapter_credential_reattestation::*,
        provider::{
            PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING,
        },
    },
    store::{
        compute_external_pool_adapter_adoption::external_pool_adapter_adoption_is_revoked_on,
        compute_external_pool_adapter_credential_verification::external_pool_adapter_credential_verification_receipt_authority_on,
        compute_external_pool_adapter_credential_verifier::current_credential_verifier_authority_on,
        compute_external_pool_adapter_credential_verifier_key::current_credential_verifier_key_authority_on,
        compute_external_pool_adapter_installation::external_pool_adapter_installation_is_revoked_on,
        compute_external_pool_adapter_registry::{
            current_external_pool_adapter_registry_release_authority_on,
            historical_external_pool_adapter_registry_provider_binding_authority_on,
        },
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
        compute_provider_registry::current_registered_provider_on,
        new_id, Store,
    },
};

use super::{
    active_subject::historical_projected_active_subject_on, challenge::ensure_upstream_lineage,
    challenge_audit::challenge_by_id_on, persistence::insert_receipt, read::*, types::*,
};

impl Store {
    pub(crate) fn create_external_pool_adapter_credential_reattestation(
        &self,
        input: CreateExternalPoolAdapterCredentialReattestation,
    ) -> Result<ExternalPoolAdapterCredentialReattestationWriteReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            receipt_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_replay(&stored, &input)?;
            let output = ExternalPoolAdapterCredentialReattestationWriteReceipt {
                reattestation: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }

        let challenge = challenge_by_id_on(&tx, &input.challenge_id)?
            .ok_or_else(|| anyhow::anyhow!("credential re-attestation challenge was not found"))?;
        let now = Utc::now();
        let challenge_expires =
            chrono::DateTime::parse_from_rfc3339(&challenge.binding.challenge_expires_at)?
                .with_timezone(&Utc);
        let report_generated =
            chrono::DateTime::parse_from_rfc3339(&challenge.binding.report_generated_at)?
                .with_timezone(&Utc);
        let report_expires =
            chrono::DateTime::parse_from_rfc3339(&challenge.binding.report_expires_at)?
                .with_timezone(&Utc);
        if now < report_generated
            || now >= challenge_expires
            || now >= report_expires
            || challenge.signature_message_digest != input.expected_signature_message_digest
            || receipt_by_challenge_on(&tx, &input.challenge_id)?.is_some()
            || receipt_by_report_on(&tx, &challenge.binding.verifier_report_id)?.is_some()
        {
            bail!("credential re-attestation challenge is stale, inexact, or consumed");
        }
        ensure_current_head(&tx, &challenge.binding)?;
        let public_key = ensure_current_roots(&tx, &challenge, now)?;
        verify_signature(&challenge, &input.signature_base64, &public_key)?;

        let timestamp = now.to_rfc3339_opts(SecondsFormat::Nanos, true);
        let signature = STANDARD.decode(&input.signature_base64)?;
        let material = ExternalPoolAdapterCredentialReattestationMaterial {
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
            evidence_scope: CREDENTIAL_REATTESTATION_EVIDENCE_SCOPE.into(),
            credential_reattestation_effect: CREDENTIAL_REATTESTATION_EFFECT.into(),
            adapter_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            provider_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            route_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            execution_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            usage_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
            settlement_effect: CREDENTIAL_REATTESTATION_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterCredentialReattestationReceipt {
            schema: CREDENTIAL_REATTESTATION_RECEIPT_SCHEMA.into(),
            reattestation_receipt_id: new_id("external_pool_adapter_credential_reattestation"),
            reattestation_receipt_digest: String::new(),
            reattestation_material_digest: credential_reattestation_material_digest(&material)?,
            canonicalization: CREDENTIAL_REATTESTATION_CANONICALIZATION.into(),
            digest_algorithm: CREDENTIAL_REATTESTATION_DIGEST_ALGORITHM.into(),
            reattestation: material,
        };
        receipt.reattestation_receipt_digest =
            credential_reattestation_receipt_json_and_digest(&receipt)?.1;
        validate_credential_reattestation_receipt(&receipt)?;
        let (json, _) = credential_reattestation_receipt_json_and_digest(&receipt)?;
        insert_receipt(&tx, &receipt, &json)?;
        let stored = receipt_by_id_on(&tx, &receipt.reattestation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("credential re-attestation disappeared"))?;
        if stored.receipt != receipt || stored.receipt_json != json {
            bail!("credential re-attestation changed during exact readback");
        }
        let output = ExternalPoolAdapterCredentialReattestationWriteReceipt {
            reattestation: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

fn ensure_current_roots(
    tx: &Transaction<'_>,
    challenge: &ExternalPoolAdapterCredentialReattestationChallenge,
    now: chrono::DateTime<Utc>,
) -> Result<String> {
    let b = &challenge.binding;
    let checked_at = now.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let provider_binding = historical_external_pool_adapter_registry_provider_binding_authority_on(
        tx,
        &b.provider_binding_id,
        &b.provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V249 Provider binding was not found"))?;
    let release = current_external_pool_adapter_registry_release_authority_on(
        tx,
        &b.registry_release_id,
        &b.registry_release_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V249 neutral release was not found"))?;
    if release.checked_at() != checked_at
        || provider_binding.binding().provider_binding_material_digest
            != b.provider_binding_material_digest
        || release.release().registry_release_material_digest != b.registry_release_material_digest
    {
        bail!("credential re-attestation V249 roots are not exact");
    }
    let pb = &provider_binding.binding().binding;
    if external_pool_adapter_installation_is_revoked_on(tx, &pb.installation_receipt_id)?
        || external_pool_adapter_adoption_is_revoked_on(tx, &pb.adoption_receipt_id)?
    {
        bail!("credential re-attestation V244/V247 upstream is terminal");
    }
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        tx,
        &b.application_id,
        &b.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V221 credential locator was not found"))?;
    let legacy = external_pool_adapter_credential_verification_receipt_authority_on(
        tx,
        &b.legacy_credential_verification_receipt_id,
        &b.legacy_credential_verification_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V243 credential receipt was not found"))?;
    ensure_upstream_lineage(
        tx,
        provider_binding.binding(),
        release.release(),
        &onboarding,
        legacy.receipt(),
    )?;

    let key = current_credential_verifier_key_authority_on(
        tx,
        &b.credential_verifier_key_record_id,
        &b.credential_verifier_key_record_digest,
        &b.credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V242 credential verifier key was not found"))?;
    let verifier = current_credential_verifier_authority_on(
        tx,
        &b.credential_verifier_record_id,
        &b.credential_verifier_record_digest,
        &b.expected_credential_verifier.verification_kind,
        &b.expected_credential_verifier.verifier_id,
        b.expected_credential_verifier.verifier_revision,
        &b.expected_credential_verifier.verifier_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V241 credential verifier was not found"))?;
    if key.verifier_record_id() != verifier.verifier_record_id()
        || key.verifier_record_digest() != verifier.verifier_record_digest()
        || key.verification_kind() != b.expected_credential_verifier.verification_kind
        || key.verifier_id() != b.expected_credential_verifier.verifier_id
        || key.verifier_revision() != b.expected_credential_verifier.verifier_revision
        || key.verifier_digest() != b.expected_credential_verifier.verifier_digest
    {
        bail!("credential re-attestation current V241/V242 lineage drifted");
    }
    ensure_exact_observed_provider(tx, b, &onboarding)?;
    Ok(key.public_key_pem().to_string())
}

fn ensure_exact_observed_provider(
    tx: &Transaction<'_>,
    b: &ExternalPoolAdapterCredentialReattestationBinding,
    onboarding: &crate::store::compute_external_pool_onboarding::HistoricalExternalPoolOnboardingApplicationAuthority,
) -> Result<()> {
    let current = current_registered_provider_on(tx, &b.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("live Provider was not found"))?;
    let provider = &current.provider;
    let adapter = provider.adapter.as_ref();
    let registering_exact = provider.status == PROVIDER_STATUS_REGISTERING
        && adapter.map(|item| item.adapter_id.as_str()) == Some(b.adapter_id.as_str());
    let projected_active_exact = provider.status == PROVIDER_STATUS_ACTIVE
        && adapter.map(|item| item.adapter_id.as_str())
            == Some(b.route_adapter_projection_id.as_str())
        && historical_projected_active_subject_on(tx, b)?.is_some();
    if current.provider_digest != b.observed_provider_digest
        || provider.policy_revision != b.observed_provider_policy_revision
        || provider.status != b.observed_provider_status
        || provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider.provider_id != b.provider_id
        || provider.owner_account_id != b.provider_owner_account_id
        || provider.created_at != onboarding.provider().created_at
        || provider.settlement_account_id.as_deref()
            != Some(b.observed_settlement_account_id.as_str())
        || adapter.map(|item| item.adapter_version.as_str()) != Some(b.release_version.as_str())
        || adapter.map(|item| item.config_revision) != Some(b.adapter_config_revision)
        || adapter.map(|item| item.config_digest.as_str()) != Some(b.adapter_config_digest.as_str())
        || !(registering_exact || projected_active_exact)
    {
        bail!("live Provider no longer matches the exact challenge observation");
    }
    Ok(())
}

fn ensure_current_head(
    tx: &Transaction<'_>,
    b: &ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<()> {
    let head = head_by_provider_binding_on(tx, &b.provider_binding_id)?;
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
                    .ok_or_else(|| {
                        anyhow::anyhow!("credential re-attestation sequence overflow")
                    })?,
            ))
        })
        .transpose()?;
    let exact = match expected {
        None => b.sequence == 1 && b.predecessor_receipt_id.is_none(),
        Some((id, digest, sequence)) => {
            b.sequence == sequence
                && b.predecessor_receipt_id.as_deref() == Some(id)
                && b.predecessor_receipt_digest.as_deref() == Some(digest)
        }
    };
    if !exact {
        bail!("credential re-attestation predecessor head changed after challenge");
    }
    Ok(())
}

fn verify_signature(
    challenge: &ExternalPoolAdapterCredentialReattestationChallenge,
    signature_base64: &str,
    public_key_pem: &str,
) -> Result<()> {
    let public = RsaPublicKey::from_public_key_pem(public_key_pem)
        .context("decode current V242 credential verifier public key")?;
    let message = STANDARD.decode(&challenge.signature_message_base64)?;
    let signature = STANDARD.decode(signature_base64)?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("credential re-attestation RSA verification failed")
}

fn validate_input(input: &CreateExternalPoolAdapterCredentialReattestation) -> Result<()> {
    identifier(&input.challenge_id, 200)?;
    identifier(&input.recorded_by_admin_user_id, 200)?;
    identifier(&input.idempotency_scope, 240)?;
    identifier(&input.idempotency_key, 240)?;
    digest(&input.expected_signature_message_digest)?;
    if input.confirmation != CREDENTIAL_REATTESTATION_CONFIRMATION {
        bail!("credential re-attestation confirmation is invalid");
    }
    let signature = STANDARD.decode(&input.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != input.signature_base64
    {
        bail!("credential re-attestation signature is invalid");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredCredentialReattestation,
    input: &CreateExternalPoolAdapterCredentialReattestation,
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
        bail!("credential re-attestation idempotency conflicts with immutable history");
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("credential re-attestation identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential re-attestation digest is invalid");
    }
    Ok(())
}
