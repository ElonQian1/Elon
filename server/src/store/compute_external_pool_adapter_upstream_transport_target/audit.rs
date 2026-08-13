use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_adapter_upstream_transport_target::{
    canonical_upstream_transport_target_json_and_digest,
    canonical_upstream_transport_target_revocation_json_and_digest,
};

use super::types::{StoredUpstreamTransportTarget, StoredUpstreamTransportTargetRevocation};

pub(super) fn audit_target(
    conn: &Connection,
    stored: StoredUpstreamTransportTarget,
) -> Result<StoredUpstreamTransportTarget> {
    let r = &stored.receipt;
    let t = &r.target;
    let canonical = canonical_upstream_transport_target_json_and_digest(r)?.0;
    let policy_json = canonical_json(&t.target_policy)?;
    if canonical != stored.receipt_json {
        bail!("upstream transport target JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets
          WHERE target_id=:id AND target_schema=:schema AND target_digest=:digest
            AND target_material_digest=:material AND target_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
            AND profile_id=:profile_id AND profile_digest=:profile_digest
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
            AND launch_policy_digest=:launch_policy
            AND network_egress_policy_id=:network_policy_id
            AND network_egress_policy_revision=:network_policy_revision
            AND network_egress_policy_digest=:network_policy_digest
            AND service_actor_id=:service_actor AND target_policy_digest=:target_policy_digest
            AND target_policy_json=:target_policy_json AND dns_hostname=:dns_hostname
            AND port=:port AND tls_server_name=:tls_server_name
            AND expected_tls_leaf_spki_sha256=:expected_spki AND sequence=:sequence
            AND predecessor_target_id IS :predecessor_id
            AND predecessor_target_digest IS :predecessor_digest
            AND recorded_by_actor_kind=:recorded_by_kind
            AND recorded_by_actor_user_id=:recorded_by_id AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND target_status=:target_status
            AND target_effect=:target_effect AND adapter_effect=:adapter_effect
            AND runtime_effect=:runtime_effect AND provider_effect=:provider_effect
            AND credential_effect=:credential_effect AND route_effect=:route_effect
            AND execution_effect=:execution_effect AND usage_effect=:usage_effect
            AND market_effect=:market_effect AND settlement_effect=:settlement_effect
            AND broker_connect_ready=:broker_connect_ready
            AND upstream_probe_observed=:upstream_probe_observed
            AND runtime_launch_ready=:runtime_launch_ready AND activation_ready=:activation_ready)",
        named_params! {
            ":id": r.target_id, ":schema": r.schema, ":digest": r.target_digest,
            ":material": r.target_material_digest, ":json": canonical,
            ":canonicalization": r.canonicalization, ":algorithm": r.digest_algorithm,
            ":profile_id": t.profile_id, ":profile_digest": t.profile_digest,
            ":candidate_id": t.candidate_id, ":candidate_digest": t.candidate_digest,
            ":delegation_id": t.delegation_id, ":delegation_digest": t.delegation_digest,
            ":binding_id": t.provider_binding_id, ":binding_digest": t.provider_binding_digest,
            ":release_id": t.registry_release_id, ":release_digest": t.registry_release_digest,
            ":installation_id": t.installation_receipt_id,
            ":installation_digest": t.installation_receipt_digest,
            ":content_digest": t.installation_content_digest,
            ":projection_id": t.route_adapter_projection_id, ":provider_id": t.provider_id,
            ":owner_id": t.provider_owner_account_id,
            ":provider_revision": t.provider_policy_revision, ":provider_digest": t.provider_digest,
            ":provider_status": t.provider_status, ":adapter_id": t.logical_adapter_id,
            ":release_version": t.release_version, ":config_revision": t.adapter_config_revision,
            ":config_digest": t.adapter_config_digest, ":implementation": t.implementation_digest,
            ":capability": t.capability_set_digest, ":verifier": t.credential_verifier_digest,
            ":launch_policy": t.launch_policy_digest,
            ":network_policy_id": t.network_egress_policy_id,
            ":network_policy_revision": i64::try_from(t.network_egress_policy_revision)?,
            ":network_policy_digest": t.network_egress_policy_digest,
            ":service_actor": t.service_actor_id, ":target_policy_digest": t.target_policy_digest,
            ":target_policy_json": policy_json, ":dns_hostname": t.dns_hostname,
            ":port": i64::from(t.port), ":tls_server_name": t.tls_server_name,
            ":expected_spki": t.expected_tls_leaf_spki_sha256,
            ":sequence": i64::try_from(t.sequence)?, ":predecessor_id": t.predecessor_target_id,
            ":predecessor_digest": t.predecessor_target_digest,
            ":recorded_by_kind": t.recorded_by_actor_kind,
            ":recorded_by_id": t.recorded_by_actor_user_id, ":recorded_at": t.recorded_at,
            ":scope": t.idempotency_scope, ":key": t.idempotency_key,
            ":confirmation": t.confirmation, ":target_status": t.target_status,
            ":target_effect": t.target_effect, ":adapter_effect": t.adapter_effect,
            ":runtime_effect": t.runtime_effect, ":provider_effect": t.provider_effect,
            ":credential_effect": t.credential_effect, ":route_effect": t.route_effect,
            ":execution_effect": t.execution_effect, ":usage_effect": t.usage_effect,
            ":market_effect": t.market_effect, ":settlement_effect": t.settlement_effect,
            ":broker_connect_ready": t.broker_connect_ready,
            ":upstream_probe_observed": t.upstream_probe_observed,
            ":runtime_launch_ready": t.runtime_launch_ready,
            ":activation_ready": t.activation_ready,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("upstream transport target scalar projection drifted from sealed JSON");
    }
    Ok(stored)
}

pub(super) fn audit_revocation(
    conn: &Connection,
    stored: StoredUpstreamTransportTargetRevocation,
) -> Result<StoredUpstreamTransportTargetRevocation> {
    let receipt = &stored.receipt;
    let r = &receipt.revocation;
    let canonical = canonical_upstream_transport_target_revocation_json_and_digest(receipt)?.0;
    if canonical != stored.receipt_json {
        bail!("upstream transport target revocation JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_upstream_transport_target_revocations
          WHERE revocation_id=:id AND revocation_schema=:schema AND revocation_digest=:digest
            AND revocation_material_digest=:material AND revocation_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
            AND target_id=:target_id AND target_digest=:target_digest
            AND profile_id=:profile_id AND profile_digest=:profile_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND provider_id=:provider_id AND revoked_by_actor_kind=:revoked_by_kind
            AND revoked_by_actor_user_id=:revoked_by_id AND reason=:reason
            AND revoked_at=:revoked_at AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND revocation_effect=:revocation_effect
            AND adapter_effect=:adapter_effect AND runtime_effect=:runtime_effect
            AND provider_effect=:provider_effect AND credential_effect=:credential_effect
            AND route_effect=:route_effect AND execution_effect=:execution_effect
            AND usage_effect=:usage_effect AND market_effect=:market_effect
            AND settlement_effect=:settlement_effect
            AND broker_connect_ready=:broker_connect_ready
            AND upstream_probe_observed=:upstream_probe_observed
            AND runtime_launch_ready=:runtime_launch_ready AND activation_ready=:activation_ready)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest, ":material": receipt.revocation_material_digest,
            ":json": canonical, ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm, ":target_id": r.target_id,
            ":target_digest": r.target_digest, ":profile_id": r.profile_id,
            ":profile_digest": r.profile_digest, ":binding_id": r.provider_binding_id,
            ":binding_digest": r.provider_binding_digest, ":provider_id": r.provider_id,
            ":revoked_by_kind": r.revoked_by_actor_kind,
            ":revoked_by_id": r.revoked_by_actor_user_id, ":reason": r.reason,
            ":revoked_at": r.revoked_at, ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope, ":key": r.idempotency_key,
            ":confirmation": r.confirmation, ":revocation_effect": r.revocation_effect,
            ":adapter_effect": r.adapter_effect, ":runtime_effect": r.runtime_effect,
            ":provider_effect": r.provider_effect, ":credential_effect": r.credential_effect,
            ":route_effect": r.route_effect, ":execution_effect": r.execution_effect,
            ":usage_effect": r.usage_effect, ":market_effect": r.market_effect,
            ":settlement_effect": r.settlement_effect,
            ":broker_connect_ready": r.broker_connect_ready,
            ":upstream_probe_observed": r.upstream_probe_observed,
            ":runtime_launch_ready": r.runtime_launch_ready,
            ":activation_ready": r.activation_ready,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("upstream transport target revocation scalar projection drifted from sealed JSON");
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
