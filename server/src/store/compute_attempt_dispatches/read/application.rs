use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::attempt_gateway::{
    canonical_dispatch_application_json_and_digest, ComputeAttemptAdapterAckEnvelope,
    ComputeAttemptDispatchApplicationEnvelope,
    COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE,
    COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA,
};

use super::{
    super::{types::ComputeAttemptDispatchApplicationReceipt, validation::prepare_application},
    StoredDispatchApplication,
};

pub(super) fn application_by_command_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StoredDispatchApplication>> {
    let stored = connection
        .query_row(
            "SELECT application_id, application_digest, command_id, ack_id, action,
                    lease_id, activation_request_digest, lease_digest, applied_at,
                    application_json
               FROM compute_attempt_dispatch_applications WHERE command_id=?1",
            params![command_id],
            |row| {
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
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()?;
    let Some((
        application_id,
        application_digest,
        command_id,
        ack_id,
        action,
        lease_id,
        activation_request_digest,
        lease_digest,
        applied_at,
        application_json,
    )) = stored
    else {
        return Ok(None);
    };
    let envelope: ComputeAttemptDispatchApplicationEnvelope =
        serde_json::from_str(&application_json)?;
    let (canonical_json, recomputed_digest) =
        canonical_dispatch_application_json_and_digest(&envelope)?;
    if canonical_json != application_json
        || recomputed_digest != application_digest
        || envelope.schema != COMPUTE_ATTEMPT_DISPATCH_APPLICATION_SCHEMA
        || envelope.application_id != application_id
        || envelope.application_digest != application_digest
        || envelope.command_id != command_id
        || envelope.ack_id != ack_id
        || envelope.action != action
        || envelope.action != COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE
        || envelope.lease_id != lease_id
        || envelope.activation_request_digest != activation_request_digest
        || envelope.lease_digest != lease_digest
        || envelope.applied_at != applied_at
    {
        bail!("Stored Attempt dispatch application failed exact canonical audit");
    }
    Ok(Some(StoredDispatchApplication {
        envelope,
        application_json,
    }))
}

pub(super) fn ensure_application_matches(
    stored: &StoredDispatchApplication,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &crate::store::ComputeAttemptActivationReceipt,
) -> Result<()> {
    let expected = prepare_application(ack, activation)?;
    if &stored.envelope != expected.envelope()
        || stored.application_json != expected.application_json
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
