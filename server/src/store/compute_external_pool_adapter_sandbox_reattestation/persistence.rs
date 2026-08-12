use anyhow::Result;
use rusqlite::{params, Transaction};

use crate::compute_federation::external_pool_adapter_sandbox_reattestation::{
    canonical_sandbox_reattestation_json, ExternalPoolAdapterSandboxReattestationChallenge,
    ExternalPoolAdapterSandboxReattestationReceipt,
};

pub(super) fn insert_challenge(
    tx: &Transaction<'_>,
    challenge: &ExternalPoolAdapterSandboxReattestationChallenge,
    json: &str,
) -> Result<()> {
    let b = &challenge.binding;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_sandbox_reattestation_challenges(
          challenge_id,challenge_nonce_base64,challenge_nonce_digest,signature_message_base64,
          signature_message_digest,registry_release_id,registry_release_digest,
          registry_release_material_digest,vulnerability_reattestation_receipt_id,
          vulnerability_reattestation_receipt_digest,vulnerability_reattestation_material_digest,
          sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,sandbox_verifier_key_id,
          sequence,predecessor_receipt_id,predecessor_receipt_digest,challenge_json,issued_at,expires_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            b.challenge_id,
            b.challenge_nonce_base64,
            b.challenge_nonce_digest,
            challenge.signature_message_base64,
            challenge.signature_message_digest,
            b.registry_release_id,
            b.registry_release_digest,
            b.registry_release_material_digest,
            b.vulnerability_reattestation_receipt_id,
            b.vulnerability_reattestation_receipt_digest,
            b.vulnerability_reattestation_material_digest,
            b.sandbox_verifier_key_record_id,
            b.sandbox_verifier_key_record_digest,
            b.sandbox_verifier_key_id,
            i64::try_from(b.sequence)?,
            b.predecessor_receipt_id,
            b.predecessor_receipt_digest,
            json,
            b.challenge_issued_at,
            b.challenge_expires_at,
        ],
    )?;
    Ok(())
}

pub(super) fn insert_receipt(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterSandboxReattestationReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.reattestation;
    let b = &item.binding;
    let supported_provider_kinds_json =
        canonical_sandbox_reattestation_json(&b.supported_provider_kinds)?;
    let supported_capabilities_json =
        canonical_sandbox_reattestation_json(&b.supported_capabilities)?;
    let credential_verifier_json =
        canonical_sandbox_reattestation_json(&b.expected_credential_verifier)?;
    let test_plan_json = canonical_sandbox_reattestation_json(&b.test_plan)?;
    let observations_json = canonical_sandbox_reattestation_json(&b.observations)?;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_sandbox_reattestation_receipts(
          reattestation_receipt_id,reattestation_receipt_digest,receipt_json,reattestation_material_digest,
          challenge_id,challenge_nonce_digest,registry_release_id,registry_release_digest,
          registry_release_material_digest,admission_id,admission_digest,package_receipt_id,
          package_receipt_digest,source_receipt_id,source_receipt_digest,adapter_id,release_version,
          route_kind,supported_provider_kinds_json,implementation_digest,
          declared_implementation_sha256,supported_capabilities_json,capability_set_digest,
          credential_verifier_json,
          credential_verifier_digest,archive_sha256,archive_size_bytes,manifest_digest,
          entry_inventory_digest,entry_count,total_uncompressed_bytes,installation_content_digest,
          vulnerability_reattestation_receipt_id,
          vulnerability_reattestation_receipt_digest,vulnerability_reattestation_material_digest,
          vulnerability_reattestation_sequence,vulnerability_reattestation_verified_at,
          vulnerability_intelligence_snapshot_digest,vulnerability_intelligence_expires_at,
          security_receipt_id,security_receipt_digest,security_material_digest,sbom_digest,
          component_inventory_digest,component_count,dependency_inventory_digest,
          sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,sandbox_verifier_key_id,
          sandbox_verifier_operator,sandbox_verifier_product,
          sequence,predecessor_receipt_id,predecessor_receipt_digest,verifier_report_id,
          sandbox_runtime_id,runtime_image_digest,isolation_profile_id,run_started_at,run_completed_at,report_generated_at,
          report_expires_at,external_network_attempt_count,write_outside_ephemeral_count,
          child_process_attempt_count,peak_memory_bytes,cpu_time_ms,test_plan_digest,test_plan_json,
          observation_inventory_digest,observations_json,capability_count,passed_capability_count,
          policy_violation_count,
          signature_message_digest,signature_base64,signature_digest,recorded_by_admin_user_id,
          confirmation,idempotency_scope,idempotency_key,verified_at,recorded_at,evidence_scope,
          sandbox_reattestation_effect,adapter_effect,provider_effect,credential_effect,route_effect,
          execution_effect,settlement_effect
        ) VALUES (
          ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,
          ?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,
          ?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,?51,?52,?53,?54,?55,?56,
          ?57,?58,?59,?60,?61,?62,?63,?64,?65,?66,?67,?68,?69,?70,?71,?72,?73,?74,
          ?75,?76,?77,?78,?79,?80,?81,?82,?83,?84,?85,?86,?87,?88,?89,?90,?91)",
        params![
            receipt.reattestation_receipt_id, receipt.reattestation_receipt_digest, json,
            receipt.reattestation_material_digest, b.challenge_id, b.challenge_nonce_digest,
            b.registry_release_id, b.registry_release_digest, b.registry_release_material_digest,
            b.admission_id, b.admission_digest, b.package_receipt_id, b.package_receipt_digest,
            b.source_receipt_id, b.source_receipt_digest, b.adapter_id, b.release_version,
            b.route_kind, supported_provider_kinds_json, b.implementation_digest,
            b.declared_implementation_sha256, supported_capabilities_json,
            b.capability_set_digest, credential_verifier_json,
            b.credential_verifier_digest, b.archive_sha256, i64::try_from(b.archive_size_bytes)?,
            b.manifest_digest, b.entry_inventory_digest, i64::try_from(b.entry_count)?,
            i64::try_from(b.total_uncompressed_bytes)?, b.installation_content_digest,
            b.vulnerability_reattestation_receipt_id, b.vulnerability_reattestation_receipt_digest,
            b.vulnerability_reattestation_material_digest,
            i64::try_from(b.vulnerability_reattestation_sequence)?,
            b.vulnerability_reattestation_verified_at, b.vulnerability_intelligence_snapshot_digest,
            b.vulnerability_intelligence_expires_at, b.security_receipt_id, b.security_receipt_digest,
            b.security_material_digest, b.sbom_digest, b.component_inventory_digest,
            i64::try_from(b.component_count)?, b.dependency_inventory_digest,
            b.sandbox_verifier_key_record_id, b.sandbox_verifier_key_record_digest,
            b.sandbox_verifier_key_id, b.sandbox_verifier_operator, b.sandbox_verifier_product,
            i64::try_from(b.sequence)?, b.predecessor_receipt_id,
            b.predecessor_receipt_digest, b.verifier_report_id, b.sandbox_runtime_id,
            b.runtime_image_digest, b.isolation_profile_id, b.run_started_at, b.run_completed_at,
            b.report_generated_at,
            b.report_expires_at, i64::try_from(b.external_network_attempt_count)?,
            i64::try_from(b.write_outside_ephemeral_count)?,
            i64::try_from(b.child_process_attempt_count)?, i64::try_from(b.peak_memory_bytes)?,
            i64::try_from(b.cpu_time_ms)?, b.test_plan_digest, test_plan_json,
            b.observation_inventory_digest, observations_json,
            i64::try_from(b.supported_capabilities.len())?, i64::try_from(b.passed_capability_count)?,
            i64::try_from(b.policy_violation_count)?, item.signature_message_digest,
            item.signature_base64, item.signature_digest, item.recorded_by_admin_user_id,
            item.confirmation, item.idempotency_scope, item.idempotency_key, item.verified_at,
            item.recorded_at, item.evidence_scope, item.sandbox_reattestation_effect,
            item.adapter_effect, item.provider_effect, item.credential_effect, item.route_effect,
            item.execution_effect, item.settlement_effect,
        ],
    )?;
    Ok(())
}
