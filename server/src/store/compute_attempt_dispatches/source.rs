use anyhow::{bail, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        attempt_gateway::{
            ComputeAttemptAdapterBinding, ComputeAttemptDispatchCommandEnvelope,
            COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
        },
        capacity::ComputeCapacityClaimState,
        execution::{JOB_STATUS_RESERVED, RESERVATION_STATUS_ACTIVE},
        offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAINING},
        provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
    },
    store::{
        compute_broker_reservation::BrokerReserveBinding,
        compute_capacity_claim_rows::stored_claim_on,
        compute_job_registry::current_registered_job_on,
        compute_offer_registry::{current_registered_offer_on, registered_offer_version_on},
        compute_provider_registry::{
            current_registered_provider_on, ComputeProviderRegistrationReceipt,
        },
        compute_reservation_registry::current_registered_reservation_on,
    },
};

use super::read::StoredDispatchCommand;

pub(super) fn current_source_blocker_on(
    connection: &Connection,
    command: &ComputeAttemptDispatchCommandEnvelope,
    adapter: &ComputeAttemptAdapterBinding,
    activated_by_user_id: &str,
    activation_idempotency_key: &str,
    require_activation_absence: bool,
) -> Result<Option<&'static str>> {
    let start = &command.command;
    let Some(provider) = current_registered_provider_on(connection, &start.provider.provider_id)?
    else {
        return Ok(Some("PROVIDER_MISSING"));
    };
    if provider.provider.status != PROVIDER_STATUS_ACTIVE
        && provider.provider.status != PROVIDER_STATUS_DRAINING
        || provider.provider.provider_kind != adapter.provider_kind
        || provider.provider.owner_account_id != activated_by_user_id
        || !adapter_matches_current_provider(adapter, &provider)
    {
        return Ok(Some("PROVIDER_ROUTE_STALE"));
    }
    let Some(reserved_offer) =
        registered_offer_version_on(connection, &start.offer.offer_id, start.offer.offer_version)?
    else {
        return Ok(Some("OFFER_HISTORY_MISSING"));
    };
    if reserved_offer.offer.offer_digest != start.offer.offer_digest
        || reserved_offer.offer.provider_id != start.provider.provider_id
        || reserved_offer.provider_policy_revision != start.provider.policy_revision
        || reserved_offer.provider_digest != start.provider.provider_digest
    {
        return Ok(Some("OFFER_HISTORY_STALE"));
    }
    let Some(offer) = current_registered_offer_on(connection, &start.offer.offer_id)? else {
        return Ok(Some("OFFER_MISSING"));
    };
    if offer.offer.provider_id != start.provider.provider_id
        || !matches!(
            offer.offer.status.as_str(),
            OFFER_STATUS_ACTIVE | OFFER_STATUS_DRAINING
        )
    {
        return Ok(Some("OFFER_STALE"));
    }
    let Some(job) = current_registered_job_on(connection, &start.job.job_id)? else {
        return Ok(Some("JOB_MISSING"));
    };
    if job.revision != start.job.job_revision
        || job.job_digest != start.job.job_digest
        || job.job.status != JOB_STATUS_RESERVED
    {
        return Ok(Some("JOB_STALE"));
    }
    let Some(reservation) =
        current_registered_reservation_on(connection, &start.reservation.reservation_id)?
    else {
        return Ok(Some("RESERVATION_MISSING"));
    };
    if reservation.revision != start.reservation.reservation_revision
        || reservation.reservation_digest != start.reservation.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || reservation.reservation.job != start.job
        || reservation.reservation.offer != start.offer
        || reservation.reservation.capacity_claim != start.capacity_claim
    {
        return Ok(Some("RESERVATION_STALE"));
    }
    if parse_timestamp(&start.hard_deadline_at)?
        > parse_timestamp(&reservation.reservation.expires_at)?
    {
        return Ok(Some("RESERVATION_WINDOW_STALE"));
    }
    let Some(claim) = stored_claim_on(connection, &start.capacity_claim.claim_id)? else {
        return Ok(Some("CLAIM_MISSING"));
    };
    if claim.revision != start.capacity_claim.claim_revision
        || claim.claim_digest != start.capacity_claim.claim_digest
        || claim.state != ComputeCapacityClaimState::Held
    {
        return Ok(Some("CLAIM_STALE"));
    }
    if require_activation_absence
        && connection
            .query_row(
                "SELECT 1 FROM compute_attempt_activations
                  WHERE reservation_id=?1 OR job_id=?2 OR lease_id=?3
                     OR (idempotency_scope=?4 AND idempotency_key=?5)
                  LIMIT 1",
                params![
                    start.identity.reservation_id,
                    start.identity.job_id,
                    start.identity.attempt_lease_id,
                    format!("compute_attempt_activation:{}", start.provider.provider_id),
                    activation_idempotency_key,
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
    {
        return Ok(Some("ATTEMPT_ALREADY_ACTIVATED"));
    }
    Ok(None)
}

pub(super) fn ack_received_after_deadline(
    ack: &crate::compute_federation::attempt_gateway::ComputeAttemptAdapterAckEnvelope,
    deadline: &str,
    ingested_at: &str,
) -> Result<bool> {
    let deadline = parse_timestamp(deadline)?;
    Ok(parse_timestamp(&ack.received_at)? > deadline || parse_timestamp(ingested_at)? > deadline)
}

pub(super) fn ensure_command_live_at(
    command: &ComputeAttemptDispatchCommandEnvelope,
    created_at: &str,
) -> Result<()> {
    let issued_at = parse_timestamp(&command.issued_at)?;
    let created_at = parse_timestamp(created_at)?;
    let not_after = parse_timestamp(&command.not_after)?;
    if issued_at > created_at || created_at >= not_after {
        bail!("Attempt Start dispatch command is not live at durable creation");
    }
    Ok(())
}

pub(super) fn current_budget_blocker_on(
    connection: &Connection,
    command: &StoredDispatchCommand,
    checked_at: &str,
) -> Result<Option<&'static str>> {
    let start = &command.command.command;
    let current_expiry = connection
        .query_row(
            "SELECT br.expires_at
               FROM compute_broker_reserve_receipts b
               JOIN billing_reservations br ON br.id=b.budget_reservation_id
              WHERE b.reservation_id=?1 AND b.request_digest=?2
                AND b.budget_reservation_id=?3 AND b.budget_reserved_fen=?4
                AND b.capacity_claim_id=?5 AND b.capacity_claim_revision=?6
                AND b.capacity_claim_digest=?7
                AND b.job_id=?8 AND b.reserved_job_revision=?9
                AND b.reserved_job_digest=?10
                AND b.reservation_revision=?11 AND b.reservation_digest=?12
                AND br.user_id=b.consumer_account_id
                AND br.reserved_fen=?4 AND br.status='reserved'",
            params![
                start.identity.reservation_id,
                command.broker_request_digest,
                command.budget_reservation_id,
                command.budget_reserved_fen,
                start.capacity_claim.claim_id,
                start.capacity_claim.claim_revision,
                start.capacity_claim.claim_digest,
                start.identity.job_id,
                start.job.job_revision,
                start.job.job_digest,
                start.reservation.reservation_revision,
                start.reservation.reservation_digest,
            ],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    let Some(expires_at) = current_expiry else {
        return Ok(Some("BUDGET_RESERVATION_STALE"));
    };
    if let Some(expires_at) = expires_at {
        if parse_timestamp(&expires_at)? < parse_timestamp(checked_at)? {
            return Ok(Some("BUDGET_RESERVATION_STALE"));
        }
    }
    Ok(None)
}

pub(super) fn ensure_broker_matches_command(
    broker: &BrokerReserveBinding,
    command: &ComputeAttemptDispatchCommandEnvelope,
) -> Result<()> {
    let start = &command.command;
    if broker.capacity_claim != start.capacity_claim
        || broker.reserved_job != start.job
        || broker.reservation_revision != start.reservation.reservation_revision
        || broker.reservation_digest != start.reservation.reservation_digest
    {
        bail!("Attempt dispatch does not bind the exact Broker reservation receipt");
    }
    Ok(())
}

fn adapter_matches_current_provider(
    adapter: &ComputeAttemptAdapterBinding,
    provider: &ComputeProviderRegistrationReceipt,
) -> bool {
    match adapter.route_kind.as_str() {
        COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT => {
            provider.provider.endpoint.as_ref().is_some_and(|endpoint| {
                adapter.endpoint_id.as_deref() == Some(endpoint.endpoint_id.as_str())
                    && adapter.endpoint_transport.as_deref() == Some(endpoint.transport.as_str())
            })
        }
        COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER => {
            provider.provider.adapter.as_ref().is_some_and(|current| {
                current.adapter_id == adapter.adapter_id
                    && current.adapter_version == adapter.adapter_version
                    && current.config_revision == adapter.config_revision
                    && current.config_digest == adapter.config_digest
            })
        }
        _ => false,
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).map_err(Into::into)
}
