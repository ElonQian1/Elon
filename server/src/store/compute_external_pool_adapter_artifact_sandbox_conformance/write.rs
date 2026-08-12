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
    compute_federation::external_pool_adapter_artifact_sandbox_conformance::*,
    store::{
        compute_external_pool_adapter_artifact_vulnerability_report::current_vulnerability_report_authority_on,
        compute_external_pool_adapter_release::admission_by_id_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        new_id, Store,
    },
};

use super::{
    read::{receipt_by_admission_on, receipt_by_idempotency_on},
    types::{
        write_receipt, CreateExternalPoolAdapterSandboxConformance,
        ExternalPoolAdapterSandboxConformanceWriteReceipt,
        GetExternalPoolAdapterSandboxConformanceChallenge,
    },
};

impl Store {
    pub(crate) fn external_pool_adapter_sandbox_conformance_challenge(
        &self,
        input: GetExternalPoolAdapterSandboxConformanceChallenge,
    ) -> Result<ExternalPoolAdapterSandboxConformanceChallenge> {
        validate_challenge_input(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let challenge = sandbox_conformance_challenge(exact_binding(&transaction, &input)?)?;
        transaction.commit()?;
        Ok(challenge)
    }

    pub(crate) fn create_external_pool_adapter_sandbox_conformance(
        &self,
        input: CreateExternalPoolAdapterSandboxConformance,
    ) -> Result<ExternalPoolAdapterSandboxConformanceWriteReceipt> {
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
        if receipt_by_admission_on(&transaction, &input.challenge.admission_id)?.is_some() {
            bail!("sandbox conformance already exists for this admission");
        }
        let binding = exact_binding(&transaction, &input.challenge)?;
        let challenge = sandbox_conformance_challenge(binding.clone())?;
        if challenge.signature_message_digest != input.expected_signature_message_digest {
            bail!("sandbox conformance signature challenge is stale");
        }
        verify_signature(&transaction, &input, &challenge.signature_message_base64)?;
        let signature = STANDARD.decode(&input.signature_base64)?;
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterSandboxConformanceMaterial {
            binding,
            signature_message_digest: challenge.signature_message_digest,
            signature_base64: input.signature_base64,
            signature_digest: hex::encode(Sha256::digest(signature)),
            verified_by_admin_user_id: input.verified_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            verified_at: timestamp.clone(),
            recorded_at: timestamp,
            evidence_scope: SANDBOX_CONFORMANCE_EVIDENCE_SCOPE.to_string(),
            conformance_effect: SANDBOX_CONFORMANCE_EFFECT.to_string(),
            credential_effect: SANDBOX_CONFORMANCE_NO_EFFECT.to_string(),
            adapter_effect: SANDBOX_CONFORMANCE_NO_EFFECT.to_string(),
            route_effect: SANDBOX_CONFORMANCE_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterSandboxConformanceReceipt {
            schema: SANDBOX_CONFORMANCE_RECEIPT_SCHEMA.to_string(),
            sandbox_conformance_receipt_id: new_id("external_pool_adapter_sandbox_conformance"),
            sandbox_conformance_receipt_digest: String::new(),
            conformance_material_digest: sandbox_conformance_material_digest(&material)?,
            canonicalization: SANDBOX_CONFORMANCE_CANONICALIZATION.to_string(),
            digest_algorithm: SANDBOX_CONFORMANCE_DIGEST_ALGORITHM.to_string(),
            conformance: material,
        };
        receipt.sandbox_conformance_receipt_digest =
            canonical_sandbox_conformance_receipt_json_and_digest(&receipt)?.1;
        validate_sandbox_conformance_receipt(&receipt)?;
        let (receipt_json, digest) =
            canonical_sandbox_conformance_receipt_json_and_digest(&receipt)?;
        if digest != receipt.sandbox_conformance_receipt_digest {
            bail!("sandbox conformance digest changed before persistence");
        }
        insert_receipt(&transaction, &receipt, &receipt_json)?;
        let stored = receipt_by_admission_on(&transaction, &input.challenge.admission_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox conformance disappeared after insert"))?;
        if stored.receipt != receipt || stored.receipt_json != receipt_json {
            bail!("sandbox conformance changed during exact readback");
        }
        let output = write_receipt(&stored, false);
        transaction.commit()?;
        Ok(output)
    }
}

fn exact_binding(
    tx: &Transaction<'_>,
    input: &GetExternalPoolAdapterSandboxConformanceChallenge,
) -> Result<ExternalPoolAdapterSandboxConformanceBinding> {
    validate_runtime_window(&input.draft)?;
    let admission = admission_by_id_on(tx, &input.admission_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox conformance admission was not found"))?;
    let vulnerability = current_vulnerability_report_authority_on(
        tx,
        &input.admission_id,
        &input.expected_vulnerability_report_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("current vulnerability report was not found"))?;
    let verifier = current_sandbox_verifier_key_authority_on(
        tx,
        &input.sandbox_verifier_key_record_id,
        &input.expected_sandbox_verifier_key_record_digest,
        &input.expected_sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active sandbox verifier key was not found"))?;
    let vulnerability_receipt = vulnerability.receipt();
    let vulnerability_binding = &vulnerability_receipt.report.binding;
    if admission.admission_id != input.admission_id
        || vulnerability_binding.admission_digest != admission.admission_digest
    {
        bail!("sandbox conformance upstream lineage drifted");
    }
    let test_plan = sandbox_capability_test_plan(
        &admission.admission_digest,
        &admission.supported_capabilities,
    )?;
    let policy_violation_count = input
        .draft
        .observations
        .iter()
        .map(|item| item.policy_violation_count)
        .sum::<u64>()
        + input.draft.external_network_attempt_count
        + input.draft.write_outside_ephemeral_count
        + input.draft.child_process_attempt_count;
    let binding = ExternalPoolAdapterSandboxConformanceBinding {
        schema: SANDBOX_CONFORMANCE_BINDING_SCHEMA.to_string(),
        admission_id: admission.admission_id,
        admission_digest: admission.admission_digest,
        adapter_id: admission.adapter_id,
        release_version: admission.release_version,
        declared_implementation_sha256: admission.declared_implementation_sha256,
        supported_capabilities: admission.supported_capabilities,
        capability_set_digest: admission.capability_set_digest,
        expected_credential_verifier: admission.expected_credential_verifier,
        vulnerability_report_receipt_id: vulnerability_receipt
            .vulnerability_report_receipt_id
            .clone(),
        vulnerability_report_receipt_digest: vulnerability_receipt
            .vulnerability_report_receipt_digest
            .clone(),
        security_receipt_digest: vulnerability_binding.security_receipt_digest.clone(),
        package_receipt_digest: vulnerability_binding.package_receipt_digest.clone(),
        archive_sha256: vulnerability_binding.archive_sha256.clone(),
        sbom_digest: vulnerability_binding.sbom_digest.clone(),
        vulnerability_intelligence_expires_at: vulnerability_binding
            .intelligence
            .expires_at
            .clone(),
        vulnerability_report_verified_at: vulnerability_receipt.report.verified_at.clone(),
        sandbox_verifier_key_record_id: verifier.key_record_id().to_string(),
        sandbox_verifier_key_record_digest: verifier.key_record_digest().to_string(),
        sandbox_verifier_key_id: verifier.key_id().to_string(),
        sandbox_verifier_operator: verifier.verifier_operator().to_string(),
        sandbox_verifier_product: verifier.verifier_product().to_string(),
        signature_algorithm: SANDBOX_CONFORMANCE_SIGNATURE_ALGORITHM.to_string(),
        sandbox_policy_id: SANDBOX_CONFORMANCE_POLICY_ID.to_string(),
        verifier_report_id: input.draft.verifier_report_id.clone(),
        sandbox_runtime_id: input.draft.sandbox_runtime_id.clone(),
        runtime_image_digest: input.draft.runtime_image_digest.clone(),
        isolation_profile_id: input.draft.isolation_profile_id.clone(),
        run_started_at: input.draft.run_started_at.clone(),
        run_completed_at: input.draft.run_completed_at.clone(),
        report_generated_at: input.draft.report_generated_at.clone(),
        report_expires_at: input.draft.report_expires_at.clone(),
        external_network_attempt_count: input.draft.external_network_attempt_count,
        write_outside_ephemeral_count: input.draft.write_outside_ephemeral_count,
        child_process_attempt_count: input.draft.child_process_attempt_count,
        peak_memory_bytes: input.draft.peak_memory_bytes,
        cpu_time_ms: input.draft.cpu_time_ms,
        test_plan_digest: sandbox_test_plan_digest(&test_plan)?,
        test_plan,
        observation_inventory_digest: sandbox_observation_inventory_digest(
            &input.draft.observations,
        )?,
        observations: input.draft.observations.clone(),
        passed_capability_count: input
            .draft
            .observations
            .iter()
            .filter(|item| item.outcome == "passed")
            .count() as u64,
        policy_violation_count,
    };
    validate_sandbox_conformance_binding(&binding)?;
    Ok(binding)
}

fn validate_runtime_window(draft: &ExternalPoolAdapterSandboxConformanceDraft) -> Result<()> {
    validate_sandbox_conformance_draft(draft)?;
    let now = Utc::now();
    let started = chrono::DateTime::parse_from_rfc3339(&draft.run_started_at)?.with_timezone(&Utc);
    let completed =
        chrono::DateTime::parse_from_rfc3339(&draft.run_completed_at)?.with_timezone(&Utc);
    let generated =
        chrono::DateTime::parse_from_rfc3339(&draft.report_generated_at)?.with_timezone(&Utc);
    let expires =
        chrono::DateTime::parse_from_rfc3339(&draft.report_expires_at)?.with_timezone(&Utc);
    if started > now + Duration::minutes(5)
        || completed > now + Duration::minutes(5)
        || generated > now + Duration::minutes(5)
        || expires <= now
    {
        bail!("sandbox conformance report is stale or outside its runtime bound");
    }
    Ok(())
}

fn verify_signature(
    tx: &Transaction<'_>,
    input: &CreateExternalPoolAdapterSandboxConformance,
    message_base64: &str,
) -> Result<()> {
    let key = current_sandbox_verifier_key_authority_on(
        tx,
        &input.challenge.sandbox_verifier_key_record_id,
        &input.challenge.expected_sandbox_verifier_key_record_digest,
        &input.challenge.expected_sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active sandbox verifier key disappeared"))?;
    let message = STANDARD.decode(message_base64)?;
    let signature = STANDARD.decode(&input.signature_base64)?;
    let public = RsaPublicKey::from_public_key_pem(key.public_key_pem())
        .context("decode registered sandbox verifier public key")?;
    let signature = RsaSignature::try_from(signature.as_slice())?;
    VerifyingKey::<Sha256>::new(public)
        .verify(&message, &signature)
        .context("sandbox conformance signature verification failed")
}

fn validate_challenge_input(
    input: &GetExternalPoolAdapterSandboxConformanceChallenge,
) -> Result<()> {
    for value in [
        &input.admission_id,
        &input.sandbox_verifier_key_record_id,
        &input.expected_sandbox_verifier_key_id,
    ] {
        if value.trim() != value || value.is_empty() || value.len() > 200 {
            bail!("sandbox conformance challenge identifier is invalid");
        }
    }
    for value in [
        &input.expected_vulnerability_report_receipt_digest,
        &input.expected_sandbox_verifier_key_record_digest,
    ] {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("sandbox conformance challenge digest is invalid");
        }
    }
    validate_sandbox_conformance_draft(&input.draft)
}

fn validate_create_input(input: &CreateExternalPoolAdapterSandboxConformance) -> Result<()> {
    validate_challenge_input(&input.challenge)?;
    if input.confirmation != SANDBOX_CONFORMANCE_CONFIRMATION
        || input.expected_signature_message_digest.len() != 64
        || input.verified_by_admin_user_id.trim().is_empty()
        || input.idempotency_scope.trim().is_empty()
        || input.idempotency_key.trim().is_empty()
    {
        bail!("sandbox conformance create request is invalid");
    }
    let signature = STANDARD.decode(&input.signature_base64)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != input.signature_base64
    {
        bail!("sandbox verifier signature Base64 is invalid");
    }
    Ok(())
}

fn ensure_replay(
    receipt: &ExternalPoolAdapterSandboxConformanceReceipt,
    input: &CreateExternalPoolAdapterSandboxConformance,
) -> Result<()> {
    let item = &receipt.conformance;
    let binding = &item.binding;
    if binding.admission_id != input.challenge.admission_id
        || binding.vulnerability_report_receipt_digest
            != input.challenge.expected_vulnerability_report_receipt_digest
        || binding.sandbox_verifier_key_record_id != input.challenge.sandbox_verifier_key_record_id
        || binding.sandbox_verifier_key_record_digest
            != input.challenge.expected_sandbox_verifier_key_record_digest
        || binding.sandbox_verifier_key_id != input.challenge.expected_sandbox_verifier_key_id
        || binding.verifier_report_id != input.challenge.draft.verifier_report_id
        || binding.sandbox_runtime_id != input.challenge.draft.sandbox_runtime_id
        || binding.runtime_image_digest != input.challenge.draft.runtime_image_digest
        || binding.isolation_profile_id != input.challenge.draft.isolation_profile_id
        || binding.run_started_at != input.challenge.draft.run_started_at
        || binding.run_completed_at != input.challenge.draft.run_completed_at
        || binding.report_generated_at != input.challenge.draft.report_generated_at
        || binding.report_expires_at != input.challenge.draft.report_expires_at
        || binding.external_network_attempt_count
            != input.challenge.draft.external_network_attempt_count
        || binding.write_outside_ephemeral_count
            != input.challenge.draft.write_outside_ephemeral_count
        || binding.child_process_attempt_count != input.challenge.draft.child_process_attempt_count
        || binding.peak_memory_bytes != input.challenge.draft.peak_memory_bytes
        || binding.cpu_time_ms != input.challenge.draft.cpu_time_ms
        || binding.observations != input.challenge.draft.observations
        || item.signature_message_digest != input.expected_signature_message_digest
        || item.signature_base64 != input.signature_base64
        || item.verified_by_admin_user_id != input.verified_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("sandbox conformance idempotency key conflicts with immutable history");
    }
    Ok(())
}

fn insert_receipt(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterSandboxConformanceReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.conformance;
    let binding = &item.binding;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_sandbox_conformance_reports (
          sandbox_conformance_receipt_id,sandbox_conformance_receipt_digest,receipt_json,
          conformance_material_digest,admission_id,admission_digest,adapter_id,release_version,
          vulnerability_report_receipt_id,vulnerability_report_receipt_digest,
          sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,sandbox_verifier_key_id,
          verifier_report_id,sandbox_runtime_id,runtime_image_digest,report_expires_at,
          capability_set_digest,test_plan_digest,observation_inventory_digest,capability_count,
          passed_capability_count,policy_violation_count,signature_message_digest,signature_base64,
          signature_digest,verified_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,
          verified_at,recorded_at,evidence_scope,conformance_effect,credential_effect,adapter_effect,route_effect
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
                  ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37)",
        params![
            receipt.sandbox_conformance_receipt_id,
            receipt.sandbox_conformance_receipt_digest,
            json,
            receipt.conformance_material_digest,
            binding.admission_id,
            binding.admission_digest,
            binding.adapter_id,
            binding.release_version,
            binding.vulnerability_report_receipt_id,
            binding.vulnerability_report_receipt_digest,
            binding.sandbox_verifier_key_record_id,
            binding.sandbox_verifier_key_record_digest,
            binding.sandbox_verifier_key_id,
            binding.verifier_report_id,
            binding.sandbox_runtime_id,
            binding.runtime_image_digest,
            binding.report_expires_at,
            binding.capability_set_digest,
            binding.test_plan_digest,
            binding.observation_inventory_digest,
            binding.supported_capabilities.len() as i64,
            binding.passed_capability_count as i64,
            binding.policy_violation_count as i64,
            item.signature_message_digest,
            item.signature_base64,
            item.signature_digest,
            item.verified_by_admin_user_id,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.verified_at,
            item.recorded_at,
            item.evidence_scope,
            item.conformance_effect,
            item.credential_effect,
            item.adapter_effect,
            item.route_effect,
        ],
    )?;
    Ok(())
}
