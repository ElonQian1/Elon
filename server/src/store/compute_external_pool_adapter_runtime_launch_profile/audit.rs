use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_adapter_runtime_launch_profile::{
    canonical_runtime_launch_profile_json_and_digest,
    canonical_runtime_launch_profile_revocation_json_and_digest,
};

use super::types::{StoredRuntimeLaunchProfile, StoredRuntimeLaunchProfileRevocation};

pub(super) fn audit_profile(
    conn: &Connection,
    stored: StoredRuntimeLaunchProfile,
) -> Result<StoredRuntimeLaunchProfile> {
    let r = &stored.receipt;
    let p = &r.profile;
    let canonical = canonical_runtime_launch_profile_json_and_digest(r)?.0;
    let policy_json = canonical_json(&p.launch_policy)?;
    if canonical != stored.receipt_json {
        bail!("runtime launch profile JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles
          WHERE profile_id=:id AND profile_schema=:schema AND profile_digest=:digest
            AND profile_material_digest=:material AND profile_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
            AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest
            AND delegation_id=:delegation_id AND delegation_digest=:delegation_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND registry_release_id=:release_id AND registry_release_digest=:release_digest
            AND installation_receipt_id=:installation_id
            AND installation_receipt_digest=:installation_digest
            AND installation_content_digest=:content_digest
            AND route_adapter_projection_id=:projection_id AND provider_id=:provider_id
            AND provider_owner_account_id=:owner_id
            AND provider_policy_revision=:provider_revision AND provider_digest=:provider_digest
            AND provider_status=:provider_status AND logical_adapter_id=:adapter_id
            AND release_version=:release_version AND adapter_config_revision=:config_revision
            AND adapter_config_digest=:config_digest AND implementation_digest=:implementation
            AND capability_set_digest=:capability AND credential_verifier_digest=:verifier
            AND credential_ref_scheme=:credential_ref_scheme
            AND credential_locator_commitment=:credential_locator_commitment
            AND service_actor_id=:service_actor
            AND entrypoint_relative_path=:entrypoint_relative_path
            AND entrypoint_path_digest=:entrypoint_path_digest
            AND entrypoint_sha256=:entrypoint_sha256 AND entrypoint_size_bytes=:entrypoint_size
            AND entry_inventory_digest=:inventory_digest AND installed_file_count=:file_count
            AND installed_total_bytes=:total_bytes AND launch_policy_digest=:policy_digest
            AND launch_policy_json=:policy_json AND sequence=:sequence
            AND predecessor_profile_id IS :predecessor_id
            AND predecessor_profile_digest IS :predecessor_digest
            AND recorded_by_actor_kind=:recorded_by_kind
            AND recorded_by_actor_user_id=:recorded_by_id AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND profile_status=:profile_status
            AND profile_effect=:profile_effect AND runtime_effect=:runtime_effect
            AND adapter_effect=:adapter_effect
            AND provider_effect=:provider_effect AND credential_effect=:credential_effect
            AND route_effect=:route_effect AND execution_effect=:execution_effect
            AND usage_effect=:usage_effect
            AND market_effect=:market_effect AND settlement_effect=:settlement_effect)",
        named_params! {
            ":id": r.profile_id, ":schema": r.schema, ":digest": r.profile_digest,
            ":material": r.profile_material_digest, ":json": canonical,
            ":canonicalization": r.canonicalization, ":algorithm": r.digest_algorithm,
            ":candidate_id": p.candidate_id, ":candidate_digest": p.candidate_digest,
            ":delegation_id": p.delegation_id, ":delegation_digest": p.delegation_digest,
            ":binding_id": p.provider_binding_id, ":binding_digest": p.provider_binding_digest,
            ":release_id": p.registry_release_id, ":release_digest": p.registry_release_digest,
            ":installation_id": p.installation_receipt_id,
            ":installation_digest": p.installation_receipt_digest,
            ":content_digest": p.installation_content_digest,
            ":projection_id": p.route_adapter_projection_id, ":provider_id": p.provider_id,
            ":owner_id": p.provider_owner_account_id, ":provider_revision": p.provider_policy_revision,
            ":provider_digest": p.provider_digest, ":provider_status": p.provider_status,
            ":adapter_id": p.logical_adapter_id, ":release_version": p.release_version,
            ":config_revision": p.adapter_config_revision, ":config_digest": p.adapter_config_digest,
            ":implementation": p.implementation_digest, ":capability": p.capability_set_digest,
            ":verifier": p.credential_verifier_digest, ":service_actor": p.service_actor_id,
            ":credential_ref_scheme": p.credential_ref_scheme,
            ":credential_locator_commitment": p.credential_locator_commitment,
            ":entrypoint_relative_path": p.entrypoint_relative_path,
            ":entrypoint_path_digest": p.entrypoint_path_digest,
            ":entrypoint_sha256": p.entrypoint_sha256, ":entrypoint_size": i64::try_from(p.entrypoint_size_bytes)?,
            ":inventory_digest": p.entry_inventory_digest, ":file_count": i64::try_from(p.installed_file_count)?,
            ":total_bytes": i64::try_from(p.installed_total_bytes)?,
            ":policy_digest": p.launch_policy_digest, ":policy_json": policy_json,
            ":sequence": i64::try_from(p.sequence)?, ":predecessor_id": p.predecessor_profile_id,
            ":predecessor_digest": p.predecessor_profile_digest,
            ":recorded_by_kind": p.recorded_by_actor_kind,
            ":recorded_by_id": p.recorded_by_actor_user_id, ":recorded_at": p.recorded_at,
            ":scope": p.idempotency_scope, ":key": p.idempotency_key,
            ":confirmation": p.confirmation, ":profile_status": p.profile_status,
            ":profile_effect": p.profile_effect, ":runtime_effect": p.runtime_effect,
            ":adapter_effect": p.adapter_effect,
            ":provider_effect": p.provider_effect, ":credential_effect": p.credential_effect,
            ":route_effect": p.route_effect, ":execution_effect": p.execution_effect,
            ":usage_effect": p.usage_effect,
            ":market_effect": p.market_effect, ":settlement_effect": p.settlement_effect,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("runtime launch profile scalar projection drifted from sealed JSON");
    }
    Ok(stored)
}

pub(super) fn audit_revocation(
    conn: &Connection,
    stored: StoredRuntimeLaunchProfileRevocation,
) -> Result<StoredRuntimeLaunchProfileRevocation> {
    let receipt = &stored.receipt;
    let r = &receipt.revocation;
    let canonical = canonical_runtime_launch_profile_revocation_json_and_digest(receipt)?.0;
    if canonical != stored.receipt_json {
        bail!("runtime launch revocation JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_runtime_launch_profile_revocations
          WHERE revocation_id=:id AND revocation_schema=:schema AND revocation_digest=:digest
            AND revocation_material_digest=:material AND revocation_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
            AND profile_id=:profile_id AND profile_digest=:profile_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest
            AND revoked_by_actor_kind=:revoked_by_kind
            AND revoked_by_actor_user_id=:revoked_by_id AND reason=:reason
            AND revoked_at=:revoked_at AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND revocation_effect=:revocation_effect
            AND runtime_effect=:runtime_effect AND adapter_effect=:adapter_effect
            AND provider_effect=:provider_effect
            AND credential_effect=:credential_effect AND route_effect=:route_effect
            AND execution_effect=:execution_effect AND usage_effect=:usage_effect
            AND market_effect=:market_effect
            AND settlement_effect=:settlement_effect)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest, ":material": receipt.revocation_material_digest,
            ":json": canonical, ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm, ":profile_id": r.profile_id,
            ":profile_digest": r.profile_digest, ":binding_id": r.provider_binding_id,
            ":binding_digest": r.provider_binding_digest, ":candidate_id": r.candidate_id,
            ":candidate_digest": r.candidate_digest, ":revoked_by_kind": r.revoked_by_actor_kind,
            ":revoked_by_id": r.revoked_by_actor_user_id, ":reason": r.reason,
            ":revoked_at": r.revoked_at, ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope, ":key": r.idempotency_key,
            ":confirmation": r.confirmation, ":revocation_effect": r.revocation_effect,
            ":runtime_effect": r.runtime_effect, ":provider_effect": r.provider_effect,
            ":adapter_effect": r.adapter_effect,
            ":credential_effect": r.credential_effect, ":route_effect": r.route_effect,
            ":execution_effect": r.execution_effect, ":market_effect": r.market_effect,
            ":usage_effect": r.usage_effect,
            ":settlement_effect": r.settlement_effect,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("runtime launch revocation scalar projection drifted from sealed JSON");
    }
    Ok(stored)
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            1024 * 1024,
        )?
        .0,
    )
}
