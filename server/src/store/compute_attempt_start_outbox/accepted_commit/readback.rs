use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_attempt_dispatch_actor_receipt_json_and_digest,
    ComputeAttemptDispatchActorReceiptEnvelope, COMPUTE_ACTOR_RECEIPT_PHASE_APPLICATION,
};

use super::{
    derive::derive_closure, source::load_source_for_replay_on, AcceptedStartCommitClosureReceipt,
    DerivedAcceptedCommitClosure,
};

pub(super) fn audit_on(
    connection: &Connection,
    command_id: &str,
    replayed: bool,
) -> Result<AcceptedStartCommitClosureReceipt> {
    let (actor_json, actor_digest) = connection
        .query_row(
            "SELECT actor_receipt_json, actor_receipt_digest
               FROM compute_attempt_dispatch_actor_receipts
              WHERE command_id=?1 AND actor_phase=?2",
            params![command_id, COMPUTE_ACTOR_RECEIPT_PHASE_APPLICATION],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted application actor closure is missing"))?;
    let actor: ComputeAttemptDispatchActorReceiptEnvelope = serde_json::from_str(&actor_json)?;
    let (canonical_actor, recomputed_actor) =
        canonical_attempt_dispatch_actor_receipt_json_and_digest(&actor)?;
    ensure!(
        canonical_actor == actor_json
            && actor.actor_receipt_digest == actor_digest
            && recomputed_actor == actor_digest,
        "accepted application actor failed canonical replay audit"
    );
    let source = load_source_for_replay_on(connection, command_id)?;
    let expected = derive_closure(&source, &actor.recorded_at)?;
    audit_expected_immutable_on(connection, &expected, false)?;
    Ok(expected.receipt(replayed))
}

pub(super) fn audit_expected_rows_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
) -> Result<()> {
    audit_expected_immutable_on(connection, expected, true)
}

fn audit_expected_immutable_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
    require_initial_pending: bool,
) -> Result<()> {
    let actor = connection
        .query_row(
            "SELECT actor_receipt_json, actor_receipt_digest
               FROM compute_attempt_dispatch_actor_receipts WHERE actor_receipt_id=?1",
            params![expected.actor.actor_receipt_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted application actor is not visible after insert"))?;
    ensure!(
        actor.0 == expected.actor_json && actor.1 == expected.actor.actor_receipt_digest,
        "accepted application actor exact readback failed"
    );
    let authority = connection
        .query_row(
            "SELECT authority_json, lease_authority_digest
               FROM compute_attempt_lease_authority_bindings
              WHERE lease_authority_id=?1 AND authority_revision=?2",
            params![
                expected.authority.lease_authority_id,
                expected.authority.authority_revision
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted lease authority is not visible after insert"))?;
    ensure!(
        authority.0 == expected.authority_json
            && authority.1 == expected.authority.lease_authority_digest,
        "accepted lease authority exact readback failed"
    );
    let commit = super::super::read::outbox_by_id_on(connection, &expected.commit.outbox_id)?
        .ok_or_else(|| anyhow!("accepted Commit outbox is not visible after insert"))?;
    ensure!(
        commit.envelope == expected.commit
            && commit.provider_id == expected.provider_id
            && commit.adapter_id == expected.adapter_id
            && commit.projection.created_at == expected.commit.issued_at,
        "accepted Commit outbox immutable readback failed"
    );
    if require_initial_pending {
        ensure!(
            commit.projection.state == "pending"
                && commit.projection.state_revision == 1
                && commit.projection.attempt_count == 0
                && commit.projection.next_attempt_at == expected.commit.not_before
                && commit.projection.claim_owner_id.is_none()
                && commit.projection.claim_token_digest.is_none()
                && commit.projection.claim_generation == 0
                && commit.projection.claim_expires_at.is_none()
                && commit.projection.last_failure_code.is_none()
                && commit.projection.updated_at == expected.commit.issued_at,
            "accepted Commit outbox initial pending readback failed"
        );
    }
    Ok(())
}
