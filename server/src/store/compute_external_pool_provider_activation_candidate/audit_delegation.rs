use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_provider_activation_candidate::canonical_activation_delegation_json_and_digest;

use super::types::StoredDelegation;

pub(super) fn audit_delegation(
    conn: &Connection,
    stored: StoredDelegation,
) -> Result<StoredDelegation> {
    let r = &stored.receipt;
    let d = &r.delegation;
    let canonical = canonical_activation_delegation_json_and_digest(r)?.0;
    let routes = canonical_json(&d.allowed_route_kinds)?;
    let phases = canonical_json(&d.allowed_actor_phases)?;
    if canonical != stored.receipt_json {
        bail!("activation delegation JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_provider_activation_delegations
          WHERE delegation_id=:id AND delegation_schema=:schema
            AND delegation_digest=:digest AND delegation_material_digest=:material
            AND delegation_json=:json AND canonicalization=:canonicalization
            AND digest_algorithm=:algorithm AND provider_binding_id=:binding_id
            AND provider_binding_digest=:binding_digest AND registry_release_id=:release_id
            AND registry_release_digest=:release_digest
            AND route_adapter_projection_id=:projection_id AND provider_id=:provider_id
            AND provider_owner_account_id=:owner_id
            AND provider_policy_revision=:provider_revision AND provider_digest=:provider_digest
            AND provider_status=:provider_status AND logical_adapter_id=:adapter_id
            AND release_version=:release_version AND adapter_config_revision=:config_revision
            AND adapter_config_digest=:config_digest AND service_actor_id=:actor_id
            AND service_actor_kind=:actor_kind AND allowed_route_kinds_json=:routes
            AND allowed_actor_phases_json=:phases AND issued_by_owner_user_id=:issued_by
            AND issued_at=:issued_at AND recorded_at=:recorded_at AND sequence=:sequence
            AND predecessor_delegation_id IS :predecessor_id
            AND predecessor_delegation_digest IS :predecessor_digest
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND delegation_effect=:effect
            AND provider_effect=:provider_effect AND credential_effect=:credential_effect
            AND route_effect=:route_effect AND execution_effect=:execution_effect
            AND market_effect=:market_effect AND settlement_effect=:settlement_effect)",
        named_params! {
            ":id": r.delegation_id, ":schema": r.schema, ":digest": r.delegation_digest,
            ":material": r.delegation_material_digest, ":json": canonical,
            ":canonicalization": r.canonicalization, ":algorithm": r.digest_algorithm,
            ":binding_id": d.provider_binding_id, ":binding_digest": d.provider_binding_digest,
            ":release_id": d.registry_release_id, ":release_digest": d.registry_release_digest,
            ":projection_id": d.route_adapter_projection_id, ":provider_id": d.provider_id,
            ":owner_id": d.provider_owner_account_id, ":provider_revision": d.provider_policy_revision,
            ":provider_digest": d.provider_digest, ":provider_status": d.provider_status,
            ":adapter_id": d.logical_adapter_id, ":release_version": d.release_version,
            ":config_revision": d.adapter_config_revision, ":config_digest": d.adapter_config_digest,
            ":actor_id": d.service_actor_id, ":actor_kind": d.service_actor_kind,
            ":routes": routes, ":phases": phases, ":issued_by": d.issued_by_owner_user_id,
            ":issued_at": d.issued_at, ":recorded_at": d.recorded_at,
            ":sequence": i64::try_from(d.sequence)?, ":predecessor_id": d.predecessor_delegation_id,
            ":predecessor_digest": d.predecessor_delegation_digest,
            ":scope": d.idempotency_scope, ":key": d.idempotency_key,
            ":confirmation": d.confirmation, ":effect": d.delegation_effect,
            ":provider_effect": d.provider_effect, ":credential_effect": d.credential_effect,
            ":route_effect": d.route_effect, ":execution_effect": d.execution_effect,
            ":market_effect": d.market_effect, ":settlement_effect": d.settlement_effect,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("activation delegation scalar projection drifted from its sealed JSON");
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
