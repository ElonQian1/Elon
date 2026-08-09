use anyhow::{bail, ensure, Result};
use rusqlite::{params, Connection};

use crate::store::compute_attempt_dispatches::PreparedApplication;

use super::source::load_source_for_persist_on;
use super::{
    derive::derive_closure, readback, AcceptedStartCommitClosureReceipt,
    AcceptedStartCommitFreshness, DerivedAcceptedCommitClosure,
};

pub(super) fn persist_on(
    connection: &Connection,
    command_id: &str,
    application: &PreparedApplication,
    closure_at: &str,
) -> Result<AcceptedStartCommitClosureReceipt> {
    if super::super::read::operation_by_command_kind_on(connection, command_id, "commit")?.is_some()
    {
        let replayed = readback::audit_on(connection, command_id, true)?;
        let application_digest = &application.application_digest;
        ensure!(
            replayed.application_actor_receipt_id
                == format!("attempt_application_actor_{application_digest}")
                && replayed.lease_authority_id
                    == format!("attempt_lease_authority_{application_digest}")
                && replayed.commit_outbox_id
                    == format!("attempt_start_commit_{application_digest}"),
            "accepted closure replay conflicts with prepared application identity"
        );
        return Ok(replayed);
    }
    match super::currentness::ensure_fresh_on(connection, command_id, closure_at)? {
        AcceptedStartCommitFreshness::Current => {}
        AcceptedStartCommitFreshness::Quarantine { reason_code } => {
            bail!("accepted closure lost fresh currentness: {reason_code}")
        }
    }
    let source = load_source_for_persist_on(connection, command_id, application)?;
    let expected = derive_closure(&source, closure_at)?;
    persist_actor_on(connection, &expected)?;
    persist_authority_on(connection, &expected)?;
    persist_commit_on(connection, &expected, closure_at)?;
    readback::audit_expected_rows_on(connection, &expected)?;
    Ok(expected.receipt(false))
}

fn persist_actor_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
) -> Result<()> {
    let actor = &expected.actor;
    let changed = connection.execute(
        "INSERT INTO compute_attempt_dispatch_actor_receipts (
            actor_receipt_id, actor_receipt_schema, actor_receipt_digest,
            actor_receipt_json, canonicalization, digest_algorithm, actor_phase,
            command_id, command_digest, provider_id, provider_owner_account_id,
            service_actor_id, actor_authorization_id, actor_authorization_digest,
            route_authorization_id, route_authorization_digest, ack_id, ack_digest,
            application_id, application_digest, issued_at, valid_until, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,
            ?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23
         )",
        params![
            actor.actor_receipt_id,
            actor.schema,
            actor.actor_receipt_digest,
            expected.actor_json,
            actor.canonicalization,
            actor.digest_algorithm,
            actor.actor_phase,
            actor.command_id,
            actor.command_digest,
            actor.provider_id,
            actor.provider_owner_account_id,
            actor.service_actor_id,
            actor.actor_authorization_id,
            actor.actor_authorization_digest,
            actor.route_authorization_id,
            actor.route_authorization_digest,
            actor.ack_id,
            actor.ack_digest,
            actor.application_id,
            actor.application_digest,
            actor.issued_at,
            actor.valid_until,
            actor.recorded_at,
        ],
    )?;
    ensure!(
        changed == 1,
        "accepted application actor insert was not exact"
    );
    Ok(())
}

fn persist_authority_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
) -> Result<()> {
    let authority = &expected.authority;
    let changed = connection.execute(
        "INSERT INTO compute_attempt_lease_authority_bindings (
            lease_authority_id, authority_revision, authority_schema,
            lease_authority_digest, authority_json, canonicalization, digest_algorithm,
            non_bearer_authority_ref, authority_hint, command_id, command_digest,
            plan_id, plan_digest, ack_id, ack_digest, application_id, application_digest,
            application_actor_receipt_id, application_actor_receipt_digest,
            lease_id, lease_digest, provider_id, executor_id, fencing_generation,
            route_authorization_id, route_authorization_digest, authority_kind,
            delivery_mode, audience, scopes_json, scope_count, scopes_digest,
            issued_at, expires_at, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,
            ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,
            ?33,?34,?35
         )",
        params![
            authority.lease_authority_id,
            authority.authority_revision,
            authority.schema,
            authority.lease_authority_digest,
            expected.authority_json,
            authority.canonicalization,
            authority.digest_algorithm,
            authority.non_bearer_authority_ref,
            authority.authority_hint,
            authority.command_id,
            authority.command_digest,
            authority.plan_id,
            authority.plan_digest,
            authority.ack_id,
            authority.ack_digest,
            authority.application_id,
            authority.application_digest,
            authority.application_actor_receipt_id,
            authority.application_actor_receipt_digest,
            authority.lease_id,
            authority.lease_digest,
            authority.provider_id,
            authority.executor_id,
            authority.fencing_generation,
            authority.route_authorization_id,
            authority.route_authorization_digest,
            authority.authority_kind,
            authority.delivery_mode,
            authority.audience,
            serde_json::to_string(&authority.scopes)?,
            i64::try_from(authority.scopes.len())?,
            authority.scopes_digest,
            authority.issued_at,
            authority.expires_at,
            authority.recorded_at,
        ],
    )?;
    ensure!(
        changed == 1,
        "accepted lease authority insert was not exact"
    );
    Ok(())
}

fn persist_commit_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
    created_at: &str,
) -> Result<()> {
    let commit = &expected.commit;
    let changed = connection.execute(
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
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
            ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,
            ?31,?32,'pending',1,0,?31,NULL,NULL,0,NULL,NULL,?33,?33
         )",
        params![
            commit.outbox_id,
            commit.schema,
            commit.outbox_digest,
            expected.commit_json,
            commit.canonicalization,
            commit.digest_algorithm,
            commit.operation_kind,
            commit.operation_generation,
            commit.subject_outbox_id,
            commit.command_id,
            commit.command_digest,
            expected.provider_id,
            expected.adapter_id,
            commit.adapter_binding_digest,
            commit.route_authorization_id,
            commit.route_authorization_digest,
            commit.actor_receipt_id,
            commit.actor_receipt_digest,
            commit.plan_id,
            commit.plan_digest,
            commit.lease_id,
            commit.fencing_generation,
            commit.ack_id,
            commit.ack_digest,
            commit.application_id,
            commit.application_digest,
            commit.lease_authority_id,
            commit.lease_authority_revision,
            commit.lease_authority_digest,
            commit.issued_at,
            commit.not_before,
            commit.not_after,
            created_at,
        ],
    )?;
    ensure!(changed == 1, "accepted Commit outbox insert was not exact");
    Ok(())
}
