use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use crate::{
    compute_federation::capacity_commitment_service,
    store::{
        ComputeBrokerFinishAction, ExpireDueComputeDeliveryAllocationReservations,
        COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION,
    },
};

use super::{get_for_consumer, tests::Fixture};

#[test]
fn exercised_reservation_expiry_refunds_and_releases_capacity_exactly_once() {
    let fixture = Fixture::new_expiry_recovery();
    let grant = fixture.create_grant("expiry", true).unwrap();
    fixture.recharge(100);
    let reservation_id = fixture.reservation_id("expiry");
    let exercised = fixture
        .exercise(&grant, &reservation_id, "expiry", true)
        .unwrap();
    let exercise = exercised.terminal_receipt.exercise.as_ref().unwrap();
    let child_claim_id = exercise.reservation_claim.claim_id.clone();
    let budget_reservation_id = exercise.budget_reservation_id.clone();
    let reservation = fixture
        .supply
        .store
        .compute_reservation(&reservation_id)
        .unwrap();
    let expires_at = reservation.reservation.expires_at.clone();

    assert_eq!(fixture.balance(), 90);
    assert_eq!(fixture.capacity(), ((80, 20), (3, 1)));
    assert_eq!(reservation.reservation.status, "active");
    assert_eq!(claim_state(&fixture, &child_claim_id), ("held".into(), 1));
    assert_eq!(fixture.table_count("compute_broker_finish_receipts"), 0);

    for invalid in [
        ExpireDueComputeDeliveryAllocationReservations {
            limit: 0,
            confirmation: COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION.into(),
        },
        ExpireDueComputeDeliveryAllocationReservations {
            limit: 101,
            confirmation: COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION.into(),
        },
        ExpireDueComputeDeliveryAllocationReservations {
            limit: 10,
            confirmation: "wrong-confirmation".into(),
        },
    ] {
        assert!(fixture
            .supply
            .store
            .expire_due_compute_delivery_allocation_reservations(invalid)
            .is_err());
    }
    assert_eq!(fixture.balance(), 90);
    assert_eq!(claim_state(&fixture, &child_claim_id), ("held".into(), 1));
    assert_eq!(fixture.table_count("compute_broker_finish_receipts"), 0);

    let _clock = advance_store_clock_past(&expires_at);
    let report = fixture
        .supply
        .store
        .expire_due_compute_delivery_allocation_reservations(expire_request(10))
        .unwrap();

    assert_eq!(report.selected_count, 1);
    assert_eq!(report.expired_count, 1);
    assert_eq!(report.replayed_count, 0);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(report.failed_count, 0);
    assert_eq!(report.money_effect, "preauthorization_refund_only");
    assert_eq!(report.provider_balance_effect, "none");
    assert_eq!(report.settlement_effect, "none");
    assert_eq!(report.items.len(), 1);
    let item = &report.items[0];
    assert_eq!(item.grant_id, grant.grant.grant_id);
    assert_eq!(item.reservation_id, reservation_id);
    assert_eq!(item.source_reservation_revision, reservation.revision);
    assert_eq!(
        item.source_reservation_digest,
        reservation.reservation_digest
    );
    assert_eq!(item.expires_at, expires_at);
    assert_eq!(item.status, "expired");
    assert!(!item.replayed);
    assert!(item.failure_code.is_none());
    assert!(item.error.is_none());

    let finish = item.broker_finish.as_ref().unwrap();
    assert_eq!(finish.action, ComputeBrokerFinishAction::Expire);
    assert_eq!(finish.status, "expired");
    assert_eq!(finish.budget_reservation_id, budget_reservation_id);
    assert_eq!(finish.budget_refunded_fen, 10);
    assert_eq!(finish.capacity_claim.claim_id, child_claim_id);
    assert_eq!(finish.capacity_claim.claim_revision, 2);
    assert_eq!(finish.terminal_job.job_id, fixture.quoted.job.job_id);
    assert!(!finish.replayed);

    assert_eq!(fixture.balance(), 100);
    assert_eq!(fixture.capacity(), ((100, 0), (4, 0)));
    assert_eq!(
        billing_state(&fixture, &budget_reservation_id),
        ("expired_released".into(), 10, 10, 0)
    );
    assert_eq!(
        claim_state(&fixture, &child_claim_id),
        ("expired".into(), 2)
    );
    let terminal_reservation = fixture
        .supply
        .store
        .compute_reservation(&reservation_id)
        .unwrap();
    assert_eq!(terminal_reservation.reservation.status, "expired");
    assert_eq!(terminal_reservation.revision, finish.reservation_revision);
    assert_eq!(
        terminal_reservation.reservation_digest,
        finish.reservation_digest
    );
    assert_eq!(
        terminal_reservation.reservation.capacity_claim,
        finish.capacity_claim
    );
    assert_eq!(terminal_reservation.reservation.job, finish.terminal_job);
    let terminal_job = fixture
        .supply
        .store
        .compute_job(&fixture.quoted.job.job_id)
        .unwrap();
    assert_eq!(terminal_job.job.status, "failed");
    assert_eq!(terminal_job.revision, finish.terminal_job.job_revision);
    assert_finish_receipt(&fixture, &reservation_id, &expires_at);

    let allocation = get_for_consumer(
        &fixture.supply.store,
        &fixture.consumer_id,
        &grant.grant.grant_id,
    )
    .unwrap();
    assert_eq!(allocation.current_status, "exercised");
    let commitment = capacity_commitment_service::get_for_owner(
        &fixture.supply.store,
        &fixture.supply.owner_id,
        &fixture.supply.provider_id,
        &fixture.supply.pool_id,
        &fixture.commitment.commitment.commitment_id,
    )
    .unwrap();
    assert_eq!(commitment.current_status, "allocated");

    let second = fixture
        .supply
        .store
        .expire_due_compute_delivery_allocation_reservations(expire_request(10))
        .unwrap();
    assert_eq!(second.selected_count, 0);
    assert_eq!(second.expired_count, 0);
    assert_eq!(second.replayed_count, 0);
    assert!(second.items.is_empty());
    assert_eq!(fixture.balance(), 100);
    assert_eq!(fixture.capacity(), ((100, 0), (4, 0)));
    assert_eq!(fixture.table_count("compute_broker_finish_receipts"), 1);
    fixture.cleanup();
}

fn expire_request(limit: usize) -> ExpireDueComputeDeliveryAllocationReservations {
    ExpireDueComputeDeliveryAllocationReservations {
        limit,
        confirmation: COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION.into(),
    }
}

fn advance_store_clock_past(expires_at: &str) -> crate::store::TestNowOverrideGuard {
    let target = DateTime::parse_from_rfc3339(expires_at)
        .unwrap()
        .with_timezone(&Utc);
    crate::store::override_now_for_test(&(target + Duration::seconds(1)).to_rfc3339()).unwrap()
}

fn claim_state(fixture: &Fixture, claim_id: &str) -> (String, i64) {
    fixture
        .supply
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT status, revision FROM compute_capacity_claims WHERE claim_id=?1",
            params![claim_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

fn billing_state(fixture: &Fixture, reservation_id: &str) -> (String, i64, i64, i64) {
    fixture
        .supply
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT status, reserved_fen, refunded_fen, settled_cost_fen
               FROM billing_reservations WHERE id=?1",
            params![reservation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
}

fn assert_finish_receipt(fixture: &Fixture, reservation_id: &str, expires_at: &str) {
    let row: (String, String, i64, String, Option<String>, Option<String>) = fixture
        .supply
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT terminal_action, budget_terminal_status, budget_refunded_fen,
                    occurred_at, start_resolution_proof_id, start_resolution_proof_digest
               FROM compute_broker_finish_receipts WHERE reservation_id=?1",
            params![reservation_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(row.0, "expire");
    assert_eq!(row.1, "expired_released");
    assert_eq!(row.2, 10);
    assert_eq!(row.3, expires_at);
    assert!(row.4.is_none());
    assert!(row.5.is_none());
}
