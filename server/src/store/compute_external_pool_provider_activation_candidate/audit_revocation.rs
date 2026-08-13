use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_provider_activation_candidate::canonical_activation_delegation_revocation_json_and_digest;

use super::types::StoredRevocation;

pub(super) fn audit_revocation(
    conn: &Connection,
    stored: StoredRevocation,
) -> Result<StoredRevocation> {
    let receipt = &stored.receipt;
    let r = &receipt.revocation;
    let canonical = canonical_activation_delegation_revocation_json_and_digest(receipt)?.0;
    if canonical != stored.receipt_json {
        bail!("activation delegation revocation JSON is not canonical and exact");
    }
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_provider_activation_delegation_revocations
          WHERE revocation_id=:id AND revocation_schema=:schema AND revocation_digest=:digest
            AND revocation_material_digest=:material AND revocation_json=:json
            AND canonicalization=:canonicalization AND digest_algorithm=:algorithm
            AND delegation_id=:delegation_id AND delegation_digest=:delegation_digest
            AND candidate_id=:candidate_id AND candidate_digest=:candidate_digest
            AND provider_binding_id=:binding_id AND provider_binding_digest=:binding_digest
            AND provider_id=:provider_id AND revoked_by_owner_user_id=:revoked_by
            AND reason=:reason AND revoked_at=:revoked_at AND recorded_at=:recorded_at
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation AND revocation_effect=:effect
            AND provider_effect=:provider_effect AND credential_effect=:credential_effect
            AND route_effect=:route_effect AND execution_effect=:execution_effect
            AND market_effect=:market_effect AND settlement_effect=:settlement_effect)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest, ":material": receipt.revocation_material_digest,
            ":json": canonical, ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm, ":delegation_id": r.delegation_id,
            ":delegation_digest": r.delegation_digest, ":candidate_id": r.candidate_id,
            ":candidate_digest": r.candidate_digest, ":binding_id": r.provider_binding_id,
            ":binding_digest": r.provider_binding_digest, ":provider_id": r.provider_id,
            ":revoked_by": r.revoked_by_owner_user_id, ":reason": r.reason,
            ":revoked_at": r.revoked_at, ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope, ":key": r.idempotency_key,
            ":confirmation": r.confirmation, ":effect": r.revocation_effect,
            ":provider_effect": r.provider_effect, ":credential_effect": r.credential_effect,
            ":route_effect": r.route_effect, ":execution_effect": r.execution_effect,
            ":market_effect": r.market_effect, ":settlement_effect": r.settlement_effect,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("activation revocation scalar projection drifted from its sealed JSON");
    }
    Ok(stored)
}
