use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::rand_core::{OsRng, RngCore};
use rusqlite::{Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        external_pool_adapter_credential_reattestation::*,
        external_pool_adapter_credential_verification::{
            credential_locator_commitment, credential_ref_scheme,
            validate_credential_verification_draft,
        },
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
    active_subject::current_projected_active_registry_subject_on, persistence::insert_challenge,
    read::head_by_provider_binding_on, types::*,
};

impl Store {
    pub(crate) fn issue_external_pool_adapter_credential_reattestation_challenge(
        &self,
        input: GetExternalPoolAdapterCredentialReattestationChallenge,
    ) -> Result<ExternalPoolAdapterCredentialReattestationChallenge> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let issued = Utc::now();
        validate_runtime_window(&input.draft, issued)?;
        let challenge = build(&tx, input, issued)?;
        let json = canonical_credential_reattestation_json(&challenge)?;
        insert_challenge(&tx, &challenge, &json)?;
        tx.commit()?;
        Ok(challenge)
    }
}

fn build(
    tx: &Transaction<'_>,
    input: GetExternalPoolAdapterCredentialReattestationChallenge,
    issued: chrono::DateTime<Utc>,
) -> Result<ExternalPoolAdapterCredentialReattestationChallenge> {
    let provider_binding = historical_external_pool_adapter_registry_provider_binding_authority_on(
        tx,
        &input.provider_binding_id,
        &input.expected_provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V249 Provider binding was not found"))?;
    let provider_binding_receipt = provider_binding.binding();
    let provider_binding_item = &provider_binding_receipt.binding;
    let checked_at = issued.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let release = current_external_pool_adapter_registry_release_authority_on(
        tx,
        &provider_binding_item.registry_release_id,
        &input.expected_registry_release_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V249 registry release was not found"))?;
    let release_receipt = release.release();
    if release.checked_at() != checked_at {
        bail!("V249 neutral release used a different checked_at anchor");
    }
    let release_item = &release_receipt.release;
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        tx,
        &provider_binding_item.application_id,
        &provider_binding_item.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V221 credential locator was not found"))?;
    let legacy = external_pool_adapter_credential_verification_receipt_authority_on(
        tx,
        &provider_binding_item.credential_verification_receipt_id,
        &provider_binding_item.credential_verification_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical V243 credential receipt was not found"))?;
    ensure_upstream_lineage(
        tx,
        provider_binding_receipt,
        release_receipt,
        &onboarding,
        legacy.receipt(),
    )?;
    let key = current_credential_verifier_key_authority_on(
        tx,
        &input.credential_verifier_key_record_id,
        &input.expected_credential_verifier_key_record_digest,
        &input.expected_credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V242 credential verifier key was not found"))?;
    let verifier_intent = &release_item.credential_verifier;
    let verifier = current_credential_verifier_authority_on(
        tx,
        key.verifier_record_id(),
        key.verifier_record_digest(),
        &verifier_intent.verification_kind,
        &verifier_intent.verifier_id,
        verifier_intent.verifier_revision,
        &verifier_intent.verifier_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V241 credential verifier was not found"))?;
    if key.verifier_record_id() != verifier.verifier_record_id()
        || key.verifier_record_digest() != verifier.verifier_record_digest()
        || key.verification_kind() != verifier_intent.verification_kind
        || key.verifier_id() != verifier_intent.verifier_id
        || key.verifier_revision() != verifier_intent.verifier_revision
        || key.verifier_digest() != verifier_intent.verifier_digest
    {
        bail!("current V241/V242 verifier lineage is not exact");
    }
    let current_provider = current_registered_provider_on(tx, &provider_binding_item.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("live Provider was not found"))?;
    ensure_observed_provider(
        tx,
        provider_binding_item,
        &onboarding,
        &current_provider,
        &checked_at,
    )?;
    let current = &current_provider.provider;
    let settlement_account_id = current
        .settlement_account_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("live Provider lacks a settlement account"))?;
    let predecessor = head_by_provider_binding_on(tx, &input.provider_binding_id)?;
    let sequence = predecessor
        .as_ref()
        .map(|stored| {
            stored
                .receipt
                .reattestation
                .binding
                .sequence
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("credential re-attestation sequence overflow"))
        })
        .transpose()?
        .unwrap_or(1);
    let mut nonce = [0_u8; 32];
    OsRng.fill_bytes(&mut nonce);
    let legacy_binding = &legacy.receipt().verification.binding;
    let binding = ExternalPoolAdapterCredentialReattestationBinding {
        schema: CREDENTIAL_REATTESTATION_BINDING_SCHEMA.into(),
        challenge_id: new_id("external_pool_adapter_credential_reattestation_challenge"),
        challenge_nonce_base64: STANDARD.encode(nonce),
        challenge_nonce_digest: hex::encode(Sha256::digest(nonce)),
        challenge_issued_at: checked_at,
        challenge_expires_at: (issued
            + Duration::minutes(CREDENTIAL_REATTESTATION_CHALLENGE_VALIDITY_MINUTES))
        .to_rfc3339_opts(SecondsFormat::Nanos, true),
        provider_binding_id: provider_binding_receipt.provider_binding_id.clone(),
        provider_binding_digest: provider_binding_receipt.provider_binding_digest.clone(),
        provider_binding_material_digest: provider_binding_receipt
            .provider_binding_material_digest
            .clone(),
        registry_release_id: release_receipt.registry_release_id.clone(),
        registry_release_digest: release_receipt.registry_release_digest.clone(),
        registry_release_material_digest: release_receipt.registry_release_material_digest.clone(),
        route_adapter_projection_id: provider_binding_item.route_adapter_projection_id.clone(),
        installation_receipt_id: provider_binding_item.installation_receipt_id.clone(),
        installation_receipt_digest: provider_binding_item.installation_receipt_digest.clone(),
        installation_content_digest: provider_binding_item.installation_content_digest.clone(),
        application_id: provider_binding_item.application_id.clone(),
        application_digest: provider_binding_item.application_digest.clone(),
        adoption_receipt_id: provider_binding_item.adoption_receipt_id.clone(),
        adoption_receipt_digest: provider_binding_item.adoption_receipt_digest.clone(),
        provider_id: current.provider_id.clone(),
        provider_kind: current.provider_kind.clone(),
        provider_owner_account_id: current.owner_account_id.clone(),
        observed_settlement_account_id: settlement_account_id,
        observed_provider_policy_revision: current.policy_revision,
        observed_provider_digest: current_provider.provider_digest,
        observed_provider_status: current.status.clone(),
        adapter_id: provider_binding_item.adapter_id.clone(),
        release_version: provider_binding_item.release_version.clone(),
        adapter_config_revision: provider_binding_item.adapter_config_revision,
        adapter_config_digest: provider_binding_item.adapter_config_digest.clone(),
        admission_id: provider_binding_item.admission_id.clone(),
        admission_digest: provider_binding_item.admission_digest.clone(),
        legacy_credential_verification_receipt_id: legacy
            .receipt()
            .credential_verification_receipt_id
            .clone(),
        legacy_credential_verification_receipt_digest: legacy
            .receipt()
            .credential_verification_receipt_digest
            .clone(),
        credential_ref_scheme: legacy_binding.credential_ref_scheme.clone(),
        credential_locator_commitment: legacy_binding.credential_locator_commitment.clone(),
        expected_credential_verifier: verifier_intent.clone(),
        credential_verifier_digest: release_item.credential_verifier_digest.clone(),
        credential_verifier_key_record_id: key.key_record_id().into(),
        credential_verifier_key_record_digest: key.key_record_digest().into(),
        credential_verifier_key_id: key.key_id().into(),
        credential_verifier_record_id: key.verifier_record_id().into(),
        credential_verifier_record_digest: key.verifier_record_digest().into(),
        signature_algorithm: CREDENTIAL_REATTESTATION_SIGNATURE_ALGORITHM.into(),
        verification_policy_id: CREDENTIAL_REATTESTATION_POLICY_ID.into(),
        sequence,
        predecessor_receipt_id: predecessor
            .as_ref()
            .map(|item| item.receipt.reattestation_receipt_id.clone()),
        predecessor_receipt_digest: predecessor
            .as_ref()
            .map(|item| item.receipt.reattestation_receipt_digest.clone()),
        verifier_report_id: input.draft.verifier_report_id,
        verification_started_at: input.draft.verification_started_at,
        verification_completed_at: input.draft.verification_completed_at,
        report_generated_at: input.draft.report_generated_at,
        report_expires_at: input.draft.report_expires_at,
        credential_resolution_outcome: input.draft.credential_resolution_outcome,
        provider_authentication_outcome: input.draft.provider_authentication_outcome,
        provider_response_evidence_digest: input.draft.provider_response_evidence_digest,
    };
    validate_credential_reattestation_binding(&binding)?;
    credential_reattestation_challenge(binding)
}

pub(super) fn ensure_upstream_lineage(
    tx: &Transaction<'_>,
    provider_binding: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingReceipt,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    onboarding: &crate::store::compute_external_pool_onboarding::HistoricalExternalPoolOnboardingApplicationAuthority,
    legacy: &crate::compute_federation::external_pool_adapter_credential_verification::ExternalPoolAdapterCredentialVerificationReceipt,
) -> Result<()> {
    let b = &provider_binding.binding;
    let r = &release.release;
    let v = &legacy.verification.binding;
    let locator = onboarding.non_bearer_credential_ref();
    if external_pool_adapter_installation_is_revoked_on(tx, &b.installation_receipt_id)?
        || external_pool_adapter_adoption_is_revoked_on(tx, &b.adoption_receipt_id)?
    {
        bail!("V249 Provider binding has a V244/V247 terminal");
    }
    if release.registry_release_id != b.registry_release_id
        || release.registry_release_digest != b.registry_release_digest
        || r.adapter_id != b.adapter_id
        || r.release_version != b.release_version
        || r.admission_id != b.admission_id
        || r.admission_digest != b.admission_digest
        || r.installation_content_digest != b.installation_content_digest
        || r.credential_verifier_digest != r.credential_verifier.verifier_digest
        || onboarding.application_id() != b.application_id
        || onboarding.application_digest() != b.application_digest
        || onboarding.provider().provider_id != b.provider_id
        || onboarding.provider().owner_account_id != b.provider_owner_account_id
        || onboarding.provider_digest() != b.provider_digest
        || onboarding.adapter_id() != b.adapter_id
        || onboarding.adapter_release_version() != b.release_version
        || onboarding.adapter_config_revision() != b.adapter_config_revision
        || onboarding.adapter_config_digest() != b.adapter_config_digest
        || credential_ref_scheme(locator)? != v.credential_ref_scheme
        || credential_locator_commitment(locator) != b.credential_locator_commitment
        || v.credential_locator_commitment != b.credential_locator_commitment
        || legacy.credential_verification_receipt_id != b.credential_verification_receipt_id
        || legacy.credential_verification_receipt_digest != b.credential_verification_receipt_digest
        || v.application_id != b.application_id
        || v.application_digest != b.application_digest
        || v.provider_id != b.provider_id
        || v.provider_owner_account_id != b.provider_owner_account_id
        || v.provider_policy_revision != b.provider_policy_revision
        || v.provider_digest != b.provider_digest
        || v.adapter_id != b.adapter_id
        || v.adapter_release_version != b.release_version
        || v.adapter_config_revision != b.adapter_config_revision
        || v.adapter_config_digest != b.adapter_config_digest
        || v.admission_id != b.admission_id
        || v.admission_digest != b.admission_digest
        || v.expected_credential_verifier != r.credential_verifier
    {
        bail!("credential re-attestation upstream lineage is not exact");
    }
    Ok(())
}

fn ensure_observed_provider(
    tx: &Transaction<'_>,
    binding: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingMaterial,
    onboarding: &crate::store::compute_external_pool_onboarding::HistoricalExternalPoolOnboardingApplicationAuthority,
    current: &crate::store::compute_provider_registry::ComputeProviderRegistrationReceipt,
    checked_at: &str,
) -> Result<()> {
    let provider = &current.provider;
    let historical = onboarding.provider();
    let adapter = provider.adapter.as_ref();
    let registering_exact = provider.status == PROVIDER_STATUS_REGISTERING
        && provider.policy_revision == binding.provider_policy_revision
        && current.provider_digest == binding.provider_digest
        && adapter.map(|item| item.adapter_id.as_str()) == Some(binding.adapter_id.as_str());
    let projected_active_exact = provider.status == PROVIDER_STATUS_ACTIVE
        && current_projected_active_registry_subject_on(tx, binding, current, checked_at)?
            .is_some();
    if provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider.provider_id != historical.provider_id
        || provider.owner_account_id != historical.owner_account_id
        || provider.created_at != historical.created_at
        || adapter.map(|item| item.adapter_version.as_str())
            != Some(binding.release_version.as_str())
        || adapter.map(|item| item.config_revision) != Some(binding.adapter_config_revision)
        || adapter.map(|item| item.config_digest.as_str())
            != Some(binding.adapter_config_digest.as_str())
        || !(registering_exact || projected_active_exact)
    {
        bail!("live Provider is not the exact registering observation");
    }
    Ok(())
}

fn validate_input(input: &GetExternalPoolAdapterCredentialReattestationChallenge) -> Result<()> {
    validate_credential_verification_draft(&input.draft)?;
    for value in [
        &input.provider_binding_id,
        &input.credential_verifier_key_record_id,
    ] {
        if value.trim() != value || value.is_empty() || value.chars().count() > 200 {
            bail!("credential re-attestation challenge identifier is invalid");
        }
    }
    for value in [
        &input.expected_provider_binding_digest,
        &input.expected_registry_release_digest,
        &input.expected_credential_verifier_key_record_digest,
        &input.expected_credential_verifier_key_id,
    ] {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("credential re-attestation challenge digest is invalid");
        }
    }
    Ok(())
}

fn validate_runtime_window(
    draft: &crate::compute_federation::external_pool_adapter_credential_verification::ExternalPoolAdapterCredentialVerificationDraft,
    issued: chrono::DateTime<Utc>,
) -> Result<()> {
    for value in [
        &draft.verification_started_at,
        &draft.verification_completed_at,
        &draft.report_generated_at,
    ] {
        if chrono::DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc)
            > issued + Duration::minutes(5)
        {
            bail!("credential re-attestation report is future-dated");
        }
    }
    if chrono::DateTime::parse_from_rfc3339(&draft.report_expires_at)?.with_timezone(&Utc) <= issued
    {
        bail!("credential re-attestation report is stale");
    }
    Ok(())
}
