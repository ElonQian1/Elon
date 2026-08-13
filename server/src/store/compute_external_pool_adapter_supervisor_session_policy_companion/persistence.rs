use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::*;

pub(super) fn insert_companion(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    let c = &receipt.companion;
    let policy_json = canonical_json(&c.supervisor_session_policy)?;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_supervisor_session_policy_companions(
          companion_id,companion_schema,companion_digest,companion_material_digest,companion_json,
          canonicalization,digest_algorithm,profile_id,profile_digest,candidate_id,candidate_digest,
          delegation_id,delegation_digest,provider_binding_id,provider_binding_digest,registry_release_id,
          registry_release_digest,installation_receipt_id,installation_receipt_digest,
          installation_content_digest,route_adapter_projection_id,provider_id,provider_owner_account_id,
          provider_policy_revision,provider_digest,provider_status,logical_adapter_id,release_version,
          adapter_config_revision,adapter_config_digest,implementation_digest,capability_set_digest,
          credential_verifier_digest,service_actor_id,launch_policy_digest,process_isolation_policy_id,
          process_isolation_policy_revision,process_isolation_policy_digest,resource_policy_id,
          resource_policy_revision,resource_policy_digest,network_egress_policy_id,
          network_egress_policy_revision,network_egress_policy_digest,entrypoint_capsule_policy_id,
          entrypoint_capsule_policy_revision,entrypoint_capsule_policy_digest,target_id,target_digest,
          target_policy_digest,supervisor_session_policy_digest,supervisor_session_policy_json,sequence,
          predecessor_companion_id,predecessor_companion_digest,recorded_by_actor_kind,
          recorded_by_actor_user_id,recorded_at,idempotency_scope,idempotency_key,confirmation,
          companion_status,companion_effect,adapter_effect,runtime_effect,provider_effect,credential_effect,
          route_effect,execution_effect,usage_effect,market_effect,settlement_effect,process_spawn_ready,
          ipc_session_ready,secret_delivery_ready,broker_connect_ready,upstream_probe_observed,
          runtime_launch_ready,activation_ready
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:profile_id,:profile_digest,
          :candidate_id,:candidate_digest,:delegation_id,:delegation_digest,:binding_id,:binding_digest,
          :release_id,:release_digest,:installation_id,:installation_digest,:content_digest,
          :projection_id,:provider_id,:owner_id,:provider_revision,:provider_digest,:provider_status,
          :adapter_id,:release_version,:config_revision,:config_digest,:implementation,:capability,
          :verifier,:service_actor,:launch_policy,:isolation_id,:isolation_revision,:isolation_digest,
          :resource_id,:resource_revision,:resource_digest,:network_id,:network_revision,:network_digest,
          :capsule_id,:capsule_revision,:capsule_digest,:target_id,:target_digest,:target_policy_digest,
          :policy_digest,:policy_json,:sequence,:predecessor_id,:predecessor_digest,:actor_kind,:actor_id,
          :recorded_at,:scope,:key,:confirmation,:status,:effect,:adapter_effect,:runtime_effect,
          :provider_effect,:credential_effect,:route_effect,:execution_effect,:usage_effect,:market_effect,
          :settlement_effect,:spawn_ready,:ipc_ready,:secret_ready,:connect_ready,:probe_observed,
          :runtime_ready,:activation_ready)",
        named_params!{
            ":id":receipt.companion_id,":schema":receipt.schema,":digest":receipt.companion_digest,
            ":material":receipt.companion_material_digest,":json":canonical_supervisor_session_companion_json_and_digest(receipt)?.0,
            ":canonicalization":receipt.canonicalization,":algorithm":receipt.digest_algorithm,
            ":profile_id":c.profile_id,":profile_digest":c.profile_digest,":candidate_id":c.candidate_id,
            ":candidate_digest":c.candidate_digest,":delegation_id":c.delegation_id,":delegation_digest":c.delegation_digest,
            ":binding_id":c.provider_binding_id,":binding_digest":c.provider_binding_digest,
            ":release_id":c.registry_release_id,":release_digest":c.registry_release_digest,
            ":installation_id":c.installation_receipt_id,":installation_digest":c.installation_receipt_digest,
            ":content_digest":c.installation_content_digest,":projection_id":c.route_adapter_projection_id,
            ":provider_id":c.provider_id,":owner_id":c.provider_owner_account_id,
            ":provider_revision":c.provider_policy_revision,":provider_digest":c.provider_digest,
            ":provider_status":c.provider_status,":adapter_id":c.logical_adapter_id,
            ":release_version":c.release_version,":config_revision":c.adapter_config_revision,
            ":config_digest":c.adapter_config_digest,":implementation":c.implementation_digest,
            ":capability":c.capability_set_digest,":verifier":c.credential_verifier_digest,
            ":service_actor":c.service_actor_id,":launch_policy":c.launch_policy_digest,
            ":isolation_id":c.process_isolation_policy_id,":isolation_revision":i64::try_from(c.process_isolation_policy_revision)?,
            ":isolation_digest":c.process_isolation_policy_digest,":resource_id":c.resource_policy_id,
            ":resource_revision":i64::try_from(c.resource_policy_revision)?,":resource_digest":c.resource_policy_digest,
            ":network_id":c.network_egress_policy_id,":network_revision":i64::try_from(c.network_egress_policy_revision)?,
            ":network_digest":c.network_egress_policy_digest,":capsule_id":c.entrypoint_capsule_policy_id,
            ":capsule_revision":i64::try_from(c.entrypoint_capsule_policy_revision)?,":capsule_digest":c.entrypoint_capsule_policy_digest,
            ":target_id":c.target_id,":target_digest":c.target_digest,":target_policy_digest":c.target_policy_digest,
            ":policy_digest":c.supervisor_session_policy_digest,":policy_json":policy_json,
            ":sequence":i64::try_from(c.sequence)?,":predecessor_id":c.predecessor_companion_id,
            ":predecessor_digest":c.predecessor_companion_digest,":actor_kind":c.recorded_by_actor_kind,
            ":actor_id":c.recorded_by_actor_user_id,":recorded_at":c.recorded_at,":scope":c.idempotency_scope,
            ":key":c.idempotency_key,":confirmation":c.confirmation,":status":c.companion_status,
            ":effect":c.companion_effect,":adapter_effect":c.adapter_effect,":runtime_effect":c.runtime_effect,
            ":provider_effect":c.provider_effect,":credential_effect":c.credential_effect,":route_effect":c.route_effect,
            ":execution_effect":c.execution_effect,":usage_effect":c.usage_effect,":market_effect":c.market_effect,
            ":settlement_effect":c.settlement_effect,":spawn_ready":c.process_spawn_ready,
            ":ipc_ready":c.ipc_session_ready,":secret_ready":c.secret_delivery_ready,
            ":connect_ready":c.broker_connect_ready,":probe_observed":c.upstream_probe_observed,
            ":runtime_ready":c.runtime_launch_ready,":activation_ready":c.activation_ready,
        },
    )?;
    Ok(())
}

pub(super) fn insert_revocation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
) -> Result<()> {
    let r = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_supervisor_session_policy_companion_revocations(
          revocation_id,revocation_schema,revocation_digest,revocation_material_digest,revocation_json,
          canonicalization,digest_algorithm,companion_id,companion_digest,target_id,target_digest,
          profile_id,profile_digest,provider_binding_id,provider_binding_digest,provider_id,
          revoked_by_actor_kind,revoked_by_actor_user_id,reason,revoked_at,recorded_at,idempotency_scope,
          idempotency_key,confirmation,revocation_effect,adapter_effect,runtime_effect,provider_effect,
          credential_effect,route_effect,execution_effect,usage_effect,market_effect,settlement_effect,
          process_spawn_ready,ipc_session_ready,secret_delivery_ready,broker_connect_ready,
          upstream_probe_observed,runtime_launch_ready,activation_ready
        ) VALUES (:id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:companion_id,
          :companion_digest,:target_id,:target_digest,:profile_id,:profile_digest,:binding_id,
          :binding_digest,:provider_id,:actor_kind,:actor_id,:reason,:revoked_at,:recorded_at,:scope,:key,
          :confirmation,:effect,:adapter_effect,:runtime_effect,:provider_effect,:credential_effect,
          :route_effect,:execution_effect,:usage_effect,:market_effect,:settlement_effect,:spawn_ready,
          :ipc_ready,:secret_ready,:connect_ready,:probe_observed,:runtime_ready,:activation_ready)",
        named_params!{
            ":id":receipt.revocation_id,":schema":receipt.schema,":digest":receipt.revocation_digest,
            ":material":receipt.revocation_material_digest,":json":canonical_supervisor_session_companion_revocation_json_and_digest(receipt)?.0,
            ":canonicalization":receipt.canonicalization,":algorithm":receipt.digest_algorithm,
            ":companion_id":r.companion_id,":companion_digest":r.companion_digest,
            ":target_id":r.target_id,":target_digest":r.target_digest,":profile_id":r.profile_id,
            ":profile_digest":r.profile_digest,":binding_id":r.provider_binding_id,
            ":binding_digest":r.provider_binding_digest,":provider_id":r.provider_id,
            ":actor_kind":r.revoked_by_actor_kind,":actor_id":r.revoked_by_actor_user_id,
            ":reason":r.reason,":revoked_at":r.revoked_at,":recorded_at":r.recorded_at,
            ":scope":r.idempotency_scope,":key":r.idempotency_key,":confirmation":r.confirmation,
            ":effect":r.revocation_effect,":adapter_effect":r.adapter_effect,":runtime_effect":r.runtime_effect,
            ":provider_effect":r.provider_effect,":credential_effect":r.credential_effect,":route_effect":r.route_effect,
            ":execution_effect":r.execution_effect,":usage_effect":r.usage_effect,":market_effect":r.market_effect,
            ":settlement_effect":r.settlement_effect,":spawn_ready":r.process_spawn_ready,
            ":ipc_ready":r.ipc_session_ready,":secret_ready":r.secret_delivery_ready,
            ":connect_ready":r.broker_connect_ready,":probe_observed":r.upstream_probe_observed,
            ":runtime_ready":r.runtime_launch_ready,":activation_ready":r.activation_ready,
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
