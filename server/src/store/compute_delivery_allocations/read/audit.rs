use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState},
    delivery_allocation::{
        ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
        DELIVERY_ALLOCATION_ACTOR_ADMIN, DELIVERY_ALLOCATION_ACTOR_CONSUMER,
        DELIVERY_ALLOCATION_STATUS_DECLINED, DELIVERY_ALLOCATION_STATUS_EXERCISED,
        DELIVERY_ALLOCATION_STATUS_EXPIRED,
    },
    execution::{JOB_STATUS_QUOTED, JOB_STATUS_RESERVED, RESERVATION_STATUS_ACTIVE},
};

use super::super::{
    super::{
        compute_broker_reservation::broker_reserve_binding_on,
        compute_capacity_claim_rows::{stored_claim_on, stored_claim_version_on},
        compute_capacity_commitments::{
            audited_capacity_commitment_source_on, audited_historical_capacity_commitment_source_on,
        },
        compute_job_registry::{registered_historical_job_version_on, registered_job_version_on},
        compute_reservation_registry::registered_reservation_version_on,
    },
    types::{DeliveryAllocationClaimTransferAuthority, DeliveryAllocationReservationAuthority},
    validation::parse_utc,
};
use super::ledger::{audit_child_hold_on, audit_parent_release_on};

pub(super) fn audit_grant_indexes_on(
    conn: &Connection,
    value: &ComputeDeliveryAllocationGrant,
) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_delivery_allocation_grants WHERE
                grant_id=?1 AND grant_schema=?2 AND grant_revision=?3 AND grant_status=?4
                AND grant_digest=?5 AND commitment_id=?6 AND commitment_revision=?7
                AND commitment_digest=?8 AND provider_owner_account_id=?9
                AND consumer_account_id=?10 AND project_id IS ?11 AND job_id=?12
                AND job_revision=?13 AND job_digest=?14 AND exercise_expires_at=?15
                AND idempotency_scope=?16 AND idempotency_key=?17 AND request_digest=?18
                AND created_at=?19 AND canonicalization='rfc8785_jcs'
                AND digest_algorithm='sha256'",
            params![
                value.grant_id,
                value.schema,
                value.grant_revision,
                value.grant_status,
                value.grant_digest,
                value.commitment.commitment_id,
                value.commitment.commitment_revision,
                value.commitment.commitment_digest,
                value.provider_owner_account_id,
                value.consumer_account_id,
                value.project_id,
                value.job.job_id,
                value.job.job_revision,
                value.job.job_digest,
                value.exercise_expires_at,
                value.idempotency_scope,
                value.idempotency_key,
                value.request_digest,
                value.created_at,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("DeliveryAllocation Grant indexed columns 与 immutable JSON 不一致");
    }
    Ok(())
}

pub(super) fn audit_grant_dependencies_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<()> {
    audit_grant_dependencies_with_policy_on(conn, grant, false)
}

pub(super) fn audit_historical_grant_dependencies_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<()> {
    audit_grant_dependencies_with_policy_on(conn, grant, true)
}

fn audit_grant_dependencies_with_policy_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    use_historical_dependencies: bool,
) -> Result<()> {
    let source = if use_historical_dependencies {
        audited_historical_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
    } else {
        audited_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
    };
    let (commitment, _) =
        source.ok_or_else(|| anyhow!("DeliveryAllocation Grant 来源 Commitment 缺失"))?;
    let root = commitment.commitment;
    if root.commitment_revision != grant.commitment.commitment_revision
        || root.commitment_digest != grant.commitment.commitment_digest
        || root.owner_account_id != grant.provider_owner_account_id
        || root.delivery_window.starts_at_utc != grant.exercise_expires_at
    {
        bail!("DeliveryAllocation Grant 来源 Commitment 摘要或窗口不一致");
    }
    let job = if use_historical_dependencies {
        registered_historical_job_version_on(conn, &grant.job.job_id, grant.job.job_revision)?
    } else {
        registered_job_version_on(conn, &grant.job.job_id, grant.job.job_revision)?
    }
    .ok_or_else(|| anyhow!("DeliveryAllocation Grant Job 历史版本缺失"))?;
    if job.job_digest != grant.job.job_digest
        || job.job.consumer_account_id != grant.consumer_account_id
        || job.job.project_id != grant.project_id
        || job.job.status != JOB_STATUS_QUOTED
        || job.job.selected_offer.as_ref().map_or(true, |offer| {
            offer.offer_id != root.offer.offer_id
                || offer.offer_version != root.offer.offer_version
                || offer.offer_digest != root.offer.offer_digest
        })
        || job.job.price_snapshot_id.as_deref() != Some(root.price_snapshot_id.as_str())
    {
        bail!("DeliveryAllocation Grant Job 历史绑定不一致");
    }
    Ok(())
}

pub(super) fn audit_terminal_indexes_on(
    conn: &Connection,
    value: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<()> {
    let exercise = value.exercise.as_ref();
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_delivery_allocation_terminal_receipts WHERE
                terminal_receipt_id=?1 AND terminal_schema=?2 AND terminal_revision=?3
                AND terminal_status=?4 AND terminal_receipt_digest=?5 AND grant_id=?6
                AND grant_digest=?7 AND commitment_id=?8 AND commitment_revision=?9
                AND commitment_digest=?10 AND actor_kind=?11 AND actor_id=?12
                AND idempotency_scope=?13 AND idempotency_key=?14 AND request_digest=?15
                AND occurred_at=?16 AND recorded_at=?17
                AND parent_claim_id IS ?18 AND parent_prior_claim_revision IS ?19
                AND parent_prior_claim_digest IS ?20 AND parent_result_claim_revision IS ?21
                AND parent_result_claim_digest IS ?22 AND parent_result_claim_state IS ?23
                AND parent_release_transaction_id IS ?24
                AND parent_release_transaction_digest IS ?25
                AND parent_release_ledger_sequence IS ?26 AND parent_release_event_kind IS ?27
                AND parent_release_causal_transaction_id IS ?28
                AND reservation_claim_id IS ?29 AND reservation_claim_revision IS ?30
                AND reservation_claim_digest IS ?31 AND reservation_parent_claim_id IS ?32
                AND reservation_hold_transaction_id IS ?33
                AND reservation_hold_transaction_digest IS ?34
                AND reservation_hold_ledger_sequence IS ?35 AND reservation_hold_event_kind IS ?36
                AND reservation_hold_causal_transaction_id IS ?37
                AND reservation_id IS ?38 AND reservation_revision IS ?39
                AND reservation_digest IS ?40 AND source_job_revision IS ?41
                AND source_job_digest IS ?42 AND reserved_job_revision IS ?43
                AND reserved_job_digest IS ?44 AND budget_reservation_id IS ?45
                AND reserved_amount_fen IS ?46 AND broker_reserve_request_digest IS ?47
                AND canonicalization='rfc8785_jcs' AND digest_algorithm='sha256'",
            params![
                value.terminal_receipt_id,
                value.schema,
                value.terminal_revision,
                value.terminal_status,
                value.terminal_receipt_digest,
                value.grant_id,
                value.grant_digest,
                value.commitment.commitment_id,
                value.commitment.commitment_revision,
                value.commitment.commitment_digest,
                value.actor_kind,
                value.actor_id,
                value.idempotency_scope,
                value.idempotency_key,
                value.request_digest,
                value.occurred_at,
                value.recorded_at,
                exercise.map(|x| x.parent_claim_id.as_str()),
                exercise.map(|x| x.parent_prior_claim_revision),
                exercise.map(|x| x.parent_prior_claim_digest.as_str()),
                exercise.map(|x| x.parent_result_claim_revision),
                exercise.map(|x| x.parent_result_claim_digest.as_str()),
                exercise.map(|x| x.parent_result_claim_state.as_str()),
                exercise.map(|x| x.parent_release_ledger.transaction_id.as_str()),
                exercise.map(|x| x.parent_release_ledger.transaction_digest.as_str()),
                exercise.map(|x| x.parent_release_ledger.ledger_sequence),
                exercise.map(|x| x.parent_release_ledger.event_kind.as_str()),
                exercise.map(|x| x.parent_release_ledger.causal_transaction_id.as_str()),
                exercise.map(|x| x.reservation_claim.claim_id.as_str()),
                exercise.map(|x| x.reservation_claim.claim_revision),
                exercise.map(|x| x.reservation_claim.claim_digest.as_str()),
                exercise.map(|x| x.reservation_claim.parent_claim_id.as_str()),
                exercise.map(|x| x.reservation_hold_ledger.transaction_id.as_str()),
                exercise.map(|x| x.reservation_hold_ledger.transaction_digest.as_str()),
                exercise.map(|x| x.reservation_hold_ledger.ledger_sequence),
                exercise.map(|x| x.reservation_hold_ledger.event_kind.as_str()),
                exercise.map(|x| x.reservation_hold_ledger.causal_transaction_id.as_str()),
                exercise.map(|x| x.reservation.reservation_id.as_str()),
                exercise.map(|x| x.reservation.reservation_revision),
                exercise.map(|x| x.reservation.reservation_digest.as_str()),
                exercise.map(|x| x.source_job_revision),
                exercise.map(|x| x.source_job_digest.as_str()),
                exercise.map(|x| x.reserved_job_revision),
                exercise.map(|x| x.reserved_job_digest.as_str()),
                exercise.map(|x| x.budget_reservation_id.as_str()),
                exercise.map(|x| x.reserved_amount_fen),
                exercise.map(|x| x.broker_reserve_request_digest.as_str()),
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("DeliveryAllocation terminal indexed columns 与 immutable JSON 不一致");
    }
    Ok(())
}

pub(super) fn reservation_authority_from_terminal_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<DeliveryAllocationReservationAuthority> {
    reservation_authority_from_terminal_with_parent_policy_on(conn, grant, terminal, true)
}

pub(super) fn historical_reservation_authority_from_terminal_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<DeliveryAllocationReservationAuthority> {
    reservation_authority_from_terminal_with_parent_policy_on(conn, grant, terminal, false)
}

fn reservation_authority_from_terminal_with_parent_policy_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
    require_current_parent: bool,
) -> Result<DeliveryAllocationReservationAuthority> {
    let evidence = terminal
        .exercise
        .as_ref()
        .ok_or_else(|| anyhow!("DeliveryAllocation terminal 不是 exercised"))?;
    let occurred_at = parse_utc(
        "DeliveryAllocation exercised occurred_at",
        &terminal.occurred_at,
    )?;
    let exercise_expires_at = parse_utc(
        "DeliveryAllocation exercise expiry",
        &grant.exercise_expires_at,
    )?;
    if terminal.terminal_status != DELIVERY_ALLOCATION_STATUS_EXERCISED
        || terminal.actor_kind != DELIVERY_ALLOCATION_ACTOR_CONSUMER
        || terminal.actor_id != grant.consumer_account_id
        || terminal.occurred_at != terminal.recorded_at
        || occurred_at >= exercise_expires_at
        || evidence.parent_claim_id.is_empty()
        || evidence.parent_release_ledger.event_kind != "reservation_released"
        || evidence.reservation_hold_ledger.event_kind != "reservation_held"
    {
        bail!("DeliveryAllocation exercised actor/evidence shape 不一致");
    }
    let source = if require_current_parent {
        audited_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
    } else {
        audited_historical_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
    };
    let (commitment, old_terminal) =
        source.ok_or_else(|| anyhow!("DeliveryAllocation exercised Commitment 缺失"))?;
    if old_terminal.is_some()
        || commitment.commitment.commitment_digest != grant.commitment.commitment_digest
    {
        bail!("DeliveryAllocation exercised 与 v225 terminal/source 冲突");
    }
    let parent_prior = stored_claim_version_on(
        conn,
        &evidence.parent_claim_id,
        evidence.parent_prior_claim_revision,
    )?
    .ok_or_else(|| anyhow!("DeliveryAllocation parent prior Claim 缺失"))?;
    let parent_result = stored_claim_version_on(
        conn,
        &evidence.parent_claim_id,
        evidence.parent_result_claim_revision,
    )?
    .ok_or_else(|| anyhow!("DeliveryAllocation parent result Claim 缺失"))?;
    let child_claim = stored_claim_version_on(
        conn,
        &evidence.reservation_claim.claim_id,
        evidence.reservation_claim.claim_revision,
    )?
    .ok_or_else(|| anyhow!("DeliveryAllocation child Reservation Claim 缺失"))?;
    let current_parent = if require_current_parent {
        Some(
            stored_claim_on(conn, &evidence.parent_claim_id)?
                .ok_or_else(|| anyhow!("DeliveryAllocation current parent Claim 缺失"))?,
        )
    } else {
        None
    };
    if evidence.parent_claim_id != commitment.commitment.claim.claim_id
        || evidence.parent_prior_claim_revision != 1
        || evidence.parent_prior_claim_digest != commitment.commitment.claim.claim_digest
        || evidence.parent_result_claim_revision != 2
        || evidence.parent_result_claim_state != "released"
        || evidence.reservation_claim.claim_revision != 1
        || evidence.reservation_claim.parent_claim_id != evidence.parent_claim_id
        || evidence.parent_release_ledger.causal_transaction_id
            != commitment.commitment.creation_ledger.transaction_id
        || evidence.reservation_hold_ledger.causal_transaction_id
            != evidence.parent_release_ledger.transaction_id
        || parent_prior.claim_digest != evidence.parent_prior_claim_digest
        || parent_prior.state != ComputeCapacityClaimState::Held
        || parent_prior.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || parent_prior.subject_kind != "compute_capacity_commitment"
        || parent_prior.subject_id != commitment.commitment.commitment_id
        || parent_prior.parent_claim_id.is_some()
        || parent_result.claim_digest != evidence.parent_result_claim_digest
        || parent_result.state != ComputeCapacityClaimState::Released
        || parent_result.subject_kind != "compute_capacity_commitment"
        || parent_result.subject_id != commitment.commitment.commitment_id
        || parent_result.parent_claim_id.is_some()
        || current_parent
            .as_ref()
            .is_some_and(|current| current != &parent_result)
        || child_claim.claim_digest != evidence.reservation_claim.claim_digest
        || child_claim.state != ComputeCapacityClaimState::Held
        || child_claim.claim_kind != ComputeCapacityClaimKind::Reservation
        || child_claim.subject_kind != "compute_reservation"
        || child_claim.subject_id != evidence.reservation.reservation_id
        || child_claim.parent_claim_id.as_deref() != Some(evidence.parent_claim_id.as_str())
        || child_claim.pool != parent_prior.pool
        || child_claim.delivery_window != parent_prior.delivery_window
        || parent_prior.lines != child_claim.lines
    {
        bail!("DeliveryAllocation whole-only parent/child Claim 证据不一致");
    }
    audit_parent_release_on(
        conn,
        &evidence.parent_release_ledger,
        &parent_prior,
        &commitment.commitment,
    )?;
    audit_child_hold_on(
        conn,
        &evidence.reservation_hold_ledger,
        &child_claim,
        &commitment.commitment,
        &grant.job.job_id,
        &evidence.reservation.reservation_id,
        &evidence.parent_release_ledger.transaction_id,
    )?;
    let source_job = if require_current_parent {
        registered_job_version_on(conn, &grant.job.job_id, grant.job.job_revision)?
    } else {
        registered_historical_job_version_on(conn, &grant.job.job_id, grant.job.job_revision)?
    }
    .ok_or_else(|| anyhow!("DeliveryAllocation source Job 缺失"))?;
    let transfer = DeliveryAllocationClaimTransferAuthority::new(
        grant.clone(),
        commitment.commitment,
        parent_prior,
        source_job,
        evidence.reservation.reservation_id.clone(),
        terminal.idempotency_key.clone(),
        source_job_deadline_with_policy_on(
            conn,
            &grant.job.job_id,
            grant.job.job_revision,
            !require_current_parent,
        )?,
        terminal.occurred_at.clone(),
    );
    Ok(DeliveryAllocationReservationAuthority::new(
        transfer,
        parent_result,
        evidence.parent_release_ledger.clone(),
        child_claim,
        evidence.reservation_hold_ledger.clone(),
    ))
}

pub(super) fn audit_exercise_consumers_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<()> {
    let evidence = terminal
        .exercise
        .as_ref()
        .ok_or_else(|| anyhow!("缺少 exercise evidence"))?;
    let _authority = reservation_authority_from_terminal_on(conn, grant, terminal)?;
    let reservation = registered_reservation_version_on(
        conn,
        &evidence.reservation.reservation_id,
        evidence.reservation.reservation_revision,
    )?
    .ok_or_else(|| anyhow!("DeliveryAllocation Reservation 历史版本缺失"))?;
    let reserved_job =
        registered_job_version_on(conn, &grant.job.job_id, evidence.reserved_job_revision)?
            .ok_or_else(|| anyhow!("DeliveryAllocation reserved Job 历史版本缺失"))?;
    let broker = broker_reserve_binding_on(
        conn,
        &evidence.reservation.reservation_id,
        &grant.consumer_account_id,
    )?;
    if reservation.reservation_digest != evidence.reservation.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || reserved_job.job_digest != evidence.reserved_job_digest
        || reserved_job.job.status != JOB_STATUS_RESERVED
        || broker.request_digest != evidence.broker_reserve_request_digest
        || broker.budget_reservation_id != evidence.budget_reservation_id
        || broker.budget_reserved_fen != evidence.reserved_amount_fen
        || broker.capacity_claim.claim_id != evidence.reservation_claim.claim_id
        || broker.capacity_claim.claim_digest != evidence.reservation_claim.claim_digest
        || broker.source_job.job_revision != evidence.source_job_revision
        || broker.source_job.job_digest != evidence.source_job_digest
        || broker.reserved_job.job_revision != evidence.reserved_job_revision
        || broker.reserved_job.job_digest != evidence.reserved_job_digest
        || broker.reservation_revision != evidence.reservation.reservation_revision
        || broker.reservation_digest != evidence.reservation.reservation_digest
    {
        bail!("DeliveryAllocation Broker/Reservation/Job readback 审计失败");
    }
    Ok(())
}

pub(super) fn validate_non_exercise_terminal(
    grant: &ComputeDeliveryAllocationGrant,
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<()> {
    let recorded_at = parse_utc(
        "DeliveryAllocation terminal recorded_at",
        &terminal.recorded_at,
    )?;
    let exercise_expires_at = parse_utc(
        "DeliveryAllocation exercise expiry",
        &grant.exercise_expires_at,
    )?;
    let valid = match terminal.terminal_status.as_str() {
        DELIVERY_ALLOCATION_STATUS_DECLINED => {
            terminal.actor_kind == DELIVERY_ALLOCATION_ACTOR_CONSUMER
                && terminal.actor_id == grant.consumer_account_id
                && terminal.exercise.is_none()
                && terminal.occurred_at == terminal.recorded_at
                && recorded_at < exercise_expires_at
        }
        DELIVERY_ALLOCATION_STATUS_EXPIRED => {
            terminal.actor_kind == DELIVERY_ALLOCATION_ACTOR_ADMIN
                && terminal.exercise.is_none()
                && terminal.occurred_at == grant.exercise_expires_at
                && recorded_at >= exercise_expires_at
        }
        _ => false,
    };
    if !valid {
        bail!("DeliveryAllocation non-exercise terminal actor/evidence 无效");
    }
    Ok(())
}

fn source_job_deadline_with_policy_on(
    conn: &Connection,
    job_id: &str,
    revision: i64,
    use_historical_dependencies: bool,
) -> Result<String> {
    let job = if use_historical_dependencies {
        registered_historical_job_version_on(conn, job_id, revision)?
    } else {
        registered_job_version_on(conn, job_id, revision)?
    };
    Ok(job
        .ok_or_else(|| anyhow!("DeliveryAllocation source Job deadline 缺失"))?
        .job
        .workload
        .deadline_at)
}
