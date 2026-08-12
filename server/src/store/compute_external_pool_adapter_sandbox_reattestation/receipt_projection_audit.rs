use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::types::{StoredSandboxReattestation, StoredSandboxReattestationRevocation};

pub(super) fn exact_receipt_projection(
    conn: &Connection,
    stored: &StoredSandboxReattestation,
) -> Result<bool> {
    let r = &stored.receipt;
    let item = &r.reattestation;
    let b = &item.binding;
    let supported_provider_kinds_json =
        crate::compute_federation::external_pool_adapter_sandbox_reattestation::canonical_sandbox_reattestation_json(&b.supported_provider_kinds)?;
    let supported_capabilities_json =
        crate::compute_federation::external_pool_adapter_sandbox_reattestation::canonical_sandbox_reattestation_json(&b.supported_capabilities)?;
    let credential_verifier_json =
        crate::compute_federation::external_pool_adapter_sandbox_reattestation::canonical_sandbox_reattestation_json(&b.expected_credential_verifier)?;
    let test_plan_json =
        crate::compute_federation::external_pool_adapter_sandbox_reattestation::canonical_sandbox_reattestation_json(&b.test_plan)?;
    let observations_json =
        crate::compute_federation::external_pool_adapter_sandbox_reattestation::canonical_sandbox_reattestation_json(&b.observations)?;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts
          WHERE reattestation_receipt_id=?1 AND reattestation_receipt_digest=?2
            AND receipt_json=?3 AND reattestation_material_digest=?4 AND challenge_id=?5
            AND challenge_nonce_digest=?6 AND registry_release_id=?7
            AND registry_release_digest=?8 AND registry_release_material_digest=?9
            AND admission_id=?10 AND admission_digest=?11 AND package_receipt_id=?12
            AND package_receipt_digest=?13 AND source_receipt_id=?14 AND source_receipt_digest=?15
            AND adapter_id=?16 AND release_version=?17 AND route_kind=?18
            AND supported_provider_kinds_json=?19 AND implementation_digest=?20
            AND declared_implementation_sha256=?21 AND supported_capabilities_json=?22
            AND capability_set_digest=?23 AND credential_verifier_json=?24
            AND credential_verifier_digest=?25 AND archive_sha256=?26 AND archive_size_bytes=?27
            AND manifest_digest=?28 AND entry_inventory_digest=?29 AND entry_count=?30
            AND total_uncompressed_bytes=?31 AND installation_content_digest=?32
            AND vulnerability_reattestation_receipt_id=?33
            AND vulnerability_reattestation_receipt_digest=?34
            AND vulnerability_reattestation_material_digest=?35
            AND vulnerability_reattestation_sequence=?36
            AND vulnerability_reattestation_verified_at=?37
            AND vulnerability_intelligence_snapshot_digest=?38
            AND vulnerability_intelligence_expires_at=?39 AND security_receipt_id=?40
            AND security_receipt_digest=?41 AND security_material_digest=?42
            AND sbom_digest=?43 AND component_inventory_digest=?44 AND component_count=?45
            AND dependency_inventory_digest=?46 AND sandbox_verifier_key_record_id=?47
            AND sandbox_verifier_key_record_digest=?48 AND sandbox_verifier_key_id=?49
            AND sandbox_verifier_operator=?50 AND sandbox_verifier_product=?51
            AND sequence=?52 AND predecessor_receipt_id IS ?53
            AND predecessor_receipt_digest IS ?54 AND verifier_report_id=?55
            AND sandbox_runtime_id=?56 AND runtime_image_digest=?57 AND isolation_profile_id=?58
            AND run_started_at=?59 AND run_completed_at=?60 AND report_generated_at=?61
            AND report_expires_at=?62 AND external_network_attempt_count=?63
            AND write_outside_ephemeral_count=?64 AND child_process_attempt_count=?65
            AND peak_memory_bytes=?66 AND cpu_time_ms=?67 AND test_plan_digest=?68
            AND test_plan_json=?69 AND observation_inventory_digest=?70
            AND observations_json=?71 AND capability_count=?72
            AND passed_capability_count=?73 AND policy_violation_count=?74
            AND signature_message_digest=?75 AND signature_base64=?76
            AND signature_digest=?77 AND recorded_by_admin_user_id=?78
            AND confirmation=?79 AND idempotency_scope=?80 AND idempotency_key=?81
            AND verified_at=?82 AND recorded_at=?83 AND evidence_scope=?84
            AND sandbox_reattestation_effect=?85 AND adapter_effect=?86
            AND provider_effect=?87 AND credential_effect=?88 AND route_effect=?89
            AND execution_effect=?90 AND settlement_effect=?91",
            params![
                r.reattestation_receipt_id,
                r.reattestation_receipt_digest,
                stored.receipt_json,
                r.reattestation_material_digest,
                b.challenge_id,
                b.challenge_nonce_digest,
                b.registry_release_id,
                b.registry_release_digest,
                b.registry_release_material_digest,
                b.admission_id,
                b.admission_digest,
                b.package_receipt_id,
                b.package_receipt_digest,
                b.source_receipt_id,
                b.source_receipt_digest,
                b.adapter_id,
                b.release_version,
                b.route_kind,
                supported_provider_kinds_json,
                b.implementation_digest,
                b.declared_implementation_sha256,
                supported_capabilities_json,
                b.capability_set_digest,
                credential_verifier_json,
                b.credential_verifier_digest,
                b.archive_sha256,
                i64::try_from(b.archive_size_bytes)?,
                b.manifest_digest,
                b.entry_inventory_digest,
                i64::try_from(b.entry_count)?,
                i64::try_from(b.total_uncompressed_bytes)?,
                b.installation_content_digest,
                b.vulnerability_reattestation_receipt_id,
                b.vulnerability_reattestation_receipt_digest,
                b.vulnerability_reattestation_material_digest,
                i64::try_from(b.vulnerability_reattestation_sequence)?,
                b.vulnerability_reattestation_verified_at,
                b.vulnerability_intelligence_snapshot_digest,
                b.vulnerability_intelligence_expires_at,
                b.security_receipt_id,
                b.security_receipt_digest,
                b.security_material_digest,
                b.sbom_digest,
                b.component_inventory_digest,
                i64::try_from(b.component_count)?,
                b.dependency_inventory_digest,
                b.sandbox_verifier_key_record_id,
                b.sandbox_verifier_key_record_digest,
                b.sandbox_verifier_key_id,
                b.sandbox_verifier_operator,
                b.sandbox_verifier_product,
                i64::try_from(b.sequence)?,
                b.predecessor_receipt_id,
                b.predecessor_receipt_digest,
                b.verifier_report_id,
                b.sandbox_runtime_id,
                b.runtime_image_digest,
                b.isolation_profile_id,
                b.run_started_at,
                b.run_completed_at,
                b.report_generated_at,
                b.report_expires_at,
                i64::try_from(b.external_network_attempt_count)?,
                i64::try_from(b.write_outside_ephemeral_count)?,
                i64::try_from(b.child_process_attempt_count)?,
                i64::try_from(b.peak_memory_bytes)?,
                i64::try_from(b.cpu_time_ms)?,
                b.test_plan_digest,
                test_plan_json,
                b.observation_inventory_digest,
                observations_json,
                i64::try_from(b.supported_capabilities.len())?,
                i64::try_from(b.passed_capability_count)?,
                i64::try_from(b.policy_violation_count)?,
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
                item.sandbox_reattestation_effect,
                item.adapter_effect,
                item.provider_effect,
                item.credential_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn exact_revocation_projection(
    conn: &Connection,
    stored: &StoredSandboxReattestationRevocation,
) -> Result<bool> {
    let r = &stored.receipt;
    let item = &r.revocation;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_revocations
          WHERE revocation_receipt_id=?1 AND revocation_receipt_digest=?2 AND receipt_json=?3
            AND revocation_material_digest=?4 AND reattestation_receipt_id=?5
            AND reattestation_receipt_digest=?6 AND registry_release_id=?7
            AND registry_release_digest=?8 AND revoked_by_admin_user_id=?9 AND reason=?10
            AND confirmation=?11 AND idempotency_scope=?12 AND idempotency_key=?13
            AND revoked_at=?14 AND recorded_at=?15 AND revocation_effect=?16
            AND adapter_effect=?17 AND provider_effect=?18 AND credential_effect=?19
            AND route_effect=?20 AND execution_effect=?21 AND settlement_effect=?22",
            params![
                r.revocation_receipt_id,
                r.revocation_receipt_digest,
                stored.receipt_json,
                r.revocation_material_digest,
                item.reattestation_receipt_id,
                item.reattestation_receipt_digest,
                item.registry_release_id,
                item.registry_release_digest,
                item.revoked_by_admin_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.revoked_at,
                item.recorded_at,
                item.revocation_effect,
                item.adapter_effect,
                item.provider_effect,
                item.credential_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
