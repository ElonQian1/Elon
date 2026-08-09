use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        attempt_gateway::{
            COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
        },
        capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState},
        execution::{
            ComputeJobVersionBinding, ComputeOfferBinding, JOB_STATUS_RESERVED,
            RESERVATION_STATUS_ACTIVE,
        },
        execution_plan::{
            ComputeAttemptExecutionPlanEnvelope, ComputeAttemptExecutionSourceBindings,
            ComputeExecutionBudgetReservationBinding, ComputeExecutionCapabilityBinding,
            ComputeExecutionCapabilityEnvelope, ComputeExecutionPriceSnapshotBinding,
            ComputeExecutionProviderVersionBinding, ComputeExecutionReservationVersionBinding,
            EXECUTION_CAPABILITY_ADAPTER_EXECUTION, EXECUTION_CAPABILITY_NODE_READY,
            EXECUTION_CAPABILITY_PROVIDER_ENDPOINT,
        },
        offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAINING},
        provider::{
            PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_KIND_MANAGED_CLUSTER, PROVIDER_KIND_USER_NODE,
            PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING,
        },
    },
    store::{
        compute_broker_reservation::broker_reserve_binding_on,
        compute_capacity_claim_rows::stored_claim_on,
        compute_job_registry::current_registered_job_on,
        compute_offer_registry::{current_registered_offer_on, registered_offer_version_on},
        compute_provider_registry::{
            current_registered_provider_on, registered_provider_version_on,
        },
        compute_reservation_registry::current_registered_reservation_on,
    },
};

use super::types::{CurrentExecutionSources, PreparedInputs};

pub(super) fn derive_source_bindings(
    sources: &CurrentExecutionSources,
) -> ComputeAttemptExecutionSourceBindings {
    let provider = &sources.historical_provider;
    let offer = &sources.historical_offer;
    let job = &sources.job;
    let reservation = &sources.reservation;
    ComputeAttemptExecutionSourceBindings {
        consumer_account_id: job.job.consumer_account_id.clone(),
        provider: ComputeExecutionProviderVersionBinding {
            provider_id: provider.provider.provider_id.clone(),
            provider_kind: provider.provider.provider_kind.clone(),
            provider_owner_account_id: provider.provider.owner_account_id.clone(),
            policy_revision: provider.provider.policy_revision,
            provider_digest: provider.provider_digest.clone(),
        },
        offer: ComputeOfferBinding {
            provider_id: offer.offer.provider_id.clone(),
            offer_id: offer.offer.offer_id.clone(),
            offer_version: offer.offer.offer_version,
            offer_digest: offer.offer.offer_digest.clone(),
        },
        job: ComputeJobVersionBinding {
            job_id: job.job.job_id.clone(),
            job_revision: job.revision,
            job_digest: job.job_digest.clone(),
        },
        reservation: ComputeExecutionReservationVersionBinding {
            reservation_id: reservation.reservation.reservation_id.clone(),
            reservation_revision: reservation.revision,
            reservation_digest: reservation.reservation_digest.clone(),
        },
        capacity_claim: reservation.reservation.capacity_claim.clone(),
        price_snapshot: ComputeExecutionPriceSnapshotBinding {
            price_snapshot_id: reservation.reservation.price_snapshot.snapshot_id.clone(),
            price_snapshot_digest: reservation
                .reservation
                .price_snapshot
                .snapshot_digest
                .clone(),
        },
        budget: ComputeExecutionBudgetReservationBinding {
            budget_reservation_id: sources.broker.budget_reservation_id.clone(),
            budget_reserved_fen: sources.broker.budget_reserved_fen,
        },
        broker_request_digest: sources.broker_request_digest.clone(),
    }
}

pub(super) fn derive_capability_binding(
    inputs: &PreparedInputs,
) -> ComputeExecutionCapabilityBinding {
    let envelope = &inputs.capability.envelope;
    ComputeExecutionCapabilityBinding {
        capability_id: envelope.capability_id.clone(),
        capability_digest: envelope.capability_digest.clone(),
        capability_kind: envelope.capability.capability_kind.clone(),
        provider_id: envelope.capability.provider_id.clone(),
        executor_id: envelope.capability.executor_id.clone(),
        expires_at: envelope.capability.expires_at.clone(),
    }
}

pub(super) fn current_execution_sources_on(
    connection: &Connection,
    plan: &ComputeAttemptExecutionPlanEnvelope,
    capability: &ComputeExecutionCapabilityEnvelope,
) -> Result<CurrentExecutionSources> {
    let candidate = &plan.plan;
    let source = &candidate.sources;

    let historical_provider = registered_provider_version_on(
        connection,
        &source.provider.provider_id,
        source.provider.policy_revision,
    )?
    .ok_or_else(|| anyhow!("Execution plan Provider history is missing"))?;
    if historical_provider.provider_digest != source.provider.provider_digest
        || historical_provider.provider.provider_kind != source.provider.provider_kind
        || historical_provider.provider.owner_account_id
            != source.provider.provider_owner_account_id
    {
        bail!("Execution plan Provider history binding is stale");
    }

    let current_provider =
        current_registered_provider_on(connection, &source.provider.provider_id)?
            .ok_or_else(|| anyhow!("Execution plan current Provider is missing"))?;
    if !matches!(
        current_provider.provider.status.as_str(),
        PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING
    ) || current_provider.provider.provider_kind != source.provider.provider_kind
        || current_provider.provider.owner_account_id != source.provider.provider_owner_account_id
    {
        bail!("Execution plan current Provider status or owner binding is stale");
    }
    ensure_current_route(&current_provider.provider, capability)?;

    let historical_offer = registered_offer_version_on(
        connection,
        &source.offer.offer_id,
        source.offer.offer_version,
    )?
    .ok_or_else(|| anyhow!("Execution plan Offer history is missing"))?;
    if historical_offer.offer.offer_digest != source.offer.offer_digest
        || historical_offer.offer.provider_id != source.provider.provider_id
        || historical_offer.offer.provider_kind != source.provider.provider_kind
        || historical_offer.provider_policy_revision != source.provider.policy_revision
        || historical_offer.provider_digest != source.provider.provider_digest
    {
        bail!("Execution plan Offer history binding is stale");
    }
    let current_offer = current_registered_offer_on(connection, &source.offer.offer_id)?
        .ok_or_else(|| anyhow!("Execution plan current Offer is missing"))?;
    if current_offer.offer.provider_id != source.provider.provider_id
        || current_offer.offer.provider_kind != source.provider.provider_kind
        || !matches!(
            current_offer.offer.status.as_str(),
            OFFER_STATUS_ACTIVE | OFFER_STATUS_DRAINING
        )
    {
        bail!("Execution plan current Offer is not eligible for reserved work");
    }

    let job = current_registered_job_on(connection, &source.job.job_id)?
        .ok_or_else(|| anyhow!("Execution plan current Job is missing"))?;
    if job.revision != source.job.job_revision
        || job.job_digest != source.job.job_digest
        || job.job.status != JOB_STATUS_RESERVED
        || job.job.consumer_account_id != source.consumer_account_id
        || job.job.selected_offer.as_ref() != Some(&source.offer)
    {
        bail!("Execution plan current Job binding is stale");
    }

    let reservation =
        current_registered_reservation_on(connection, &source.reservation.reservation_id)?
            .ok_or_else(|| anyhow!("Execution plan current Reservation is missing"))?;
    if reservation.revision != source.reservation.reservation_revision
        || reservation.reservation_digest != source.reservation.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || reservation.reservation.job != source.job
        || reservation.reservation.offer != source.offer
        || reservation.reservation.capacity_claim != source.capacity_claim
        || reservation.reservation.price_snapshot.snapshot_id
            != source.price_snapshot.price_snapshot_id
        || reservation.reservation.price_snapshot.snapshot_digest
            != source.price_snapshot.price_snapshot_digest
    {
        bail!("Execution plan current Reservation binding is stale");
    }

    let claim = stored_claim_on(connection, &source.capacity_claim.claim_id)?
        .ok_or_else(|| anyhow!("Execution plan Capacity Claim is missing"))?;
    if claim.revision != source.capacity_claim.claim_revision
        || claim.claim_digest != source.capacity_claim.claim_digest
        || claim.state != ComputeCapacityClaimState::Held
        || claim.claim_kind != ComputeCapacityClaimKind::Reservation
        || claim.subject_kind != "compute_reservation"
        || claim.subject_id != source.reservation.reservation_id
    {
        bail!("Execution plan Capacity Claim binding is stale");
    }

    let broker = broker_reserve_binding_on(
        connection,
        &source.reservation.reservation_id,
        &source.consumer_account_id,
    )?;
    if broker.budget_reservation_id != source.budget.budget_reservation_id
        || broker.budget_reserved_fen != source.budget.budget_reserved_fen
        || broker.capacity_claim != source.capacity_claim
        || broker.reserved_job != source.job
        || broker.reservation_revision != source.reservation.reservation_revision
        || broker.reservation_digest != source.reservation.reservation_digest
    {
        bail!("Execution plan Broker reservation binding is stale");
    }
    let broker_request_digest = connection.query_row(
        "SELECT request_digest FROM compute_broker_reserve_receipts
          WHERE reservation_id=?1 AND consumer_account_id=?2
            AND budget_reservation_id=?3 AND capacity_claim_id=?4
            AND job_id=?5 AND reservation_revision=?6 AND reservation_digest=?7",
        params![
            source.reservation.reservation_id,
            source.consumer_account_id,
            source.budget.budget_reservation_id,
            source.capacity_claim.claim_id,
            source.job.job_id,
            source.reservation.reservation_revision,
            source.reservation.reservation_digest,
        ],
        |row| row.get::<_, String>(0),
    )?;
    if broker_request_digest != source.broker_request_digest {
        bail!("Execution plan Broker request digest is stale");
    }

    let budget_expires_at = connection
        .query_row(
            "SELECT expires_at FROM billing_reservations
              WHERE id=?1 AND user_id=?2 AND reserved_fen=?3 AND status='reserved'",
            params![
                source.budget.budget_reservation_id,
                source.consumer_account_id,
                source.budget.budget_reserved_fen,
            ],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Execution plan budget reservation is stale"))?;

    Ok(CurrentExecutionSources {
        historical_provider,
        historical_offer,
        job,
        reservation,
        claim,
        broker,
        broker_request_digest,
        budget_expires_at,
    })
}

fn ensure_current_route(
    provider: &crate::compute_federation::provider::ComputeProvider,
    envelope: &ComputeExecutionCapabilityEnvelope,
) -> Result<()> {
    let capability = &envelope.capability;
    if capability.provider_id != provider.provider_id
        || capability.provider_kind != provider.provider_kind
    {
        bail!("Execution capability does not belong to the current Provider");
    }
    match capability.capability_kind.as_str() {
        EXECUTION_CAPABILITY_NODE_READY if provider.provider_kind != PROVIDER_KIND_USER_NODE => {
            bail!("Node-ready capability requires a user-node Provider");
        }
        EXECUTION_CAPABILITY_PROVIDER_ENDPOINT
            if provider.provider_kind != PROVIDER_KIND_MANAGED_CLUSTER =>
        {
            bail!("Provider-endpoint capability requires a managed-cluster Provider");
        }
        EXECUTION_CAPABILITY_ADAPTER_EXECUTION
            if !matches!(
                provider.provider_kind.as_str(),
                PROVIDER_KIND_MANAGED_CLUSTER | PROVIDER_KIND_EXTERNAL_POOL
            ) =>
        {
            bail!("Adapter capability requires a managed-cluster or external-pool Provider");
        }
        _ => {}
    }
    let route = &capability.route;
    match route.route_kind.as_str() {
        COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT => {
            let endpoint = provider
                .endpoint
                .as_ref()
                .ok_or_else(|| anyhow!("Current Provider endpoint route is missing"))?;
            if route.endpoint_id.as_deref() != Some(endpoint.endpoint_id.as_str())
                || route.endpoint_transport.as_deref() != Some(endpoint.transport.as_str())
            {
                bail!("Execution capability endpoint route is stale");
            }
        }
        COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER => {
            let adapter = provider
                .adapter
                .as_ref()
                .ok_or_else(|| anyhow!("Current Provider server Adapter is missing"))?;
            if route.endpoint_id.is_some()
                || route.endpoint_transport.is_some()
                || route.adapter_id != adapter.adapter_id
                || route.adapter_version != adapter.adapter_version
                || route.adapter_config_revision != adapter.config_revision
                || route.adapter_config_digest != adapter.config_digest
            {
                bail!("Execution capability server Adapter route is stale");
            }
        }
        _ => bail!("Execution capability route kind is unsupported"),
    }
    if provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
        && route.route_kind != COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER
    {
        bail!("External-pool execution must use the current server Adapter");
    }
    Ok(())
}
