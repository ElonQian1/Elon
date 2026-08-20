use anyhow::{ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::ComputeStartOutboxRemoteObservationEnvelope;

use super::{super::types::StoredStartOutboxOperation, CleanupSource};

pub(super) fn ensure_cleanup_operation(
    operation: &StoredStartOutboxOperation,
    source: &CleanupSource,
    operation_kind: &str,
    subject_outbox_id: &str,
) -> Result<()> {
    let envelope = &operation.envelope;
    let prepare = &source.prepare;
    ensure!(
        envelope.operation_kind == operation_kind
            && envelope.operation_generation == 1
            && envelope.subject_outbox_id.as_deref() == Some(subject_outbox_id)
            && envelope.command_id == prepare.envelope.command_id
            && envelope.command_digest == prepare.envelope.command_digest
            && operation.provider_id == prepare.provider_id
            && operation.adapter_id == prepare.adapter_id
            && envelope.adapter_binding_digest == prepare.envelope.adapter_binding_digest
            && envelope.route_authorization_id == prepare.envelope.route_authorization_id
            && envelope.route_authorization_digest == prepare.envelope.route_authorization_digest
            && envelope.actor_receipt_id == prepare.envelope.actor_receipt_id
            && envelope.actor_receipt_digest == prepare.envelope.actor_receipt_digest
            && envelope.plan_id == prepare.envelope.plan_id
            && envelope.plan_digest == prepare.envelope.plan_digest
            && envelope.lease_id == prepare.envelope.lease_id
            && envelope.fencing_generation == prepare.envelope.fencing_generation
            && envelope.application_id.is_none()
            && envelope.application_digest.is_none()
            && envelope.lease_authority_id.is_none()
            && envelope.lease_authority_revision.is_none()
            && envelope.lease_authority_digest.is_none()
            && envelope.issued_at == envelope.not_before
            && envelope.issued_at < envelope.not_after
            && envelope.not_after.as_str() <= source.cleanup_expires_at.as_str(),
        "cleanup operation failed exact source audit"
    );
    Ok(())
}

pub(super) fn ensure_durable_observation_on(
    connection: &Connection,
    observation: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM compute_attempt_start_remote_observations
              WHERE observation_id=?1 AND observation_digest=?2 AND outbox_id=?3
                AND command_id=?4 AND adapter_observation_id=?5
                AND observation_kind='prepare_response' AND response_outcome='accepted'
                AND remote_execution_state='prepared' AND terminality='non_terminal'",
            params![
                observation.observation_id,
                observation.observation_digest,
                observation.outbox_id,
                observation.command_id,
                observation.adapter_observation_id,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(
        exists,
        "quarantined cleanup lacks durable accepted observation"
    );
    Ok(())
}

pub(super) fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(super) fn next_store_time_after(value: &str) -> Result<String> {
    let floor = DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc) + Duration::nanoseconds(1);
    Ok(std::cmp::max(Utc::now(), floor).to_rfc3339_opts(SecondsFormat::Nanos, true))
}
