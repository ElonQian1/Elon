use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::store::{
    compute_attempt_activations::compute_attempt_activation_on,
    compute_attempt_start_outbox::audit_accepted_start_commit_closure_on,
    ComputeAttemptActivationReceipt,
};

use super::{
    read::{
        ack_receipt, require_application_for_accepted_replay, StoredDispatchAck,
        StoredDispatchCommand,
    },
    types::ComputeAttemptDispatchAckCommit,
};

pub(super) fn replay_ack_commit(
    connection: &Connection,
    command: &StoredDispatchCommand,
    stored: StoredDispatchAck,
) -> Result<ComputeAttemptDispatchAckCommit> {
    match stored.disposition.as_str() {
        "rejected" => Ok(ComputeAttemptDispatchAckCommit::Rejected {
            ack: ack_receipt(stored, true),
        }),
        "quarantined" => Ok(ComputeAttemptDispatchAckCommit::Quarantined {
            ack: ack_receipt(stored, true),
        }),
        "accepted_applied" => {
            let start = &command.command.command;
            let activation_lease_id = stored
                .activation_lease_id
                .as_deref()
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its activation lease"))?;
            if activation_lease_id != start.identity.attempt_lease_id {
                bail!("Accepted Adapter ACK conflicts with its immutable command lease");
            }
            let stored_ack = stored.ack.clone();
            let expected_application_id = stored
                .application_id
                .clone()
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its application ID"))?;
            let mut activation = compute_attempt_activation_on(connection, activation_lease_id)?;
            ensure_activation_matches_command(command, &stored_ack, &activation)?;
            let application = require_application_for_accepted_replay(
                connection,
                &stored_ack.command_id,
                &stored_ack,
                &activation,
            )?;
            if application.application_id != expected_application_id {
                bail!("Accepted Adapter ACK application ID conflicts with its application");
            }
            let accepted_closure =
                audit_accepted_start_commit_closure_on(connection, &stored_ack.command_id)?;
            activation.replayed = true;
            activation.capacity_ledger.replayed = true;
            Ok(ComputeAttemptDispatchAckCommit::Activated {
                ack: ack_receipt(stored, true),
                application,
                accepted_closure,
                activation,
            })
        }
        _ => bail!("Stored Adapter ACK has an unknown disposition"),
    }
}

pub(super) fn ensure_activation_matches_command(
    command: &StoredDispatchCommand,
    ack: &crate::compute_federation::attempt_gateway::ComputeAttemptAdapterAckEnvelope,
    activation: &ComputeAttemptActivationReceipt,
) -> Result<()> {
    let start = &command.command.command;
    let lease = &activation.lease;
    if ack.remote_execution_ref.as_deref() != Some(activation.executor_acceptance_ref.as_str())
        || lease.lease_id != start.identity.attempt_lease_id
        || lease.job_id != start.identity.job_id
        || lease.reservation_id != start.identity.reservation_id
        || lease.provider_id != start.provider.provider_id
        || lease.executor_id != start.executor_id
        || lease.attempt_no != start.identity.attempt_no
        || lease.shard_id != start.identity.shard_id
        || lease.fencing_generation != start.identity.fencing_generation
        || lease.lease_credential_ref != command.lease_credential_ref
        || lease.lease_credential_hint != command.lease_credential_hint
        || lease.expires_at != start.lease_expires_at
        || lease.hard_deadline_at != start.hard_deadline_at
        || activation.source_job != start.job
        || activation.source_reservation_revision != start.reservation.reservation_revision
        || activation.source_reservation_digest != start.reservation.reservation_digest
        || activation.source_claim != start.capacity_claim
        || activation.budget_reservation_id != command.budget_reservation_id
        || activation.budget_reserved_fen != command.budget_reserved_fen
        || activation.activated_by_user_id != command.activated_by_user_id
    {
        bail!("Attempt activation lineage conflicts with its immutable dispatch command");
    }
    Ok(())
}
