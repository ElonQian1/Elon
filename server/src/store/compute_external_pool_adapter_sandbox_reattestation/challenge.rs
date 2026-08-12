use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::rand_core::{OsRng, RngCore};
use rusqlite::{Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_sandbox_conformance::{
            sandbox_capability_test_plan, sandbox_observation_inventory_digest,
            sandbox_test_plan_digest, validate_sandbox_conformance_draft,
        },
        external_pool_adapter_sandbox_reattestation::*,
    },
    store::{
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_release_authority_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        compute_external_pool_adapter_vulnerability_reattestation::current_external_pool_adapter_vulnerability_reattestation_authority_on,
        new_id, Store,
    },
};

use super::{persistence::insert_challenge, read::head_by_release_on, types::*};

impl Store {
    pub(crate) fn issue_external_pool_adapter_sandbox_reattestation_challenge(
        &self,
        input: GetExternalPoolAdapterSandboxReattestationChallenge,
    ) -> Result<ExternalPoolAdapterSandboxReattestationChallenge> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let issued = Utc::now();
        validate_runtime_window(&input.draft, issued)?;
        let challenge = build(&tx, input, issued)?;
        let json = canonical_sandbox_reattestation_json(&challenge)?;
        insert_challenge(&tx, &challenge, &json)?;
        tx.commit()?;
        Ok(challenge)
    }
}

fn build(
    tx: &Transaction<'_>,
    input: GetExternalPoolAdapterSandboxReattestationChallenge,
    issued: chrono::DateTime<Utc>,
) -> Result<ExternalPoolAdapterSandboxReattestationChallenge> {
    let checked_at = issued.to_rfc3339_opts(SecondsFormat::Nanos, true);
    let release = current_external_pool_adapter_registry_release_authority_on(
        tx,
        &input.registry_release_id,
        &input.expected_registry_release_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V249 registry release was not found"))?;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        tx,
        &input.registry_release_id,
        &input.vulnerability_reattestation_receipt_id,
        &input.expected_vulnerability_reattestation_receipt_digest,
        &checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current V250 vulnerability re-attestation was not found"))?;
    if release.checked_at() != checked_at || vulnerability.checked_at() != checked_at {
        bail!("sandbox re-attestation roots used different checked_at anchors");
    }
    let verifier = current_sandbox_verifier_key_authority_on(
        tx,
        &input.sandbox_verifier_key_record_id,
        &input.expected_sandbox_verifier_key_record_digest,
        &input.expected_sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active V237 sandbox verifier was not found"))?;
    let release_receipt = release.release();
    let release_item = &release_receipt.release;
    let vulnerability_receipt = vulnerability.receipt();
    let vulnerability_binding = &vulnerability_receipt.reattestation.binding;
    if vulnerability_binding.registry_release_digest != release_receipt.registry_release_digest {
        bail!("V250 vulnerability authority does not bind the exact V249 release");
    }
    let predecessor = head_by_release_on(tx, &input.registry_release_id)?;
    let sequence = predecessor
        .as_ref()
        .map(|stored| {
            stored
                .receipt
                .reattestation
                .binding
                .sequence
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation sequence overflow"))
        })
        .transpose()?
        .unwrap_or(1);
    let test_plan = sandbox_capability_test_plan(
        &release_item.admission_digest,
        &release_item.supported_capabilities,
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
    let mut nonce = [0_u8; 32];
    OsRng.fill_bytes(&mut nonce);
    let binding = ExternalPoolAdapterSandboxReattestationBinding {
        schema: SANDBOX_REATTESTATION_BINDING_SCHEMA.into(),
        challenge_id: new_id("external_pool_adapter_sandbox_reattestation_challenge"),
        challenge_nonce_base64: STANDARD.encode(nonce),
        challenge_nonce_digest: hex::encode(Sha256::digest(nonce)),
        challenge_issued_at: checked_at,
        challenge_expires_at: (issued
            + Duration::minutes(SANDBOX_REATTESTATION_CHALLENGE_VALIDITY_MINUTES))
        .to_rfc3339_opts(SecondsFormat::Nanos, true),
        registry_release_id: release_receipt.registry_release_id.clone(),
        registry_release_digest: release_receipt.registry_release_digest.clone(),
        registry_release_material_digest: release_receipt.registry_release_material_digest.clone(),
        admission_id: release_item.admission_id.clone(),
        admission_digest: release_item.admission_digest.clone(),
        package_receipt_id: release_item.package_receipt_id.clone(),
        package_receipt_digest: release_item.package_receipt_digest.clone(),
        source_receipt_id: release_item.source_receipt_id.clone(),
        source_receipt_digest: release_item.source_receipt_digest.clone(),
        adapter_id: release_item.adapter_id.clone(),
        release_version: release_item.release_version.clone(),
        route_kind: release_item.route_kind.clone(),
        supported_provider_kinds: release_item.supported_provider_kinds.clone(),
        implementation_digest: release_item.implementation_digest.clone(),
        declared_implementation_sha256: release_item.declared_implementation_sha256.clone(),
        supported_capabilities: release_item.supported_capabilities.clone(),
        capability_set_digest: release_item.capability_set_digest.clone(),
        expected_credential_verifier: release_item.credential_verifier.clone(),
        credential_verifier_digest: release_item.credential_verifier_digest.clone(),
        archive_sha256: release_item.archive_sha256.clone(),
        archive_size_bytes: release_item.archive_size_bytes,
        manifest_digest: release_item.manifest_digest.clone(),
        entry_inventory_digest: release_item.entry_inventory_digest.clone(),
        entry_count: release_item.entry_count,
        total_uncompressed_bytes: release_item.total_uncompressed_bytes,
        installation_content_digest: release_item.installation_content_digest.clone(),
        vulnerability_reattestation_receipt_id: vulnerability_receipt
            .reattestation_receipt_id
            .clone(),
        vulnerability_reattestation_receipt_digest: vulnerability_receipt
            .reattestation_receipt_digest
            .clone(),
        vulnerability_reattestation_material_digest: vulnerability_receipt
            .reattestation_material_digest
            .clone(),
        vulnerability_reattestation_sequence: vulnerability_binding.sequence,
        vulnerability_reattestation_verified_at: vulnerability_receipt
            .reattestation
            .verified_at
            .clone(),
        vulnerability_intelligence_snapshot_digest: vulnerability_binding
            .intelligence
            .snapshot_digest
            .clone(),
        vulnerability_intelligence_expires_at: vulnerability_binding
            .intelligence
            .expires_at
            .clone(),
        security_receipt_id: vulnerability_binding.security_receipt_id.clone(),
        security_receipt_digest: vulnerability_binding.security_receipt_digest.clone(),
        security_material_digest: vulnerability_binding.security_material_digest.clone(),
        sbom_digest: vulnerability_binding.sbom_digest.clone(),
        component_inventory_digest: vulnerability_binding.component_inventory_digest.clone(),
        component_count: vulnerability_binding.component_count,
        dependency_inventory_digest: vulnerability_binding.dependency_inventory_digest.clone(),
        sandbox_verifier_key_record_id: verifier.key_record_id().into(),
        sandbox_verifier_key_record_digest: verifier.key_record_digest().into(),
        sandbox_verifier_key_id: verifier.key_id().into(),
        sandbox_verifier_operator: verifier.verifier_operator().into(),
        sandbox_verifier_product: verifier.verifier_product().into(),
        signature_algorithm: SANDBOX_REATTESTATION_SIGNATURE_ALGORITHM.into(),
        sandbox_policy_id: SANDBOX_REATTESTATION_POLICY_ID.into(),
        sequence,
        predecessor_receipt_id: predecessor
            .as_ref()
            .map(|item| item.receipt.reattestation_receipt_id.clone()),
        predecessor_receipt_digest: predecessor
            .as_ref()
            .map(|item| item.receipt.reattestation_receipt_digest.clone()),
        verifier_report_id: input.draft.verifier_report_id,
        sandbox_runtime_id: input.draft.sandbox_runtime_id,
        runtime_image_digest: input.draft.runtime_image_digest,
        isolation_profile_id: input.draft.isolation_profile_id,
        run_started_at: input.draft.run_started_at,
        run_completed_at: input.draft.run_completed_at,
        report_generated_at: input.draft.report_generated_at,
        report_expires_at: input.draft.report_expires_at,
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
        passed_capability_count: input
            .draft
            .observations
            .iter()
            .filter(|item| item.outcome == "passed")
            .count() as u64,
        policy_violation_count,
        observations: input.draft.observations,
    };
    validate_sandbox_reattestation_binding(&binding)?;
    sandbox_reattestation_challenge(binding)
}

fn validate_input(input: &GetExternalPoolAdapterSandboxReattestationChallenge) -> Result<()> {
    validate_sandbox_conformance_draft(&input.draft)?;
    for value in [
        &input.registry_release_id,
        &input.vulnerability_reattestation_receipt_id,
        &input.sandbox_verifier_key_record_id,
        &input.expected_sandbox_verifier_key_id,
    ] {
        if value.trim() != value || value.is_empty() || value.chars().count() > 240 {
            bail!("sandbox re-attestation challenge identifier is invalid");
        }
    }
    for value in [
        &input.expected_registry_release_digest,
        &input.expected_vulnerability_reattestation_receipt_digest,
        &input.expected_sandbox_verifier_key_record_digest,
    ] {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            bail!("sandbox re-attestation challenge digest is invalid");
        }
    }
    Ok(())
}

fn validate_runtime_window(
    draft: &crate::compute_federation::external_pool_adapter_artifact_sandbox_conformance::ExternalPoolAdapterSandboxConformanceDraft,
    now: chrono::DateTime<Utc>,
) -> Result<()> {
    for value in [
        &draft.run_started_at,
        &draft.run_completed_at,
        &draft.report_generated_at,
    ] {
        if chrono::DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc)
            > now + Duration::minutes(5)
        {
            bail!("sandbox re-attestation report is future-dated");
        }
    }
    if chrono::DateTime::parse_from_rfc3339(&draft.report_expires_at)?.with_timezone(&Utc) <= now {
        bail!("sandbox re-attestation report is stale");
    }
    Ok(())
}
