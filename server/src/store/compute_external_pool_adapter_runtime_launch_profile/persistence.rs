use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::external_pool_adapter_runtime_launch_profile::{
    canonical_runtime_launch_profile_json_and_digest,
    canonical_runtime_launch_profile_revocation_json_and_digest,
    ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
};

pub(super) fn insert_profile(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> Result<()> {
    let p = &receipt.profile;
    let policy_json = canonical_json(&p.launch_policy)?;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_launch_profiles(
          profile_id,profile_schema,profile_digest,profile_material_digest,profile_json,
          canonicalization,digest_algorithm,candidate_id,candidate_digest,delegation_id,
          delegation_digest,provider_binding_id,provider_binding_digest,registry_release_id,
          registry_release_digest,installation_receipt_id,installation_receipt_digest,
          installation_content_digest,route_adapter_projection_id,provider_id,
          provider_owner_account_id,provider_policy_revision,provider_digest,provider_status,
          logical_adapter_id,release_version,adapter_config_revision,adapter_config_digest,
          implementation_digest,capability_set_digest,credential_verifier_digest,
          credential_ref_scheme,credential_locator_commitment,service_actor_id,
          entrypoint_relative_path,entrypoint_path_digest,entrypoint_sha256,entrypoint_size_bytes,
          entry_inventory_digest,installed_file_count,installed_total_bytes,launch_policy_digest,
          launch_policy_json,sequence,predecessor_profile_id,predecessor_profile_digest,
          recorded_by_actor_kind,recorded_by_actor_user_id,recorded_at,idempotency_scope,
          idempotency_key,confirmation,profile_status,profile_effect,adapter_effect,
          runtime_effect,provider_effect,credential_effect,route_effect,execution_effect,
          usage_effect,market_effect,
          settlement_effect
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:candidate_id,
          :candidate_digest,:delegation_id,:delegation_digest,:binding_id,:binding_digest,
          :release_id,:release_digest,:installation_id,:installation_digest,:content_digest,
          :projection_id,:provider_id,:owner_id,:provider_revision,:provider_digest,
          :provider_status,:adapter_id,:release_version,:config_revision,:config_digest,
          :implementation,:capability,:verifier,:credential_ref_scheme,
          :credential_locator_commitment,:actor_id,:entrypoint_relative_path,:entrypoint_path_digest,
          :entrypoint_sha256,:entrypoint_size,:inventory_digest,:file_count,:total_bytes,
          :policy_digest,:policy_json,:sequence,:predecessor_id,:predecessor_digest,
          :recorded_by_kind,:recorded_by_id,:recorded_at,:scope,:key,:confirmation,
          :profile_status,:profile_effect,:adapter_effect,:runtime_effect,:provider_effect,
          :credential_effect,:route_effect,:execution_effect,:usage_effect,:market_effect,
          :settlement_effect)",
        named_params! {
            ":id": receipt.profile_id, ":schema": receipt.schema,
            ":digest": receipt.profile_digest, ":material": receipt.profile_material_digest,
            ":json": canonical_runtime_launch_profile_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
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
            ":verifier": p.credential_verifier_digest, ":actor_id": p.service_actor_id,
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
    )?;
    Ok(())
}

pub(super) fn insert_revocation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
) -> Result<()> {
    let r = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_launch_profile_revocations(
          revocation_id,revocation_schema,revocation_digest,revocation_material_digest,
          revocation_json,canonicalization,digest_algorithm,profile_id,profile_digest,
          provider_binding_id,provider_binding_digest,candidate_id,candidate_digest,
          revoked_by_actor_kind,revoked_by_actor_user_id,reason,revoked_at,recorded_at,
          idempotency_scope,idempotency_key,confirmation,revocation_effect,adapter_effect,
          runtime_effect,provider_effect,credential_effect,route_effect,execution_effect,
          usage_effect,market_effect,
          settlement_effect
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:profile_id,
          :profile_digest,:binding_id,:binding_digest,:candidate_id,:candidate_digest,
          :revoked_by_kind,:revoked_by_id,:reason,:revoked_at,:recorded_at,:scope,:key,
          :confirmation,:revocation_effect,:adapter_effect,:runtime_effect,:provider_effect,
          :credential_effect,:route_effect,:execution_effect,:usage_effect,:market_effect,
          :settlement_effect)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest,
            ":material": receipt.revocation_material_digest,
            ":json": canonical_runtime_launch_profile_revocation_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":profile_id": r.profile_id, ":profile_digest": r.profile_digest,
            ":binding_id": r.provider_binding_id, ":binding_digest": r.provider_binding_digest,
            ":candidate_id": r.candidate_id, ":candidate_digest": r.candidate_digest,
            ":revoked_by_kind": r.revoked_by_actor_kind,
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
    )?;
    Ok(())
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
