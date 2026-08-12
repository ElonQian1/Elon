use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
    RsaPublicKey,
};
use rusqlite::{params, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_credential_verification::*,
    store::{
        compute_external_pool_adapter_credential_verifier_key::current_credential_verifier_key_authority_on,
        compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
        compute_external_pool_onboarding::current_external_pool_onboarding_application_authority_on,
        new_id, Store,
    },
};

use super::{
    read::{receipt_by_idempotency_on, receipt_by_report_on},
    types::{
        write_receipt, CreateExternalPoolAdapterCredentialVerification,
        ExternalPoolAdapterCredentialVerificationWriteReceipt,
        GetExternalPoolAdapterCredentialVerificationChallenge,
    },
};

impl Store {
    pub(crate) fn external_pool_adapter_credential_verification_challenge(
        &self,
        input: GetExternalPoolAdapterCredentialVerificationChallenge,
    ) -> Result<ExternalPoolAdapterCredentialVerificationChallenge> {
        validate_challenge_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let challenge = credential_verification_challenge(exact_binding(&transaction, &input)?)?;
        transaction.commit()?;
        Ok(challenge)
    }

    pub(crate) fn create_external_pool_adapter_credential_verification(
        &self,
        input: CreateExternalPoolAdapterCredentialVerification,
    ) -> Result<ExternalPoolAdapterCredentialVerificationWriteReceipt> {
        validate_create_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = receipt_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored.receipt, &input)?;
            let output = write_receipt(&stored, true);
            transaction.commit()?;
            return Ok(output);
        }
        if receipt_by_report_on(&transaction, &input.challenge.draft.verifier_report_id)?.is_some()
        {
            bail!("credential verifier report ID already exists");
        }
        let binding = exact_binding(&transaction, &input.challenge)?;
        let challenge = credential_verification_challenge(binding.clone())?;
        if challenge.signature_message_digest != input.expected_signature_message_digest {
            bail!("credential verification signature challenge is stale");
        }
        verify_signature(&transaction, &input, &challenge.signature_message_base64)?;
        let signature = STANDARD.decode(&input.signature_base64)?;
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterCredentialVerificationMaterial {
            binding,
            signature_message_digest: challenge.signature_message_digest,
            signature_base64: input.signature_base64,
            signature_digest: hex::encode(Sha256::digest(signature)),
            recorded_by_admin_user_id: input.recorded_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            verified_at: timestamp.clone(),
            recorded_at: timestamp,
            evidence_scope: CREDENTIAL_VERIFICATION_EVIDENCE_SCOPE.to_string(),
            credential_effect: CREDENTIAL_VERIFICATION_EFFECT.to_string(),
            adapter_effect: CREDENTIAL_VERIFICATION_NO_EFFECT.to_string(),
            route_effect: CREDENTIAL_VERIFICATION_NO_EFFECT.to_string(),
            execution_effect: CREDENTIAL_VERIFICATION_NO_EFFECT.to_string(),
            settlement_effect: CREDENTIAL_VERIFICATION_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterCredentialVerificationReceipt {
            schema: CREDENTIAL_VERIFICATION_RECEIPT_SCHEMA.to_string(),
            credential_verification_receipt_id: new_id(
                "external_pool_adapter_credential_verification",
            ),
            credential_verification_receipt_digest: String::new(),
            verification_material_digest: credential_verification_material_digest(&material)?,
            canonicalization: CREDENTIAL_VERIFICATION_CANONICALIZATION.to_string(),
            digest_algorithm: CREDENTIAL_VERIFICATION_DIGEST_ALGORITHM.to_string(),
            verification: material,
        };
        receipt.credential_verification_receipt_digest =
            canonical_credential_verification_receipt_json_and_digest(&receipt)?.1;
        validate_credential_verification_receipt(&receipt)?;
        let (receipt_json, digest) =
            canonical_credential_verification_receipt_json_and_digest(&receipt)?;
        if digest != receipt.credential_verification_receipt_digest {
            bail!("credential verification digest changed before persistence");
        }
        insert_receipt(&transaction, &receipt, &receipt_json)?;
        let stored = receipt_by_report_on(
            &transaction,
            &receipt.verification.binding.verifier_report_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("credential verification disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("credential verification changed during exact readback");
        }
        let output = write_receipt(&stored, false);
        transaction.commit()?;
        Ok(output)
    }
}

fn exact_binding(
    tx: &Transaction<'_>,
    input: &GetExternalPoolAdapterCredentialVerificationChallenge,
) -> Result<ExternalPoolAdapterCredentialVerificationBinding> {
    validate_runtime_window(&input.draft)?;
    let onboarding = current_external_pool_onboarding_application_authority_on(
        tx,
        &input.application_id,
        &input.expected_application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current onboarding application was not found"))?;
    let admission = current_external_pool_adapter_release_admission_authority_on(
        tx,
        &input.admission_id,
        &input.expected_admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current Adapter release admission was not found"))?;
    let verifier = current_credential_verifier_key_authority_on(
        tx,
        &input.credential_verifier_key_record_id,
        &input.expected_credential_verifier_key_record_digest,
        &input.expected_credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("current credential verifier key was not found"))?;
    let provider = onboarding.provider();
    let settlement_account_id = provider
        .settlement_account_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("onboarding Provider lacks settlement account"))?;
    if onboarding.adapter_id() != admission.adapter_id()
        || onboarding.adapter_release_version() != admission.release_version()
        || admission.expected_credential_verifier().verification_kind
            != verifier.verification_kind()
        || admission.expected_credential_verifier().verifier_id != verifier.verifier_id()
        || admission.expected_credential_verifier().verifier_revision
            != verifier.verifier_revision()
        || admission.expected_credential_verifier().verifier_digest != verifier.verifier_digest()
    {
        bail!("credential verification upstream lineage is incompatible");
    }
    let locator = onboarding.non_bearer_credential_ref();
    let binding = ExternalPoolAdapterCredentialVerificationBinding {
        schema: CREDENTIAL_VERIFICATION_BINDING_SCHEMA.to_string(),
        application_id: onboarding.application_id().to_string(),
        application_digest: onboarding.application_digest().to_string(),
        onboarding_applied_at: onboarding.applied_at().to_string(),
        provider_id: provider.provider_id.clone(),
        provider_kind: provider.provider_kind.clone(),
        provider_owner_account_id: provider.owner_account_id.clone(),
        settlement_account_id,
        provider_policy_revision: provider.policy_revision,
        provider_digest: onboarding.provider_digest().to_string(),
        provider_status: provider.status.clone(),
        adapter_id: onboarding.adapter_id().to_string(),
        adapter_release_version: onboarding.adapter_release_version().to_string(),
        adapter_config_revision: onboarding.adapter_config_revision(),
        adapter_config_digest: onboarding.adapter_config_digest().to_string(),
        credential_ref_scheme: credential_ref_scheme(locator)?.to_string(),
        credential_locator_commitment: credential_locator_commitment(locator),
        admission_id: admission.admission_id().to_string(),
        admission_digest: admission.admission_digest().to_string(),
        admission_applied_at: admission.applied_at().to_string(),
        declared_implementation_sha256: admission.declared_implementation_sha256().to_string(),
        capability_set_digest: admission.capability_set_digest().to_string(),
        expected_credential_verifier: admission.expected_credential_verifier().clone(),
        credential_verifier_key_record_id: verifier.key_record_id().to_string(),
        credential_verifier_key_record_digest: verifier.key_record_digest().to_string(),
        credential_verifier_key_id: verifier.key_id().to_string(),
        credential_verifier_record_id: verifier.verifier_record_id().to_string(),
        credential_verifier_record_digest: verifier.verifier_record_digest().to_string(),
        signature_algorithm: CREDENTIAL_VERIFICATION_SIGNATURE_ALGORITHM.to_string(),
        verification_policy_id: CREDENTIAL_VERIFICATION_POLICY_ID.to_string(),
        verifier_report_id: input.draft.verifier_report_id.clone(),
        verification_started_at: input.draft.verification_started_at.clone(),
        verification_completed_at: input.draft.verification_completed_at.clone(),
        report_generated_at: input.draft.report_generated_at.clone(),
        report_expires_at: input.draft.report_expires_at.clone(),
        credential_resolution_outcome: input.draft.credential_resolution_outcome.clone(),
        provider_authentication_outcome: input.draft.provider_authentication_outcome.clone(),
        provider_response_evidence_digest: input.draft.provider_response_evidence_digest.clone(),
    };
    validate_credential_verification_binding(&binding)?;
    Ok(binding)
}

fn validate_runtime_window(draft: &ExternalPoolAdapterCredentialVerificationDraft) -> Result<()> {
    validate_credential_verification_draft(draft)?;
    let now = Utc::now();
    let started =
        chrono::DateTime::parse_from_rfc3339(&draft.verification_started_at)?.with_timezone(&Utc);
    let completed =
        chrono::DateTime::parse_from_rfc3339(&draft.verification_completed_at)?.with_timezone(&Utc);
    let generated =
        chrono::DateTime::parse_from_rfc3339(&draft.report_generated_at)?.with_timezone(&Utc);
    let expires =
        chrono::DateTime::parse_from_rfc3339(&draft.report_expires_at)?.with_timezone(&Utc);
    if started > now + Duration::minutes(5)
        || completed > now + Duration::minutes(5)
        || generated > now + Duration::minutes(5)
        || expires <= now
    {
        bail!("credential verification report is stale or outside its runtime bound");
    }
    Ok(())
}

fn verify_signature(
    tx: &Transaction<'_>,
    input: &CreateExternalPoolAdapterCredentialVerification,
    message_base64: &str,
) -> Result<()> {
    let key = current_credential_verifier_key_authority_on(
        tx,
        &input.challenge.credential_verifier_key_record_id,
        &input
            .challenge
            .expected_credential_verifier_key_record_digest,
        &input.challenge.expected_credential_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active credential verifier key disappeared"))?;
    let message = STANDARD.decode(message_base64)?;
    let signature = STANDARD.decode(&input.signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(key.public_key_pem())
        .context("decode registered credential verifier public key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("credential verification signature failed")
}

fn validate_challenge_input(
    input: &GetExternalPoolAdapterCredentialVerificationChallenge,
) -> Result<()> {
    for value in [
        &input.application_id,
        &input.admission_id,
        &input.credential_verifier_key_record_id,
    ] {
        if value.is_empty() || value.trim() != value || value.len() > 200 {
            bail!("credential verification challenge identifier is invalid");
        }
    }
    for value in [
        &input.expected_application_digest,
        &input.expected_admission_digest,
        &input.expected_credential_verifier_key_record_digest,
        &input.expected_credential_verifier_key_id,
    ] {
        validate_digest(value)?;
    }
    validate_credential_verification_draft(&input.draft)
}

fn validate_create_input(input: &CreateExternalPoolAdapterCredentialVerification) -> Result<()> {
    validate_challenge_input(&input.challenge)?;
    validate_digest(&input.expected_signature_message_digest)?;
    if input.confirmation != CREDENTIAL_VERIFICATION_CONFIRMATION
        || input.recorded_by_admin_user_id.trim().is_empty()
        || input.idempotency_scope.trim().is_empty()
        || input.idempotency_key.trim().is_empty()
    {
        bail!("credential verification create request is invalid");
    }
    let signature = STANDARD.decode(&input.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != input.signature_base64
    {
        bail!("credential verifier signature Base64 is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential verification digest is invalid");
    }
    Ok(())
}

fn ensure_replay(
    receipt: &ExternalPoolAdapterCredentialVerificationReceipt,
    input: &CreateExternalPoolAdapterCredentialVerification,
) -> Result<()> {
    let item = &receipt.verification;
    let binding = &item.binding;
    if binding.application_id != input.challenge.application_id
        || binding.application_digest != input.challenge.expected_application_digest
        || binding.admission_id != input.challenge.admission_id
        || binding.admission_digest != input.challenge.expected_admission_digest
        || binding.credential_verifier_key_record_id
            != input.challenge.credential_verifier_key_record_id
        || binding.credential_verifier_key_record_digest
            != input
                .challenge
                .expected_credential_verifier_key_record_digest
        || binding.credential_verifier_key_id != input.challenge.expected_credential_verifier_key_id
        || binding.verifier_report_id != input.challenge.draft.verifier_report_id
        || binding.verification_started_at != input.challenge.draft.verification_started_at
        || binding.verification_completed_at != input.challenge.draft.verification_completed_at
        || binding.report_generated_at != input.challenge.draft.report_generated_at
        || binding.report_expires_at != input.challenge.draft.report_expires_at
        || binding.credential_resolution_outcome
            != input.challenge.draft.credential_resolution_outcome
        || binding.provider_authentication_outcome
            != input.challenge.draft.provider_authentication_outcome
        || binding.provider_response_evidence_digest
            != input.challenge.draft.provider_response_evidence_digest
        || item.signature_message_digest != input.expected_signature_message_digest
        || item.signature_base64 != input.signature_base64
        || item.recorded_by_admin_user_id != input.recorded_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("credential verification idempotency key conflicts with immutable history");
    }
    Ok(())
}

fn insert_receipt(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterCredentialVerificationReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.verification;
    let binding = &item.binding;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_credential_verification_receipts(
          credential_verification_receipt_id,credential_verification_receipt_digest,receipt_json,
          verification_material_digest,application_id,application_digest,provider_id,
          provider_policy_revision,provider_digest,adapter_id,adapter_release_version,
          adapter_config_revision,adapter_config_digest,credential_ref_scheme,
          credential_locator_commitment,admission_id,admission_digest,
          credential_verifier_key_record_id,credential_verifier_key_record_digest,
          credential_verifier_key_id,credential_verifier_record_id,
          credential_verifier_record_digest,verifier_report_id,report_expires_at,
          provider_response_evidence_digest,signature_message_digest,signature_base64,
          signature_digest,recorded_by_admin_user_id,confirmation,idempotency_scope,
          idempotency_key,verified_at,recorded_at,evidence_scope,credential_effect,
          adapter_effect,route_effect,execution_effect,settlement_effect
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
                  ?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,
                  ?35,?36,?37,?38,?39,?40)",
        params![
            receipt.credential_verification_receipt_id,
            receipt.credential_verification_receipt_digest,
            json,
            receipt.verification_material_digest,
            binding.application_id,
            binding.application_digest,
            binding.provider_id,
            binding.provider_policy_revision,
            binding.provider_digest,
            binding.adapter_id,
            binding.adapter_release_version,
            binding.adapter_config_revision,
            binding.adapter_config_digest,
            binding.credential_ref_scheme,
            binding.credential_locator_commitment,
            binding.admission_id,
            binding.admission_digest,
            binding.credential_verifier_key_record_id,
            binding.credential_verifier_key_record_digest,
            binding.credential_verifier_key_id,
            binding.credential_verifier_record_id,
            binding.credential_verifier_record_digest,
            binding.verifier_report_id,
            binding.report_expires_at,
            binding.provider_response_evidence_digest,
            item.signature_message_digest,
            item.signature_base64,
            item.signature_digest,
            item.recorded_by_admin_user_id,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.verified_at,
            item.recorded_at,
            item.evidence_scope,
            item.credential_effect,
            item.adapter_effect,
            item.route_effect,
            item.execution_effect,
            item.settlement_effect,
        ],
    )?;
    Ok(())
}
