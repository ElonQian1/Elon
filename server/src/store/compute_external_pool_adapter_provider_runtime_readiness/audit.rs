use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::*;

pub(super) fn audit_readiness_projection(
    conn: &Connection,
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessReceipt,
    receipt_json: &str,
) -> Result<()> {
    let r = &receipt.readiness;
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_provider_runtime_readiness_receipts
          WHERE readiness_receipt_id=:id AND readiness_receipt_schema=:schema
            AND readiness_receipt_digest=:digest AND readiness_material_digest=:material
            AND readiness_receipt_json=:json AND canonicalization=:canonicalization
            AND digest_algorithm=:algorithm AND policy_id=:policy_id
            AND policy_revision=:policy_revision AND policy_digest=:policy_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND registry_release_id=:release_id AND registry_release_digest=:release_digest
            AND registry_release_material_digest=:release_material
            AND installation_receipt_id=:installation_id
            AND installation_receipt_digest=:installation_digest
            AND installation_content_digest=:content_digest
            AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest
            AND delegation_id=:delegation_id AND delegation_digest=:delegation_digest
            AND profile_id=:profile_id AND profile_digest=:profile_digest
            AND target_id=:target_id AND target_digest=:target_digest
            AND companion_id=:companion_id AND companion_digest=:companion_digest
            AND provider_id=:provider_id AND provider_policy_revision=:provider_revision
            AND provider_digest=:provider_digest AND provider_status=:provider_status
            AND vulnerability_reattestation_receipt_id=:vulnerability_id
            AND vulnerability_reattestation_receipt_digest=:vulnerability_digest
            AND sandbox_reattestation_receipt_id=:sandbox_id
            AND sandbox_reattestation_receipt_digest=:sandbox_digest
            AND credential_reattestation_receipt_id=:credential_id
            AND credential_reattestation_receipt_digest=:credential_digest
            AND runtime_compatibility_verification_receipt_id=:verification_id
            AND runtime_compatibility_verification_receipt_digest=:verification_digest
            AND launch_policy_digest=:launch_policy AND target_policy_digest=:target_policy
            AND entrypoint_capsule_policy_digest=:capsule_policy
            AND supervisor_session_policy_digest=:session_policy
            AND source_capsule_sha256=:source_sha AND source_capsule_size_bytes=:source_size
            AND launch_image_sha256=:launch_sha AND launch_image_size_bytes=:launch_size
            AND runtime_custody_epoch_digest=:epoch_digest
            AND runtime_bundle_identity_commitment=:bundle_commitment
            AND post_cleanup_observation_commitment=:observation_commitment
            AND probe_execution_id=:execution_id AND request_bytes=:request_bytes
            AND response_bytes=:response_bytes AND probe_checked_at=:probe_at
            AND cleanup_completed_at=:cleanup_at AND checked_at=:checked_at
            AND expires_at=:expires_at AND sequence=:sequence
            AND predecessor_readiness_receipt_id IS :predecessor_id
            AND predecessor_readiness_receipt_digest IS :predecessor_digest
            AND recorded_by_actor_kind=:actor_kind AND recorded_by_actor_user_id=:actor_id
            AND recorded_at=:recorded_at AND idempotency_scope=:scope
            AND idempotency_key=:key AND confirmation=:confirmation
            AND evidence_scope=:evidence_scope AND receipt_status=:status
            AND effects_json=:effects AND observed_readiness_json=:readiness)",
        named_params! {
            ":id": receipt.readiness_receipt_id,
            ":schema": receipt.schema,
            ":digest": receipt.readiness_receipt_digest,
            ":material": receipt.readiness_material_digest,
            ":json": receipt_json,
            ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm,
            ":policy_id": r.policy_id,
            ":policy_revision": i64::try_from(r.policy_revision)?,
            ":policy_digest": r.policy_digest,
            ":binding_id": r.provider_binding_id,
            ":binding_digest": r.provider_binding_digest,
            ":release_id": r.registry_release_id,
            ":release_digest": r.registry_release_digest,
            ":release_material": r.registry_release_material_digest,
            ":installation_id": r.installation_receipt_id,
            ":installation_digest": r.installation_receipt_digest,
            ":content_digest": r.installation_content_digest,
            ":candidate_id": r.candidate_id,
            ":candidate_digest": r.candidate_digest,
            ":delegation_id": r.delegation_id,
            ":delegation_digest": r.delegation_digest,
            ":profile_id": r.profile_id,
            ":profile_digest": r.profile_digest,
            ":target_id": r.target_id,
            ":target_digest": r.target_digest,
            ":companion_id": r.companion_id,
            ":companion_digest": r.companion_digest,
            ":provider_id": r.provider_id,
            ":provider_revision": r.provider_policy_revision,
            ":provider_digest": r.provider_digest,
            ":provider_status": r.provider_status,
            ":vulnerability_id": r.vulnerability_reattestation_receipt_id,
            ":vulnerability_digest": r.vulnerability_reattestation_receipt_digest,
            ":sandbox_id": r.sandbox_reattestation_receipt_id,
            ":sandbox_digest": r.sandbox_reattestation_receipt_digest,
            ":credential_id": r.credential_reattestation_receipt_id,
            ":credential_digest": r.credential_reattestation_receipt_digest,
            ":verification_id": r.runtime_compatibility_verification_receipt_id,
            ":verification_digest": r.runtime_compatibility_verification_receipt_digest,
            ":launch_policy": r.launch_policy_digest,
            ":target_policy": r.target_policy_digest,
            ":capsule_policy": r.entrypoint_capsule_policy_digest,
            ":session_policy": r.supervisor_session_policy_digest,
            ":source_sha": r.source_capsule_sha256,
            ":source_size": i64::try_from(r.source_capsule_size_bytes)?,
            ":launch_sha": r.launch_image_sha256,
            ":launch_size": i64::try_from(r.launch_image_size_bytes)?,
            ":epoch_digest": r.sealed_bindings.runtime_custody_epoch_digest,
            ":bundle_commitment": r.sealed_bindings.runtime_bundle_identity_commitment,
            ":observation_commitment": r.sealed_bindings.post_cleanup_observation_commitment,
            ":execution_id": r.probe_execution_id,
            ":request_bytes": i64::try_from(r.request_bytes)?,
            ":response_bytes": i64::try_from(r.response_bytes)?,
            ":probe_at": r.probe_checked_at,
            ":cleanup_at": r.cleanup_completed_at,
            ":checked_at": r.checked_at,
            ":expires_at": r.expires_at,
            ":sequence": i64::try_from(r.sequence)?,
            ":predecessor_id": r.predecessor_readiness_receipt_id,
            ":predecessor_digest": r.predecessor_readiness_receipt_digest,
            ":actor_kind": r.recorded_by_actor_kind,
            ":actor_id": r.recorded_by_actor_user_id,
            ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope,
            ":key": r.idempotency_key,
            ":confirmation": r.confirmation,
            ":evidence_scope": r.evidence_scope,
            ":status": r.receipt_status,
            ":effects": canonical_json(&r.effects)?,
            ":readiness": canonical_json(&r.observed_readiness)?,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("provider runtime readiness SQL projection is not exact")
    }
    Ok(())
}

pub(super) fn audit_revocation_projection(
    conn: &Connection,
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
    receipt_json: &str,
) -> Result<()> {
    let r = &receipt.revocation;
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_provider_runtime_readiness_revocations
          WHERE revocation_receipt_id=:id AND revocation_receipt_schema=:schema
            AND revocation_receipt_digest=:digest AND revocation_material_digest=:material
            AND revocation_receipt_json=:json AND canonicalization=:canonicalization
            AND digest_algorithm=:algorithm AND readiness_receipt_id=:readiness_id
            AND readiness_receipt_digest=:readiness_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest
            AND profile_id=:profile_id AND profile_digest=:profile_digest
            AND target_id=:target_id AND target_digest=:target_digest
            AND companion_id=:companion_id AND companion_digest=:companion_digest
            AND provider_id=:provider_id AND revoked_by_actor_kind=:actor_kind
            AND revoked_by_actor_user_id=:actor_id AND reason=:reason
            AND revoked_at=:revoked_at AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND revocation_status=:status
            AND effects_json=:effects AND readiness_json=:readiness)",
        named_params! {
            ":id": receipt.revocation_receipt_id,
            ":schema": receipt.schema,
            ":digest": receipt.revocation_receipt_digest,
            ":material": receipt.revocation_material_digest,
            ":json": receipt_json,
            ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm,
            ":readiness_id": r.readiness_receipt_id,
            ":readiness_digest": r.readiness_receipt_digest,
            ":binding_id": r.provider_binding_id,
            ":binding_digest": r.provider_binding_digest,
            ":candidate_id": r.candidate_id,
            ":candidate_digest": r.candidate_digest,
            ":profile_id": r.profile_id,
            ":profile_digest": r.profile_digest,
            ":target_id": r.target_id,
            ":target_digest": r.target_digest,
            ":companion_id": r.companion_id,
            ":companion_digest": r.companion_digest,
            ":provider_id": r.provider_id,
            ":actor_kind": r.revoked_by_actor_kind,
            ":actor_id": r.revoked_by_actor_user_id,
            ":reason": r.reason,
            ":revoked_at": r.revoked_at,
            ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope,
            ":key": r.idempotency_key,
            ":confirmation": r.confirmation,
            ":status": r.revocation_status,
            ":effects": canonical_json(&r.effects)?,
            ":readiness": canonical_json(&r.readiness)?,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("provider runtime readiness revocation SQL projection is not exact")
    }
    Ok(())
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES,
        )?
        .0,
    )
}
