use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::attempt_gateway::{
    canonical_adapter_ack_json_and_digest, canonical_adapter_binding_json_and_digest,
    canonical_dispatch_command_json_and_digest, ComputeAttemptAdapterAckEnvelope,
    ComputeAttemptAdapterBinding, ComputeAttemptDispatchCommandEnvelope,
    ValidatedComputeAttemptStartDispatch,
};

use super::{
    types::{
        ComputeAttemptDispatchAckReceipt, ComputeAttemptDispatchApplicationReceipt,
        ComputeAttemptDispatchCommandReceipt,
    },
    validation::{prepare_application, PreparedStartDispatch, PreparedVerifiedAck},
};

pub(super) struct StoredDispatchCommand {
    pub command: ComputeAttemptDispatchCommandEnvelope,
    pub adapter: ComputeAttemptAdapterBinding,
    pub adapter_binding_digest: String,
    pub lease_credential_ref: String,
    pub lease_credential_hint: String,
    pub activation_idempotency_key: String,
    pub activated_by_user_id: String,
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
    pub broker_request_digest: String,
    pub created_at: String,
}

pub(super) struct StoredDispatchAck {
    pub ack: ComputeAttemptAdapterAckEnvelope,
    pub provider_id: String,
    pub adapter_id: String,
    pub disposition: String,
    pub disposition_reason_code: Option<String>,
    pub activation_lease_id: Option<String>,
    pub application_id: Option<String>,
}

pub(super) struct StoredDispatchApplication {
    receipt: ComputeAttemptDispatchApplicationReceipt,
    application_json: String,
}

impl StoredDispatchApplication {
    pub(super) fn into_receipt(
        mut self,
        replayed: bool,
    ) -> ComputeAttemptDispatchApplicationReceipt {
        self.receipt.replayed = replayed;
        self.receipt
    }
}

pub(super) fn command_by_id_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StoredDispatchCommand>> {
    stored_command_on(connection, "WHERE command_id=?1", params![command_id])
}

pub(super) fn command_by_idempotency_on(
    connection: &Connection,
    provider_id: &str,
    idempotency_key: &str,
) -> Result<Option<StoredDispatchCommand>> {
    stored_command_on(
        connection,
        "WHERE provider_id=?1 AND activation_idempotency_key=?2",
        params![provider_id, idempotency_key],
    )
}

fn stored_command_on<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredDispatchCommand>> {
    let sql = format!(
        "SELECT command_json, command_digest, adapter_binding_json,
                adapter_binding_digest, lease_credential_ref, lease_credential_hint,
                activation_idempotency_key, activated_by_user_id,
                budget_reservation_id, budget_reserved_fen, broker_request_digest, created_at
           FROM compute_attempt_dispatch_commands {predicate}"
    );
    let stored = connection
        .query_row(&sql, parameters, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })
        .optional()?;
    let Some((
        command_json,
        command_digest,
        adapter_json,
        adapter_binding_digest,
        lease_credential_ref,
        lease_credential_hint,
        activation_idempotency_key,
        activated_by_user_id,
        budget_reservation_id,
        budget_reserved_fen,
        broker_request_digest,
        created_at,
    )) = stored
    else {
        return Ok(None);
    };
    let command: ComputeAttemptDispatchCommandEnvelope = serde_json::from_str(&command_json)?;
    let adapter: ComputeAttemptAdapterBinding = serde_json::from_str(&adapter_json)?;
    let (canonical_command, recomputed_command_digest) =
        canonical_dispatch_command_json_and_digest(&command)?;
    let (canonical_adapter, recomputed_adapter_digest) =
        canonical_adapter_binding_json_and_digest(&adapter)?;
    if canonical_command != command_json
        || command.command_digest != command_digest
        || recomputed_command_digest != command_digest
        || canonical_adapter != adapter_json
        || recomputed_adapter_digest != adapter_binding_digest
        || command.command.provider.provider_id != adapter.provider_id
    {
        bail!("Stored Attempt dispatch command or Adapter binding failed exact audit");
    }
    Ok(Some(StoredDispatchCommand {
        command,
        adapter,
        adapter_binding_digest,
        lease_credential_ref,
        lease_credential_hint,
        activation_idempotency_key,
        activated_by_user_id,
        budget_reservation_id,
        budget_reserved_fen,
        broker_request_digest,
        created_at,
    }))
}

pub(super) fn ensure_command_replay_matches(
    stored: &StoredDispatchCommand,
    plan: &ValidatedComputeAttemptStartDispatch,
    prepared: &PreparedStartDispatch,
) -> Result<()> {
    if &stored.command != plan.command()
        || &stored.adapter != plan.adapter()
        || stored.adapter_binding_digest != prepared.adapter_digest
        || stored.lease_credential_ref != plan.activation().lease_credential_ref()
        || stored.lease_credential_hint != plan.activation().lease_credential_hint()
        || stored.activation_idempotency_key != plan.activation().idempotency_key()
        || stored.activated_by_user_id != plan.activation().activated_by_user_id()
    {
        bail!("Attempt dispatch command replay conflicts with the immutable stored intent");
    }
    Ok(())
}

pub(super) fn command_receipt(
    stored: StoredDispatchCommand,
    replayed: bool,
) -> ComputeAttemptDispatchCommandReceipt {
    ComputeAttemptDispatchCommandReceipt {
        command: stored.command,
        adapter: stored.adapter,
        adapter_binding_digest: stored.adapter_binding_digest,
        created_at: stored.created_at,
        replayed,
    }
}

pub(super) fn ack_by_command_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StoredDispatchAck>> {
    stored_ack_on(connection, "WHERE command_id=?1", params![command_id])
}

pub(super) fn ack_by_adapter_ack_on(
    connection: &Connection,
    provider_id: &str,
    adapter_id: &str,
    adapter_ack_id: &str,
) -> Result<Option<StoredDispatchAck>> {
    stored_ack_on(
        connection,
        "WHERE provider_id=?1 AND adapter_id=?2 AND adapter_ack_id=?3",
        params![provider_id, adapter_id, adapter_ack_id],
    )
}

fn stored_ack_on<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredDispatchAck>> {
    let sql = format!(
        "SELECT ack_json, ack_digest, provider_id, adapter_id,
                disposition, disposition_reason_code, activation_lease_id, application_id
           FROM compute_attempt_dispatch_acks {predicate}"
    );
    let stored = connection
        .query_row(&sql, parameters, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .optional()?;
    let Some((
        ack_json,
        ack_digest,
        provider_id,
        adapter_id,
        disposition,
        disposition_reason_code,
        activation_lease_id,
        application_id,
    )) = stored
    else {
        return Ok(None);
    };
    let ack: ComputeAttemptAdapterAckEnvelope = serde_json::from_str(&ack_json)?;
    let (canonical, recomputed) = canonical_adapter_ack_json_and_digest(&ack)?;
    let disposition_valid = match disposition.as_str() {
        "rejected" => {
            ack.outcome == "rejected"
                && disposition_reason_code.is_none()
                && activation_lease_id.is_none()
                && application_id.is_none()
        }
        "quarantined" => {
            ack.outcome == "accepted"
                && disposition_reason_code
                    .as_ref()
                    .is_some_and(|value| !value.is_empty())
                && activation_lease_id.is_none()
                && application_id.is_none()
        }
        "accepted_applied" => {
            ack.outcome == "accepted"
                && disposition_reason_code.is_none()
                && activation_lease_id
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
                && application_id
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
        }
        _ => false,
    };
    if canonical != ack_json
        || ack.ack_digest != ack_digest
        || recomputed != ack_digest
        || !disposition_valid
    {
        bail!("Stored Adapter ACK failed exact canonical audit");
    }
    Ok(Some(StoredDispatchAck {
        ack,
        provider_id,
        adapter_id,
        disposition,
        disposition_reason_code,
        activation_lease_id,
        application_id,
    }))
}

pub(super) fn ensure_remote_ack_replay_matches(
    stored: &StoredDispatchAck,
    verified: &crate::compute_federation::attempt_gateway::VerifiedComputeAttemptAdapterAck,
    prepared: &PreparedVerifiedAck,
) -> Result<()> {
    let incoming = verified.ack();
    let first = &stored.ack;
    if first.schema != incoming.schema
        || first.adapter_ack_id != incoming.adapter_ack_id
        || first.command_id != incoming.command_id
        || first.command_digest != incoming.command_digest
        || first.adapter_binding_digest != incoming.adapter_binding_digest
        || first.outcome != incoming.outcome
        || first.remote_execution_ref != incoming.remote_execution_ref
        || first.reason_code != incoming.reason_code
        || first.observed_at != incoming.observed_at
        || stored.provider_id != verified.adapter().provider_id
        || stored.adapter_id != verified.adapter().adapter_id
        || first.adapter_binding_digest != prepared.adapter_digest
    {
        bail!("Remote Adapter ACK replay conflicts with its first durable receipt");
    }
    Ok(())
}

pub(super) fn ensure_ack_replay_matches(
    stored: &StoredDispatchAck,
    verified: &crate::compute_federation::attempt_gateway::VerifiedComputeAttemptAdapterAck,
    prepared: &PreparedVerifiedAck,
) -> Result<()> {
    if &stored.ack != verified.ack()
        || stored.provider_id != verified.adapter().provider_id
        || stored.adapter_id != verified.adapter().adapter_id
        || stored.ack.ack_digest != prepared.ack_digest
        || stored.ack.adapter_binding_digest != prepared.adapter_digest
    {
        bail!("Adapter ACK replay conflicts with the immutable stored ACK");
    }
    Ok(())
}

pub(super) fn ack_receipt(
    stored: StoredDispatchAck,
    replayed: bool,
) -> ComputeAttemptDispatchAckReceipt {
    ComputeAttemptDispatchAckReceipt {
        ack: stored.ack,
        disposition: stored.disposition,
        disposition_reason_code: stored.disposition_reason_code,
        replayed,
    }
}

pub(super) fn application_by_command_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StoredDispatchApplication>> {
    connection
        .query_row(
            "SELECT application_id, application_digest, command_id, ack_id, action,
                    lease_id, activation_request_digest, lease_digest, applied_at,
                    application_json
               FROM compute_attempt_dispatch_applications WHERE command_id=?1",
            params![command_id],
            |row| {
                Ok(StoredDispatchApplication {
                    receipt: ComputeAttemptDispatchApplicationReceipt {
                        application_id: row.get(0)?,
                        application_digest: row.get(1)?,
                        command_id: row.get(2)?,
                        ack_id: row.get(3)?,
                        action: row.get(4)?,
                        lease_id: row.get(5)?,
                        activation_request_digest: row.get(6)?,
                        lease_digest: row.get(7)?,
                        applied_at: row.get(8)?,
                        schema: crate::compute_federation::attempt_gateway::COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA.to_string(),
                        replayed: false,
                    },
                    application_json: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub(super) fn ensure_application_matches(
    stored: &StoredDispatchApplication,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &crate::store::ComputeAttemptActivationReceipt,
) -> Result<()> {
    let expected = prepare_application(ack, activation)?;
    let receipt = &stored.receipt;
    if receipt.schema
        != crate::compute_federation::attempt_gateway::COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA
        || receipt.application_id != expected.application_id
        || receipt.application_digest != expected.application_digest
        || stored.application_json != expected.application_json
        || receipt.command_id != ack.command_id
        || receipt.ack_id != ack.ack_id
        || receipt.action != "v185_activate"
        || receipt.lease_id != activation.lease.lease_id
        || receipt.activation_request_digest != activation.request_digest
        || receipt.lease_digest != activation.lease_digest
        || receipt.applied_at != activation.activated_at
    {
        bail!("Stored Attempt dispatch application failed exact canonical audit");
    }
    Ok(())
}

pub(super) fn require_application_for_accepted_replay(
    connection: &Connection,
    command_id: &str,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &crate::store::ComputeAttemptActivationReceipt,
) -> Result<ComputeAttemptDispatchApplicationReceipt> {
    let stored = application_by_command_on(connection, command_id)?.ok_or_else(|| {
        anyhow!("Accepted Adapter ACK is missing its atomic activation application")
    })?;
    ensure_application_matches(&stored, ack, activation)?;
    Ok(stored.into_receipt(true))
}
