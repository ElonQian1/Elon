//! Exact persistence of the immutable prepare operation before command insertion.

use anyhow::{ensure, Result};
use rusqlite::{params, Connection};

use crate::compute_federation::start_outbox::{
    canonical_start_outbox_operation_json_and_digest, ValidatedComputeStartOutboxOperation,
};

pub(super) fn persist_prepare_operation_on(
    connection: &Connection,
    operation: &ValidatedComputeStartOutboxOperation,
    created_at: &str,
) -> Result<()> {
    let envelope = operation.envelope();
    let route = operation.route_authorization().envelope();
    let (json, digest) = canonical_start_outbox_operation_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.outbox_digest,
        "prepare outbox digest mismatch"
    );
    ensure!(
        created_at < envelope.not_after.as_str(),
        "prepare outbox is already outside its delivery window"
    );
    connection.execute(
        "INSERT INTO compute_attempt_start_outbox (
            outbox_id, outbox_schema, outbox_digest, outbox_json,
            canonicalization, digest_algorithm, operation_kind, operation_generation,
            subject_outbox_id, command_id, command_digest, provider_id, adapter_id,
            adapter_binding_digest, route_authorization_id, route_authorization_digest,
            actor_receipt_id, actor_receipt_digest, plan_id, plan_digest, lease_id,
            fencing_generation, ack_id, ack_digest, application_id, application_digest,
            lease_authority_id, lease_authority_revision, lease_authority_digest,
            issued_at, not_before, not_after, state, state_revision, attempt_count,
            next_attempt_at, claim_owner_id, claim_token_digest, claim_generation,
            claim_expires_at, last_failure_code, created_at, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26,
            ?27, ?28, ?29, ?30, ?31, ?32, 'pending', 1, 0, ?31,
            NULL, NULL, 0, NULL, NULL, ?33, ?33
         )",
        params![
            envelope.outbox_id,
            envelope.schema,
            envelope.outbox_digest,
            json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            envelope.operation_kind,
            envelope.operation_generation,
            envelope.subject_outbox_id,
            envelope.command_id,
            envelope.command_digest,
            route.authorization.provider.provider_id,
            route.authorization.route.adapter.adapter_id,
            envelope.adapter_binding_digest,
            envelope.route_authorization_id,
            envelope.route_authorization_digest,
            envelope.actor_receipt_id,
            envelope.actor_receipt_digest,
            envelope.plan_id,
            envelope.plan_digest,
            envelope.lease_id,
            envelope.fencing_generation,
            envelope.ack_id,
            envelope.ack_digest,
            envelope.application_id,
            envelope.application_digest,
            envelope.lease_authority_id,
            envelope.lease_authority_revision,
            envelope.lease_authority_digest,
            envelope.issued_at,
            envelope.not_before,
            envelope.not_after,
            created_at,
        ],
    )?;
    Ok(())
}
