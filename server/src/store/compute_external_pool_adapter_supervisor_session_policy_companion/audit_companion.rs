use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::canonical_supervisor_session_companion_json_and_digest,
    store::compute_external_pool_adapter_supervisor_session_policy_companion::types::StoredSupervisorSessionPolicyCompanion,
};

pub(super) fn audit_companion(
    conn: &Connection,
    stored: StoredSupervisorSessionPolicyCompanion,
) -> Result<StoredSupervisorSessionPolicyCompanion> {
    let r = &stored.receipt;
    let c = &r.companion;
    let json = canonical_supervisor_session_companion_json_and_digest(r)?.0;
    let policy_json = canonical_json(&c.supervisor_session_policy)?;
    if json != stored.receipt_json {
        bail!("supervisor session companion JSON is not canonical and exact")
    }
    let exact:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions WHERE
 companion_id=:id AND companion_schema=:schema AND companion_digest=:digest AND companion_material_digest=:material AND companion_json=:json AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
 AND profile_id=:profile_id AND profile_digest=:profile_digest AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest AND delegation_id=:delegation_id AND delegation_digest=:delegation_digest
 AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest AND registry_release_id=:release_id AND registry_release_digest=:release_digest AND installation_receipt_id=:installation_id AND installation_receipt_digest=:installation_digest AND installation_content_digest=:content_digest
 AND route_adapter_projection_id=:projection_id AND provider_id=:provider_id AND provider_owner_account_id=:owner_id AND provider_policy_revision=:provider_revision AND provider_digest=:provider_digest AND provider_status=:provider_status AND logical_adapter_id=:adapter_id AND release_version=:release_version AND adapter_config_revision=:config_revision AND adapter_config_digest=:config_digest AND implementation_digest=:implementation AND capability_set_digest=:capability AND credential_verifier_digest=:verifier AND service_actor_id=:service_actor
 AND launch_policy_digest=:launch_policy AND process_isolation_policy_id=:isolation_id AND process_isolation_policy_revision=:isolation_revision AND process_isolation_policy_digest=:isolation_digest AND resource_policy_id=:resource_id AND resource_policy_revision=:resource_revision AND resource_policy_digest=:resource_digest AND network_egress_policy_id=:network_id AND network_egress_policy_revision=:network_revision AND network_egress_policy_digest=:network_digest
 AND entrypoint_capsule_policy_id=:capsule_id AND entrypoint_capsule_policy_revision=:capsule_revision AND entrypoint_capsule_policy_digest=:capsule_digest AND target_id=:target_id AND target_digest=:target_digest AND target_policy_digest=:target_policy_digest AND supervisor_session_policy_digest=:policy_digest AND supervisor_session_policy_json=:policy_json AND sequence=:sequence AND predecessor_companion_id IS :predecessor_id AND predecessor_companion_digest IS :predecessor_digest
 AND recorded_by_actor_kind=:actor_kind AND recorded_by_actor_user_id=:actor_id AND recorded_at=:recorded_at AND idempotency_scope=:scope AND idempotency_key=:key AND confirmation=:confirmation AND companion_status=:status AND companion_effect=:effect AND adapter_effect=:adapter_effect AND runtime_effect=:runtime_effect AND provider_effect=:provider_effect AND credential_effect=:credential_effect AND route_effect=:route_effect AND execution_effect=:execution_effect AND usage_effect=:usage_effect AND market_effect=:market_effect AND settlement_effect=:settlement_effect
 AND process_spawn_ready=:spawn_ready AND ipc_session_ready=:ipc_ready AND secret_delivery_ready=:secret_ready AND broker_connect_ready=:connect_ready AND upstream_probe_observed=:probe_observed AND runtime_launch_ready=:runtime_ready AND activation_ready=:activation_ready)",named_params!{
 ":id":r.companion_id,":schema":r.schema,":digest":r.companion_digest,":material":r.companion_material_digest,":json":json,":canonicalization":r.canonicalization,":algorithm":r.digest_algorithm,
 ":profile_id":c.profile_id,":profile_digest":c.profile_digest,":candidate_id":c.candidate_id,":candidate_digest":c.candidate_digest,":delegation_id":c.delegation_id,":delegation_digest":c.delegation_digest,":binding_id":c.provider_binding_id,":binding_digest":c.provider_binding_digest,":release_id":c.registry_release_id,":release_digest":c.registry_release_digest,":installation_id":c.installation_receipt_id,":installation_digest":c.installation_receipt_digest,":content_digest":c.installation_content_digest,
 ":projection_id":c.route_adapter_projection_id,":provider_id":c.provider_id,":owner_id":c.provider_owner_account_id,":provider_revision":c.provider_policy_revision,":provider_digest":c.provider_digest,":provider_status":c.provider_status,":adapter_id":c.logical_adapter_id,":release_version":c.release_version,":config_revision":c.adapter_config_revision,":config_digest":c.adapter_config_digest,":implementation":c.implementation_digest,":capability":c.capability_set_digest,":verifier":c.credential_verifier_digest,":service_actor":c.service_actor_id,
 ":launch_policy":c.launch_policy_digest,":isolation_id":c.process_isolation_policy_id,":isolation_revision":i64::try_from(c.process_isolation_policy_revision)?,":isolation_digest":c.process_isolation_policy_digest,":resource_id":c.resource_policy_id,":resource_revision":i64::try_from(c.resource_policy_revision)?,":resource_digest":c.resource_policy_digest,":network_id":c.network_egress_policy_id,":network_revision":i64::try_from(c.network_egress_policy_revision)?,":network_digest":c.network_egress_policy_digest,
 ":capsule_id":c.entrypoint_capsule_policy_id,":capsule_revision":i64::try_from(c.entrypoint_capsule_policy_revision)?,":capsule_digest":c.entrypoint_capsule_policy_digest,":target_id":c.target_id,":target_digest":c.target_digest,":target_policy_digest":c.target_policy_digest,":policy_digest":c.supervisor_session_policy_digest,":policy_json":policy_json,":sequence":i64::try_from(c.sequence)?,":predecessor_id":c.predecessor_companion_id,":predecessor_digest":c.predecessor_companion_digest,
 ":actor_kind":c.recorded_by_actor_kind,":actor_id":c.recorded_by_actor_user_id,":recorded_at":c.recorded_at,":scope":c.idempotency_scope,":key":c.idempotency_key,":confirmation":c.confirmation,":status":c.companion_status,":effect":c.companion_effect,":adapter_effect":c.adapter_effect,":runtime_effect":c.runtime_effect,":provider_effect":c.provider_effect,":credential_effect":c.credential_effect,":route_effect":c.route_effect,":execution_effect":c.execution_effect,":usage_effect":c.usage_effect,":market_effect":c.market_effect,":settlement_effect":c.settlement_effect,
 ":spawn_ready":c.process_spawn_ready,":ipc_ready":c.ipc_session_ready,":secret_ready":c.secret_delivery_ready,":connect_ready":c.broker_connect_ready,":probe_observed":c.upstream_probe_observed,":runtime_ready":c.runtime_launch_ready,":activation_ready":c.activation_ready},|row|row.get(0))?;
    if !exact {
        bail!("supervisor session companion scalar projection drifted from sealed JSON")
    }
    Ok(stored)
}
fn canonical_json<T: serde::Serialize>(v: &T) -> Result<String> {
    Ok(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            v,
            1024 * 1024,
        )?
        .0,
    )
}
