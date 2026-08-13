use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_provider_activation_candidate::canonical_activation_candidate_json_and_digest;

use super::types::StoredCandidate;

pub(super) fn audit_candidate(
    conn: &Connection,
    stored: StoredCandidate,
) -> Result<StoredCandidate> {
    let r = &stored.receipt;
    let c = &r.candidate;
    let canonical = canonical_activation_candidate_json_and_digest(r)?.0;
    if canonical != stored.receipt_json {
        bail!("activation candidate JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_provider_activation_candidates
          WHERE candidate_id=:id AND candidate_schema=:schema AND candidate_digest=:digest
            AND candidate_material_digest=:material AND candidate_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
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
            AND logical_adapter_binding_digest=:binding_shape
            AND logical_projection_compatibility_digest=:compatibility
            AND service_actor_id=:actor_id AND sequence=:sequence
            AND predecessor_candidate_id IS :predecessor_id
            AND predecessor_candidate_digest IS :predecessor_digest
            AND checked_at=:checked_at AND recorded_at=:recorded_at
            AND candidate_status=:candidate_status AND activation_closure_status=:closure_status
            AND candidate_effect=:effect AND provider_effect=:provider_effect
            AND credential_effect=:credential_effect AND route_effect=:route_effect
            AND execution_effect=:execution_effect AND market_effect=:market_effect
            AND settlement_effect=:settlement_effect)",
        named_params! {
            ":id": r.candidate_id, ":schema": r.schema, ":digest": r.candidate_digest,
            ":material": r.candidate_material_digest, ":json": canonical,
            ":canonicalization": r.canonicalization, ":algorithm": r.digest_algorithm,
            ":delegation_id": c.delegation_id, ":delegation_digest": c.delegation_digest,
            ":binding_id": c.provider_binding_id, ":binding_digest": c.provider_binding_digest,
            ":release_id": c.registry_release_id, ":release_digest": c.registry_release_digest,
            ":installation_id": c.installation_receipt_id,
            ":installation_digest": c.installation_receipt_digest,
            ":content_digest": c.installation_content_digest,
            ":projection_id": c.route_adapter_projection_id, ":provider_id": c.provider_id,
            ":owner_id": c.provider_owner_account_id, ":provider_revision": c.provider_policy_revision,
            ":provider_digest": c.provider_digest, ":provider_status": c.provider_status,
            ":adapter_id": c.logical_adapter_id, ":release_version": c.release_version,
            ":config_revision": c.adapter_config_revision, ":config_digest": c.adapter_config_digest,
            ":implementation": c.implementation_digest, ":capability": c.capability_set_digest,
            ":verifier": c.credential_verifier_digest,
            ":binding_shape": c.logical_adapter_binding_digest,
            ":compatibility": c.logical_projection_compatibility_digest,
            ":actor_id": c.service_actor_id, ":sequence": i64::try_from(c.sequence)?,
            ":predecessor_id": c.predecessor_candidate_id,
            ":predecessor_digest": c.predecessor_candidate_digest,
            ":checked_at": c.checked_at, ":recorded_at": c.recorded_at,
            ":candidate_status": c.candidate_status, ":closure_status": c.activation_closure_status,
            ":effect": c.candidate_effect, ":provider_effect": c.provider_effect,
            ":credential_effect": c.credential_effect, ":route_effect": c.route_effect,
            ":execution_effect": c.execution_effect, ":market_effect": c.market_effect,
            ":settlement_effect": c.settlement_effect,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("activation candidate scalar projection drifted from its sealed JSON");
    }
    Ok(stored)
}
