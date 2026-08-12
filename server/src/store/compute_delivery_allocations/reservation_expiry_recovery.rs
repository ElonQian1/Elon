use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
    store::{
        compute_attempt_start_outbox::BrokerFinishStartUnresolved, ComputeBrokerFinishAction,
        ComputeBrokerFinishReceipt, FinishComputeBrokerRequest,
    },
};

use super::super::{now, Store};

pub(crate) const COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION: &str =
    "confirm_compute_delivery_allocation_reservation_expire_due";
pub(crate) const COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRY_IDEMPOTENCY_PREFIX: &str =
    "sys-da-expire-v1:";

const EXPIRY_KEY_SCHEMA: &str = "compute_federation.delivery_allocation_reservation_expiry_key.v1";
const EXPIRY_KEY_JSON_LIMIT: usize = 16 * 1024;
const STATUS_EXPIRED: &str = "expired";
const STATUS_BLOCKED_NO_START: &str = "blocked_no_start";
const STATUS_FAILED: &str = "failed";
const FAILURE_START_UNRESOLVED: &str = "ATTEMPT_START_UNRESOLVED";
const FAILURE_EXPIRY_FAILED: &str = "DELIVERY_ALLOCATION_RESERVATION_EXPIRY_FAILED";

#[derive(Debug, Clone)]
pub(crate) struct ExpireDueComputeDeliveryAllocationReservations {
    pub limit: usize,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationReservationExpiryItem {
    pub grant_id: String,
    pub terminal_receipt_id: String,
    pub reservation_id: String,
    pub source_reservation_revision: i64,
    pub source_reservation_digest: String,
    pub expires_at: String,
    pub status: String,
    pub replayed: bool,
    pub broker_finish: Option<ComputeBrokerFinishReceipt>,
    pub failure_code: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeDeliveryAllocationReservationExpiryReport {
    pub recovery_started_at: String,
    pub selected_count: usize,
    pub expired_count: usize,
    pub replayed_count: usize,
    pub blocked_count: usize,
    pub failed_count: usize,
    pub items: Vec<ComputeDeliveryAllocationReservationExpiryItem>,
    pub money_effect: &'static str,
    pub provider_balance_effect: &'static str,
    pub settlement_effect: &'static str,
}

#[derive(Debug, Clone)]
pub(super) struct DueReservationCandidate {
    pub(super) grant_id: String,
    pub(super) terminal_receipt_id: String,
    pub(super) reservation_id: String,
    pub(super) consumer_account_id: String,
    pub(super) source_reservation_revision: i64,
    pub(super) source_reservation_digest: String,
    pub(super) expires_at: String,
    pub(super) job_id: String,
    pub(super) capacity_claim_id: String,
    pub(super) budget_reservation_id: String,
}

#[derive(Serialize)]
struct ExpiryKeyMaterial<'a> {
    schema: &'static str,
    reservation_id: &'a str,
    source_revision: i64,
    source_digest: &'a str,
    expires_at: &'a str,
}

impl Store {
    pub(crate) fn expire_due_compute_delivery_allocation_reservations(
        &self,
        input: ExpireDueComputeDeliveryAllocationReservations,
    ) -> Result<ComputeDeliveryAllocationReservationExpiryReport> {
        validate_input(&input)?;
        let recovery_started_at = now();
        let candidates = {
            let connection = self.conn()?;
            due_reservations_on(&connection, &recovery_started_at, None, None, input.limit)?
        };
        Ok(self.expire_due_reservation_candidates(recovery_started_at, candidates))
    }

    pub(super) fn expire_due_reservation_candidates(
        &self,
        recovery_started_at: String,
        candidates: Vec<DueReservationCandidate>,
    ) -> ComputeDeliveryAllocationReservationExpiryReport {
        let mut report = ComputeDeliveryAllocationReservationExpiryReport {
            recovery_started_at,
            selected_count: candidates.len(),
            expired_count: 0,
            replayed_count: 0,
            blocked_count: 0,
            failed_count: 0,
            items: Vec::with_capacity(candidates.len()),
            money_effect: "preauthorization_refund_only",
            provider_balance_effect: "none",
            settlement_effect: "none",
        };
        for candidate in candidates {
            report
                .items
                .push(match self.expire_due_reservation(&candidate) {
                    Ok(receipt) => {
                        report.expired_count += 1;
                        if receipt.replayed {
                            report.replayed_count += 1;
                        }
                        item_for_success(candidate, receipt)
                    }
                    Err(error)
                        if error
                            .downcast_ref::<BrokerFinishStartUnresolved>()
                            .is_some() =>
                    {
                        report.blocked_count += 1;
                        item_for_error(
                            candidate,
                            STATUS_BLOCKED_NO_START,
                            FAILURE_START_UNRESOLVED,
                            error,
                        )
                    }
                    Err(error) => {
                        report.failed_count += 1;
                        item_for_error(candidate, STATUS_FAILED, FAILURE_EXPIRY_FAILED, error)
                    }
                });
        }
        report
    }

    fn expire_due_reservation(
        &self,
        candidate: &DueReservationCandidate,
    ) -> Result<ComputeBrokerFinishReceipt> {
        validate_candidate(candidate)?;
        self.finish_compute_broker(&FinishComputeBrokerRequest {
            reservation_id: candidate.reservation_id.clone(),
            consumer_account_id: candidate.consumer_account_id.clone(),
            idempotency_key: expiry_idempotency_key(candidate)?,
            expected_reservation_revision: candidate.source_reservation_revision,
            expected_reservation_digest: candidate.source_reservation_digest.clone(),
            action: ComputeBrokerFinishAction::Expire,
            occurred_at: candidate.expires_at.clone(),
        })
    }
}

fn validate_input(input: &ExpireDueComputeDeliveryAllocationReservations) -> Result<()> {
    if !(1..=100).contains(&input.limit) {
        bail!("DeliveryAllocation Reservation 到期恢复 limit 必须在 1..=100");
    }
    if input.confirmation != COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION {
        bail!("DeliveryAllocation Reservation 到期恢复需要固定确认文本");
    }
    Ok(())
}

pub(super) fn due_reservations_on(
    connection: &Connection,
    recovery_started_at: &str,
    after_expires_at: Option<&str>,
    after_reservation_id: Option<&str>,
    limit: usize,
) -> Result<Vec<DueReservationCandidate>> {
    let mut statement = connection.prepare(
        "SELECT grant.grant_id, terminal.terminal_receipt_id, terminal.reservation_id,
                reservation.consumer_account_id, reservation.current_revision,
                reservation.current_reservation_digest, reservation.expires_at,
                reservation.job_id, reservation.capacity_claim_id,
                terminal.budget_reservation_id
           FROM compute_delivery_allocation_terminal_receipts terminal
           JOIN compute_delivery_allocation_grants grant
             ON grant.grant_id=terminal.grant_id
            AND grant.grant_digest=terminal.grant_digest
            AND grant.commitment_id=terminal.commitment_id
            AND grant.commitment_revision=terminal.commitment_revision
            AND grant.commitment_digest=terminal.commitment_digest
           JOIN compute_reservations reservation
             ON reservation.reservation_id=terminal.reservation_id
           JOIN compute_jobs job ON job.job_id=reservation.job_id
           JOIN compute_capacity_claims claim
             ON claim.claim_id=reservation.capacity_claim_id
           JOIN compute_broker_reserve_receipts broker
             ON broker.reservation_id=reservation.reservation_id
           JOIN billing_reservations money
             ON money.id=terminal.budget_reservation_id
          WHERE terminal.terminal_status='exercised'
            AND reservation.status='active'
            AND job.status='reserved'
            AND claim.status='held'
            AND grant.consumer_account_id=reservation.consumer_account_id
            AND grant.job_id=reservation.job_id
            AND grant.job_revision=terminal.source_job_revision
            AND grant.job_digest=terminal.source_job_digest
            AND job.consumer_account_id=reservation.consumer_account_id
            AND job.current_revision=reservation.job_revision
            AND job.current_job_digest=reservation.job_digest
            AND claim.revision=reservation.capacity_claim_revision
            AND claim.claim_digest=reservation.capacity_claim_digest
            AND claim.claim_kind='reservation'
            AND claim.subject_kind='compute_reservation'
            AND claim.subject_id=reservation.reservation_id
            AND claim.parent_claim_id=terminal.parent_claim_id
            AND terminal.reservation_revision=reservation.current_revision
            AND terminal.reservation_digest=reservation.current_reservation_digest
            AND reservation.consumer_authorization_ref=terminal.budget_reservation_id
            AND terminal.reservation_claim_id=reservation.capacity_claim_id
            AND terminal.reservation_claim_revision=reservation.capacity_claim_revision
            AND terminal.reservation_claim_digest=reservation.capacity_claim_digest
            AND terminal.reserved_job_revision=reservation.job_revision
            AND terminal.reserved_job_digest=reservation.job_digest
            AND broker.consumer_account_id=reservation.consumer_account_id
            AND broker.reservation_revision=reservation.current_revision
            AND broker.reservation_digest=reservation.current_reservation_digest
            AND broker.job_id=reservation.job_id
            AND broker.request_digest=terminal.broker_reserve_request_digest
            AND broker.source_job_revision=terminal.source_job_revision
            AND broker.source_job_digest=terminal.source_job_digest
            AND broker.reserved_job_revision=terminal.reserved_job_revision
            AND broker.reserved_job_digest=terminal.reserved_job_digest
            AND broker.capacity_claim_id=reservation.capacity_claim_id
            AND broker.capacity_claim_revision=reservation.capacity_claim_revision
            AND broker.capacity_claim_digest=reservation.capacity_claim_digest
            AND broker.budget_reservation_id=terminal.budget_reservation_id
            AND broker.budget_reserved_fen=terminal.reserved_amount_fen
            AND broker.budget_adapter='platform_balance_cny'
            AND money.user_id=reservation.consumer_account_id
            AND money.compute_call_id=('compute_broker:' || reservation.reservation_id)
            AND money.feature='compute_federation_reservation'
            AND money.usage_mode='platform_balance_cny'
            AND money.status='reserved'
            AND money.reserved_fen=terminal.reserved_amount_fen
            AND julianday(reservation.expires_at)<=julianday(?1)
            AND (
                ?2 IS NULL
                OR julianday(reservation.expires_at)>julianday(?2)
                OR (julianday(reservation.expires_at)=julianday(?2)
                    AND reservation.expires_at>?2)
                OR (julianday(reservation.expires_at)=julianday(?2)
                    AND reservation.expires_at=?2
                    AND reservation.reservation_id>?3)
            )
            AND NOT EXISTS (
                SELECT 1 FROM compute_broker_finish_receipts finish
                 WHERE finish.reservation_id=reservation.reservation_id
            )
          ORDER BY julianday(reservation.expires_at), reservation.expires_at,
                   reservation.reservation_id
          LIMIT ?4",
    )?;
    let rows = statement.query_map(
        params![
            recovery_started_at,
            after_expires_at,
            after_reservation_id,
            limit as i64
        ],
        |row| {
            Ok(DueReservationCandidate {
                grant_id: row.get(0)?,
                terminal_receipt_id: row.get(1)?,
                reservation_id: row.get(2)?,
                consumer_account_id: row.get(3)?,
                source_reservation_revision: row.get(4)?,
                source_reservation_digest: row.get(5)?,
                expires_at: row.get(6)?,
                job_id: row.get(7)?,
                capacity_claim_id: row.get(8)?,
                budget_reservation_id: row.get(9)?,
            })
        },
    )?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn validate_candidate(candidate: &DueReservationCandidate) -> Result<()> {
    let required = [
        ("grant_id", candidate.grant_id.as_str()),
        (
            "terminal_receipt_id",
            candidate.terminal_receipt_id.as_str(),
        ),
        ("reservation_id", candidate.reservation_id.as_str()),
        (
            "consumer_account_id",
            candidate.consumer_account_id.as_str(),
        ),
        (
            "source_reservation_digest",
            candidate.source_reservation_digest.as_str(),
        ),
        ("expires_at", candidate.expires_at.as_str()),
        ("job_id", candidate.job_id.as_str()),
        ("capacity_claim_id", candidate.capacity_claim_id.as_str()),
        (
            "budget_reservation_id",
            candidate.budget_reservation_id.as_str(),
        ),
    ];
    if candidate.source_reservation_revision <= 0
        || required.iter().any(|(_, value)| value.trim().is_empty())
    {
        bail!("DeliveryAllocation Reservation 到期候选投影无效");
    }
    Ok(())
}

fn expiry_idempotency_key(candidate: &DueReservationCandidate) -> Result<String> {
    let material = ExpiryKeyMaterial {
        schema: EXPIRY_KEY_SCHEMA,
        reservation_id: &candidate.reservation_id,
        source_revision: candidate.source_reservation_revision,
        source_digest: &candidate.source_reservation_digest,
        expires_at: &candidate.expires_at,
    };
    let (canonical, _) =
        canonical_compute_plugin_ijson_and_sha256(&material, EXPIRY_KEY_JSON_LIMIT)?;
    let digest = hex::encode(Sha256::digest(canonical.as_bytes()));
    Ok(format!(
        "{COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRY_IDEMPOTENCY_PREFIX}{digest}"
    ))
}

fn item_for_success(
    candidate: DueReservationCandidate,
    receipt: ComputeBrokerFinishReceipt,
) -> ComputeDeliveryAllocationReservationExpiryItem {
    ComputeDeliveryAllocationReservationExpiryItem {
        grant_id: candidate.grant_id,
        terminal_receipt_id: candidate.terminal_receipt_id,
        reservation_id: candidate.reservation_id,
        source_reservation_revision: candidate.source_reservation_revision,
        source_reservation_digest: candidate.source_reservation_digest,
        expires_at: candidate.expires_at,
        status: STATUS_EXPIRED.to_string(),
        replayed: receipt.replayed,
        broker_finish: Some(receipt),
        failure_code: None,
        error: None,
    }
}

fn item_for_error(
    candidate: DueReservationCandidate,
    status: &str,
    failure_code: &str,
    error: anyhow::Error,
) -> ComputeDeliveryAllocationReservationExpiryItem {
    ComputeDeliveryAllocationReservationExpiryItem {
        grant_id: candidate.grant_id,
        terminal_receipt_id: candidate.terminal_receipt_id,
        reservation_id: candidate.reservation_id,
        source_reservation_revision: candidate.source_reservation_revision,
        source_reservation_digest: candidate.source_reservation_digest,
        expires_at: candidate.expires_at,
        status: status.to_string(),
        replayed: false,
        broker_finish: None,
        failure_code: Some(failure_code.to_string()),
        error: Some(error.to_string()),
    }
}
