use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::canonical_supervisor_session_companion_revocation_json_and_digest,
    store::compute_external_pool_adapter_supervisor_session_policy_companion::types::StoredSupervisorSessionPolicyCompanionRevocation,
};
use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};
pub(super) fn audit_revocation(
    conn: &Connection,
    stored: StoredSupervisorSessionPolicyCompanionRevocation,
) -> Result<StoredSupervisorSessionPolicyCompanionRevocation> {
    let r = &stored.receipt;
    let v = &r.revocation;
    let json = canonical_supervisor_session_companion_revocation_json_and_digest(r)?.0;
    if json != stored.receipt_json {
        bail!("supervisor session companion revocation JSON is not canonical and exact")
    }
    let exact:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companion_revocations WHERE revocation_id=:id AND revocation_schema=:schema AND revocation_digest=:digest AND revocation_material_digest=:material AND revocation_json=:json AND canonicalization=:canonicalization AND digest_algorithm=:algorithm AND companion_id=:companion_id AND companion_digest=:companion_digest AND target_id=:target_id AND target_digest=:target_digest AND profile_id=:profile_id AND profile_digest=:profile_digest AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest AND provider_id=:provider_id AND revoked_by_actor_kind=:actor_kind AND revoked_by_actor_user_id=:actor_id AND reason=:reason AND revoked_at=:revoked_at AND recorded_at=:recorded_at AND idempotency_scope=:scope AND idempotency_key=:key AND confirmation=:confirmation AND revocation_effect=:effect AND adapter_effect=:adapter_effect AND runtime_effect=:runtime_effect AND provider_effect=:provider_effect AND credential_effect=:credential_effect AND route_effect=:route_effect AND execution_effect=:execution_effect AND usage_effect=:usage_effect AND market_effect=:market_effect AND settlement_effect=:settlement_effect AND process_spawn_ready=:spawn_ready AND ipc_session_ready=:ipc_ready AND secret_delivery_ready=:secret_ready AND broker_connect_ready=:connect_ready AND upstream_probe_observed=:probe_observed AND runtime_launch_ready=:runtime_ready AND activation_ready=:activation_ready)",named_params!{
 ":id":r.revocation_id,":schema":r.schema,":digest":r.revocation_digest,":material":r.revocation_material_digest,":json":json,":canonicalization":r.canonicalization,":algorithm":r.digest_algorithm,":companion_id":v.companion_id,":companion_digest":v.companion_digest,":target_id":v.target_id,":target_digest":v.target_digest,":profile_id":v.profile_id,":profile_digest":v.profile_digest,":binding_id":v.provider_binding_id,":binding_digest":v.provider_binding_digest,":provider_id":v.provider_id,":actor_kind":v.revoked_by_actor_kind,":actor_id":v.revoked_by_actor_user_id,":reason":v.reason,":revoked_at":v.revoked_at,":recorded_at":v.recorded_at,":scope":v.idempotency_scope,":key":v.idempotency_key,":confirmation":v.confirmation,":effect":v.revocation_effect,":adapter_effect":v.adapter_effect,":runtime_effect":v.runtime_effect,":provider_effect":v.provider_effect,":credential_effect":v.credential_effect,":route_effect":v.route_effect,":execution_effect":v.execution_effect,":usage_effect":v.usage_effect,":market_effect":v.market_effect,":settlement_effect":v.settlement_effect,":spawn_ready":v.process_spawn_ready,":ipc_ready":v.ipc_session_ready,":secret_ready":v.secret_delivery_ready,":connect_ready":v.broker_connect_ready,":probe_observed":v.upstream_probe_observed,":runtime_ready":v.runtime_launch_ready,":activation_ready":v.activation_ready},|row|row.get(0))?;
    if !exact {
        bail!("supervisor session companion revocation scalar projection drifted from sealed JSON")
    }
    Ok(stored)
}
