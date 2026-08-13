use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_credential_reattestation::canonical_credential_reattestation_json;

use super::types::{StoredCredentialReattestation, StoredCredentialReattestationRevocation};

pub(super) fn exact_receipt_projection(
    conn: &Connection,
    stored: &StoredCredentialReattestation,
) -> Result<bool> {
    let r = &stored.receipt;
    let item = &r.reattestation;
    let b = &item.binding;
    let verifier_json = canonical_credential_reattestation_json(&b.expected_credential_verifier)?;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts
          WHERE reattestation_receipt_id=?1 AND reattestation_receipt_digest=?2
            AND receipt_json=?3 AND reattestation_material_digest=?4 AND challenge_id=?5
            AND challenge_nonce_digest=?6 AND provider_binding_id=?7
            AND provider_binding_digest=?8 AND provider_binding_material_digest=?9
            AND registry_release_id=?10 AND registry_release_digest=?11
            AND registry_release_material_digest=?12 AND route_adapter_projection_id=?13
            AND installation_receipt_id=?14 AND installation_receipt_digest=?15
            AND installation_content_digest=?16 AND application_id=?17
            AND application_digest=?18 AND adoption_receipt_id=?19
            AND adoption_receipt_digest=?20 AND provider_id=?21 AND provider_kind=?22
            AND provider_owner_account_id=?23 AND observed_settlement_account_id=?24
            AND observed_provider_policy_revision=?25 AND observed_provider_digest=?26
            AND observed_provider_status=?27 AND adapter_id=?28 AND release_version=?29
            AND adapter_config_revision=?30 AND adapter_config_digest=?31
            AND admission_id=?32 AND admission_digest=?33
            AND legacy_credential_verification_receipt_id=?34
            AND legacy_credential_verification_receipt_digest=?35
            AND credential_ref_scheme=?36 AND credential_locator_commitment=?37
            AND expected_credential_verifier_json=?38 AND credential_verifier_digest=?39
            AND credential_verifier_key_record_id=?40
            AND credential_verifier_key_record_digest=?41
            AND credential_verifier_key_id=?42 AND credential_verifier_record_id=?43
            AND credential_verifier_record_digest=?44 AND sequence=?45
            AND predecessor_receipt_id IS ?46 AND predecessor_receipt_digest IS ?47
            AND verifier_report_id=?48 AND verification_started_at=?49
            AND verification_completed_at=?50 AND report_generated_at=?51
            AND report_expires_at=?52 AND credential_resolution_outcome=?53
            AND provider_authentication_outcome=?54 AND provider_response_evidence_digest=?55
            AND signature_message_digest=?56 AND signature_base64=?57
            AND signature_digest=?58 AND recorded_by_admin_user_id=?59
            AND confirmation=?60 AND idempotency_scope=?61 AND idempotency_key=?62
            AND verified_at=?63 AND recorded_at=?64 AND evidence_scope=?65
            AND credential_reattestation_effect=?66 AND adapter_effect=?67
            AND provider_effect=?68 AND route_effect=?69 AND execution_effect=?70
            AND usage_effect=?71 AND settlement_effect=?72",
            params![
                r.reattestation_receipt_id,
                r.reattestation_receipt_digest,
                stored.receipt_json,
                r.reattestation_material_digest,
                b.challenge_id,
                b.challenge_nonce_digest,
                b.provider_binding_id,
                b.provider_binding_digest,
                b.provider_binding_material_digest,
                b.registry_release_id,
                b.registry_release_digest,
                b.registry_release_material_digest,
                b.route_adapter_projection_id,
                b.installation_receipt_id,
                b.installation_receipt_digest,
                b.installation_content_digest,
                b.application_id,
                b.application_digest,
                b.adoption_receipt_id,
                b.adoption_receipt_digest,
                b.provider_id,
                b.provider_kind,
                b.provider_owner_account_id,
                b.observed_settlement_account_id,
                b.observed_provider_policy_revision,
                b.observed_provider_digest,
                b.observed_provider_status,
                b.adapter_id,
                b.release_version,
                b.adapter_config_revision,
                b.adapter_config_digest,
                b.admission_id,
                b.admission_digest,
                b.legacy_credential_verification_receipt_id,
                b.legacy_credential_verification_receipt_digest,
                b.credential_ref_scheme,
                b.credential_locator_commitment,
                verifier_json,
                b.credential_verifier_digest,
                b.credential_verifier_key_record_id,
                b.credential_verifier_key_record_digest,
                b.credential_verifier_key_id,
                b.credential_verifier_record_id,
                b.credential_verifier_record_digest,
                i64::try_from(b.sequence)?,
                b.predecessor_receipt_id,
                b.predecessor_receipt_digest,
                b.verifier_report_id,
                b.verification_started_at,
                b.verification_completed_at,
                b.report_generated_at,
                b.report_expires_at,
                b.credential_resolution_outcome,
                b.provider_authentication_outcome,
                b.provider_response_evidence_digest,
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
                item.credential_reattestation_effect,
                item.adapter_effect,
                item.provider_effect,
                item.route_effect,
                item.execution_effect,
                item.usage_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn exact_revocation_projection(
    conn: &Connection,
    stored: &StoredCredentialReattestationRevocation,
) -> Result<bool> {
    let r = &stored.receipt;
    let item = &r.revocation;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_revocations
          WHERE revocation_receipt_id=?1 AND revocation_receipt_digest=?2 AND receipt_json=?3
            AND revocation_material_digest=?4 AND reattestation_receipt_id=?5
            AND reattestation_receipt_digest=?6 AND provider_binding_id=?7
            AND provider_binding_digest=?8 AND revoked_by_admin_user_id=?9 AND reason=?10
            AND confirmation=?11 AND idempotency_scope=?12 AND idempotency_key=?13
            AND revoked_at=?14 AND recorded_at=?15 AND revocation_effect=?16
            AND adapter_effect=?17 AND provider_effect=?18 AND route_effect=?19
            AND execution_effect=?20 AND usage_effect=?21 AND settlement_effect=?22",
            params![
                r.revocation_receipt_id,
                r.revocation_receipt_digest,
                stored.receipt_json,
                r.revocation_material_digest,
                item.reattestation_receipt_id,
                item.reattestation_receipt_digest,
                item.provider_binding_id,
                item.provider_binding_digest,
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
                item.route_effect,
                item.execution_effect,
                item.usage_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
