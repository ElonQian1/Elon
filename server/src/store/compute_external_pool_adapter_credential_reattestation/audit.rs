use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::Connection;
use sha2::Sha256;

use crate::{
    compute_federation::{
        external_pool_adapter_credential_reattestation::*,
        external_pool_adapter_credential_verification::{
            credential_locator_commitment, credential_ref_scheme,
        },
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING},
    },
    store::{
        compute_external_pool_adapter_credential_verification::external_pool_adapter_credential_verification_receipt_authority_on,
        compute_external_pool_adapter_registry::{
            historical_external_pool_adapter_registry_provider_binding_authority_on,
            historical_external_pool_adapter_registry_release_authority_on,
        },
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
    },
};

use super::{
    challenge_audit::challenge_by_id_on,
    receipt_projection_audit::exact_receipt_projection,
    roots::{
        historical_credential_verifier_key_on, historical_credential_verifier_on,
        historical_observed_provider_on,
    },
    types::StoredCredentialReattestation,
};

pub(super) fn audit_receipt(
    conn: &Connection,
    stored: StoredCredentialReattestation,
) -> Result<StoredCredentialReattestation> {
    validate_credential_reattestation_receipt(&stored.receipt)?;
    let (json, digest) = credential_reattestation_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.reattestation;
    let b = &item.binding;
    let challenge = credential_reattestation_challenge(b.clone())?;
    let durable = challenge_by_id_on(conn, &b.challenge_id)?
        .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost durable challenge"))?;
    let provider_binding = historical_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        &b.provider_binding_id,
        &b.provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V249 binding"))?;
    let release = historical_external_pool_adapter_registry_release_authority_on(
        conn,
        &b.registry_release_id,
        &b.registry_release_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V249 release"))?;
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &b.application_id,
        &b.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V221 locator"))?;
    let legacy = external_pool_adapter_credential_verification_receipt_authority_on(
        conn,
        &b.legacy_credential_verification_receipt_id,
        &b.legacy_credential_verification_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V243 receipt"))?;
    let key = historical_credential_verifier_key_on(
        conn,
        &b.credential_verifier_key_record_id,
        &b.credential_verifier_key_record_digest,
        &b.credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V242 key"))?;
    let verifier = historical_credential_verifier_on(
        conn,
        &b.credential_verifier_record_id,
        &b.credential_verifier_record_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V241 verifier"))?;
    let observed = historical_observed_provider_on(
        conn,
        &b.provider_id,
        b.observed_provider_policy_revision,
        &b.observed_provider_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost observed Provider"))?;
    verify_rsa(
        &challenge,
        &item.signature_base64,
        &key.registration.public_key_pem,
    )?;
    audit_lineage(
        conn,
        &stored,
        provider_binding.binding(),
        release.release(),
        &onboarding,
        legacy.receipt(),
        &key,
        &verifier,
        &observed,
    )?;
    if json != stored.receipt_json
        || digest != stored.receipt.reattestation_receipt_digest
        || challenge.signature_message_digest != item.signature_message_digest
        || durable != challenge
        || !exact_receipt_projection(conn, &stored)?
    {
        bail!("credential re-attestation failed exact historical audit");
    }
    Ok(stored)
}

#[allow(clippy::too_many_arguments)]
fn audit_lineage(
    conn: &Connection,
    stored: &StoredCredentialReattestation,
    provider_binding: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingReceipt,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    onboarding: &crate::store::compute_external_pool_onboarding::HistoricalExternalPoolOnboardingApplicationAuthority,
    legacy: &crate::compute_federation::external_pool_adapter_credential_verification::ExternalPoolAdapterCredentialVerificationReceipt,
    key: &crate::compute_federation::external_pool_adapter_credential_verifier_key::CredentialVerifierKeyRecord,
    verifier: &crate::compute_federation::external_pool_adapter_credential_verifier::ExternalPoolAdapterCredentialVerifierRecord,
    observed: &crate::compute_federation::provider::ComputeProvider,
) -> Result<()> {
    let b = &stored.receipt.reattestation.binding;
    let pb = &provider_binding.binding;
    let r = &release.release;
    let v = &legacy.verification.binding;
    let vr = &verifier.registration;
    let adapter = observed.adapter.as_ref();
    let locator = onboarding.non_bearer_credential_ref();
    let observed_allowed = observed.status == PROVIDER_STATUS_REGISTERING
        && observed.policy_revision == pb.provider_policy_revision
        && b.observed_provider_digest == pb.provider_digest;
    if provider_binding.provider_binding_id != b.provider_binding_id
        || provider_binding.provider_binding_digest != b.provider_binding_digest
        || provider_binding.provider_binding_material_digest != b.provider_binding_material_digest
        || release.registry_release_id != b.registry_release_id
        || release.registry_release_digest != b.registry_release_digest
        || release.registry_release_material_digest != b.registry_release_material_digest
        || pb.registry_release_id != b.registry_release_id
        || pb.registry_release_digest != b.registry_release_digest
        || pb.route_adapter_projection_id != b.route_adapter_projection_id
        || pb.installation_receipt_id != b.installation_receipt_id
        || pb.installation_receipt_digest != b.installation_receipt_digest
        || pb.installation_content_digest != b.installation_content_digest
        || pb.application_id != b.application_id
        || pb.application_digest != b.application_digest
        || pb.adoption_receipt_id != b.adoption_receipt_id
        || pb.adoption_receipt_digest != b.adoption_receipt_digest
        || pb.provider_id != b.provider_id
        || pb.provider_owner_account_id != b.provider_owner_account_id
        || pb.adapter_id != b.adapter_id
        || pb.release_version != b.release_version
        || pb.adapter_config_revision != b.adapter_config_revision
        || pb.adapter_config_digest != b.adapter_config_digest
        || pb.admission_id != b.admission_id
        || pb.admission_digest != b.admission_digest
        || pb.credential_verification_receipt_id != b.legacy_credential_verification_receipt_id
        || pb.credential_verification_receipt_digest
            != b.legacy_credential_verification_receipt_digest
        || pb.credential_locator_commitment != b.credential_locator_commitment
        || r.credential_verifier != b.expected_credential_verifier
        || r.credential_verifier_digest != b.credential_verifier_digest
        || r.installation_content_digest != b.installation_content_digest
        || onboarding.provider().provider_id != b.provider_id
        || onboarding.provider().owner_account_id != b.provider_owner_account_id
        || onboarding.application_digest() != b.application_digest
        || credential_ref_scheme(locator)? != b.credential_ref_scheme
        || credential_locator_commitment(locator) != b.credential_locator_commitment
        || v.credential_locator_commitment != b.credential_locator_commitment
        || v.expected_credential_verifier != b.expected_credential_verifier
        || key.registration.verifier_record_id != b.credential_verifier_record_id
        || key.registration.verifier_record_digest != b.credential_verifier_record_digest
        || key.registration.verification_kind != b.expected_credential_verifier.verification_kind
        || key.registration.verifier_id != b.expected_credential_verifier.verifier_id
        || key.registration.verifier_revision != b.expected_credential_verifier.verifier_revision
        || key.registration.verifier_digest != b.expected_credential_verifier.verifier_digest
        || verifier.verifier_record_id != b.credential_verifier_record_id
        || vr.verification_kind != b.expected_credential_verifier.verification_kind
        || vr.verifier_id != b.expected_credential_verifier.verifier_id
        || vr.verifier_revision != b.expected_credential_verifier.verifier_revision
        || vr.verifier_digest != b.expected_credential_verifier.verifier_digest
        || observed.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || observed.provider_id != b.provider_id
        || observed.owner_account_id != b.provider_owner_account_id
        || observed.created_at != onboarding.provider().created_at
        || observed.settlement_account_id.as_deref()
            != Some(b.observed_settlement_account_id.as_str())
        || observed.policy_revision != b.observed_provider_policy_revision
        || observed.status != b.observed_provider_status
        || adapter.map(|x| x.adapter_id.as_str()) != Some(b.adapter_id.as_str())
        || adapter.map(|x| x.adapter_version.as_str()) != Some(b.release_version.as_str())
        || adapter.map(|x| x.config_revision) != Some(b.adapter_config_revision)
        || adapter.map(|x| x.config_digest.as_str()) != Some(b.adapter_config_digest.as_str())
        || !observed_allowed
        || !predecessor_is_exact(conn, stored)?
    {
        bail!("credential re-attestation root lineage is not exact");
    }
    Ok(())
}

fn predecessor_is_exact(conn: &Connection, stored: &StoredCredentialReattestation) -> Result<bool> {
    use super::read::receipt_by_id_on;
    let b = &stored.receipt.reattestation.binding;
    match (
        b.predecessor_receipt_id.as_deref(),
        b.predecessor_receipt_digest.as_deref(),
    ) {
        (None, None) => Ok(b.sequence == 1),
        (Some(id), Some(digest)) => {
            let predecessor = receipt_by_id_on(conn, id)?
                .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost predecessor"))?;
            let p = &predecessor.receipt.reattestation.binding;
            Ok(predecessor.receipt.reattestation_receipt_digest == digest
                && p.provider_binding_id == b.provider_binding_id
                && p.sequence.checked_add(1) == Some(b.sequence))
        }
        _ => Ok(false),
    }
}

fn verify_rsa(
    challenge: &ExternalPoolAdapterCredentialReattestationChallenge,
    signature_base64: &str,
    public_key_pem: &str,
) -> Result<()> {
    let message = STANDARD.decode(&challenge.signature_message_base64)?;
    let signature = STANDARD.decode(signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(public_key_pem)
        .context("decode historical V242 credential verifier public key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("credential re-attestation historical RSA audit failed")
}
