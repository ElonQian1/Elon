use anyhow::{ensure, Result};
use rusqlite::{params, Connection};

use crate::{
    compute_federation::start_outbox::{
        canonical_start_outbox_operation_json_and_digest, ComputeStartOutboxOperationEnvelope,
        COMPUTE_START_OPERATION_CANCEL, COMPUTE_START_OUTBOX_CANONICALIZATION,
        COMPUTE_START_OUTBOX_DIGEST_ALGORITHM, COMPUTE_START_OUTBOX_OPERATION_SCHEMA,
    },
    store::new_id,
};

use super::{
    super::types::{StartOutboxCleanupReceipt, StoredStartOutboxOperation},
    CleanupSource,
};

pub(super) fn cleanup_envelope(
    source: &CleanupSource,
    operation_kind: &str,
    subject_outbox_id: &str,
    ack: Option<(&str, &str)>,
    issued_at: &str,
) -> Result<ComputeStartOutboxOperationEnvelope> {
    ensure!(
        matches!(operation_kind, "cancel" | "reconcile"),
        "unsupported cleanup operation kind"
    );
    let prepare = &source.prepare.envelope;
    let mut envelope = ComputeStartOutboxOperationEnvelope {
        schema: COMPUTE_START_OUTBOX_OPERATION_SCHEMA.to_string(),
        outbox_id: new_id(if operation_kind == COMPUTE_START_OPERATION_CANCEL {
            "start_cancel"
        } else {
            "start_reconcile"
        }),
        outbox_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        operation_kind: operation_kind.to_string(),
        operation_generation: 1,
        subject_outbox_id: Some(subject_outbox_id.to_string()),
        command_id: prepare.command_id.clone(),
        command_digest: prepare.command_digest.clone(),
        adapter_binding_digest: prepare.adapter_binding_digest.clone(),
        route_authorization_id: prepare.route_authorization_id.clone(),
        route_authorization_digest: prepare.route_authorization_digest.clone(),
        plan_id: prepare.plan_id.clone(),
        plan_digest: prepare.plan_digest.clone(),
        lease_id: prepare.lease_id.clone(),
        fencing_generation: prepare.fencing_generation,
        ack_id: ack.map(|value| value.0.to_string()),
        ack_digest: ack.map(|value| value.1.to_string()),
        application_id: None,
        application_digest: None,
        lease_authority_id: None,
        lease_authority_revision: None,
        lease_authority_digest: None,
        actor_receipt_id: prepare.actor_receipt_id.clone(),
        actor_receipt_digest: prepare.actor_receipt_digest.clone(),
        issued_at: issued_at.to_string(),
        not_before: issued_at.to_string(),
        not_after: source.cleanup_expires_at.clone(),
    };
    let (_, digest) = canonical_start_outbox_operation_json_and_digest(&envelope)?;
    envelope.outbox_digest = digest;
    Ok(envelope)
}

pub(super) fn persist_cleanup_on(
    connection: &Connection,
    source: &CleanupSource,
    envelope: &ComputeStartOutboxOperationEnvelope,
    state: &str,
    created_at: &str,
) -> Result<()> {
    let (json, digest) = canonical_start_outbox_operation_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.outbox_digest
            && envelope.issued_at < envelope.not_after
            && created_at == envelope.issued_at
            && ((envelope.operation_kind == "cancel" && state == "pending")
                || (envelope.operation_kind == "reconcile" && state == "blocked")),
        "cleanup outbox digest or horizon is invalid"
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
            ?27, ?28, ?29, ?30, ?31, ?32, ?33, 1, 0, ?31,
            NULL, NULL, 0, NULL, NULL, ?34, ?34
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
            source.prepare.provider_id,
            source.prepare.adapter_id,
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
            state,
            created_at,
        ],
    )?;
    Ok(())
}

pub(super) fn cleanup_receipt(
    cancel: &StoredStartOutboxOperation,
    reconcile: &StoredStartOutboxOperation,
    replayed: bool,
) -> StartOutboxCleanupReceipt {
    StartOutboxCleanupReceipt {
        cancel_outbox_id: cancel.envelope.outbox_id.clone(),
        cancel_outbox_digest: cancel.envelope.outbox_digest.clone(),
        reconcile_outbox_id: reconcile.envelope.outbox_id.clone(),
        reconcile_outbox_digest: reconcile.envelope.outbox_digest.clone(),
        command_id: cancel.envelope.command_id.clone(),
        ack_bound: cancel.envelope.ack_id.is_some(),
        replayed,
    }
}
