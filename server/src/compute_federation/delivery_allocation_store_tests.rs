use chrono::{DateTime, Duration, Utc};

use crate::{
    compute_federation::{
        capacity_commitment_service::{self, test_support::Fixture as CapacityCommitmentFixture},
        execution::ComputeProviderScope,
    },
    compute_federation_broker_service::{
        self, control_plane_tests::workload, CreateMyComputeJobRequest, QuoteMyComputeJobRequest,
    },
    store::{
        ComputeCapacityCommitmentCreateReceipt, ComputeDeliveryAllocationGrantWriteReceipt,
        ComputeJobRegistrationReceipt,
    },
};

use super::{
    create_for_provider_owner, decline_for_consumer, exercise_for_consumer, get_for_consumer,
    CreateDeliveryAllocationGrantBody, DeclineDeliveryAllocationGrantBody,
    ExerciseDeliveryAllocationGrantBody,
};

#[test]
fn exercise_moves_the_whole_claim_once_and_replays_without_double_charge() {
    let fixture = Fixture::new();
    assert_eq!(fixture.capacity(), ((80, 20), (3, 1)));

    let denied = fixture.create_grant("primary", false).unwrap_err();
    assert!(denied.to_string().contains("显式确认"));
    assert_eq!(fixture.table_count("compute_delivery_allocation_grants"), 0);

    let grant = fixture.create_grant("primary", true).unwrap();
    assert!(!grant.replayed);
    assert_eq!(grant.grant.grant_status, "granted");
    assert_eq!(grant.grant.job.job_id, fixture.quoted.job.job_id);
    let replayed_grant = fixture.create_grant("primary", true).unwrap();
    assert!(replayed_grant.replayed);
    assert_eq!(replayed_grant.grant.grant_id, grant.grant.grant_id);

    fixture.recharge(100);
    let reservation_id = fixture.reservation_id("primary");
    let exercised = fixture
        .exercise(&grant, &reservation_id, "primary", true)
        .unwrap();
    assert!(!exercised.replayed);
    assert_eq!(exercised.terminal_receipt.terminal_status, "exercised");
    let evidence = exercised.terminal_receipt.exercise.as_ref().unwrap();
    assert_eq!(evidence.parent_result_claim_state, "released");
    assert_eq!(evidence.parent_result_claim_revision, 2);
    assert_eq!(evidence.reservation_claim.claim_revision, 1);
    assert_eq!(
        evidence.reservation_claim.parent_claim_id,
        evidence.parent_claim_id
    );
    assert_eq!(
        evidence.parent_release_ledger.event_kind,
        "reservation_released"
    );
    assert_eq!(
        evidence.reservation_hold_ledger.event_kind,
        "reservation_held"
    );
    assert_eq!(
        evidence.reservation_hold_ledger.causal_transaction_id,
        evidence.parent_release_ledger.transaction_id
    );
    assert_eq!(evidence.reservation.reservation_id, reservation_id);
    assert_eq!(evidence.reserved_amount_fen, 10);
    assert_eq!(fixture.capacity(), ((80, 20), (3, 1)));
    assert_eq!(fixture.balance(), 90);

    let commitment = capacity_commitment_service::get_for_owner(
        &fixture.supply.store,
        &fixture.supply.owner_id,
        &fixture.supply.provider_id,
        &fixture.supply.pool_id,
        &fixture.commitment.commitment.commitment_id,
    )
    .unwrap();
    assert_eq!(commitment.current_status, "allocated");
    let reservation = compute_federation_broker_service::get_reservation_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &reservation_id,
    )
    .unwrap();
    assert_eq!(reservation.reservation.status, "active");
    let job = compute_federation_broker_service::get_job_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &fixture.quoted.job.job_id,
    )
    .unwrap();
    assert_eq!(job.job.status, "reserved");

    let replayed = fixture
        .exercise(&grant, &reservation_id, "primary", true)
        .unwrap();
    assert!(replayed.replayed);
    assert_eq!(
        replayed.terminal_receipt.terminal_receipt_id,
        exercised.terminal_receipt.terminal_receipt_id
    );
    assert_eq!(fixture.balance(), 90);
    assert_eq!(fixture.table_count("compute_delivery_allocation_grants"), 1);
    assert_eq!(
        fixture.table_count("compute_delivery_allocation_terminal_receipts"),
        1
    );
    fixture.cleanup();
}

#[test]
fn insufficient_balance_rolls_back_claim_job_reservation_and_terminal() {
    let fixture = Fixture::new();
    let grant = fixture.create_grant("rollback", true).unwrap();
    fixture.recharge(5);
    let reservation_id = fixture.reservation_id("rollback");

    let error = fixture
        .exercise(&grant, &reservation_id, "rollback", true)
        .unwrap_err();
    assert!(format!("{error:#}").contains("余额不足"));
    assert_eq!(fixture.balance(), 5);
    assert_eq!(fixture.capacity(), ((80, 20), (3, 1)));
    assert_eq!(
        fixture.table_count("compute_delivery_allocation_terminal_receipts"),
        0
    );
    assert!(compute_federation_broker_service::get_reservation_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &reservation_id,
    )
    .is_err());
    let detail = get_for_consumer(
        &fixture.supply.store,
        &fixture.consumer_id,
        &grant.grant.grant_id,
    )
    .unwrap();
    assert_eq!(detail.current_status, "granted");
    let job = compute_federation_broker_service::get_job_for_user(
        &fixture.supply.store,
        &fixture.consumer_id,
        Some(&fixture.project_id),
        &fixture.quoted.job.job_id,
    )
    .unwrap();
    assert_eq!(job.job.status, "quoted");
    fixture.cleanup();
}

#[test]
fn decline_is_idempotent_and_does_not_move_capacity_or_budget() {
    let fixture = Fixture::new();
    let grant = fixture.create_grant("decline", true).unwrap();
    let body = DeclineDeliveryAllocationGrantBody {
        idempotency_key: "decline-terminal".into(),
        expected_grant_revision: grant.grant.grant_revision,
        expected_grant_digest: grant.grant.grant_digest.clone(),
        confirm_decline: true,
    };
    let declined = decline_for_consumer(
        &fixture.supply.store,
        &fixture.consumer_id,
        &grant.grant.grant_id,
        body.clone(),
    )
    .unwrap();
    assert!(!declined.replayed);
    assert_eq!(declined.terminal_receipt.terminal_status, "declined");
    assert!(declined.terminal_receipt.exercise.is_none());
    let replayed = decline_for_consumer(
        &fixture.supply.store,
        &fixture.consumer_id,
        &grant.grant.grant_id,
        body,
    )
    .unwrap();
    assert!(replayed.replayed);
    assert_eq!(fixture.capacity(), ((80, 20), (3, 1)));
    assert_eq!(fixture.balance(), 0);
    fixture.cleanup();
}

struct Fixture {
    supply: CapacityCommitmentFixture,
    consumer_id: String,
    project_id: String,
    commitment: ComputeCapacityCommitmentCreateReceipt,
    quoted: ComputeJobRegistrationReceipt,
}

impl Fixture {
    fn new() -> Self {
        let supply = CapacityCommitmentFixture::new_delivery_allocation();
        let commitment = capacity_commitment_service::create_for_owner(
            &supply.store,
            &supply.owner_id,
            &supply.provider_id,
            &supply.pool_id,
            supply.create_body("delivery-allocation-commitment", true),
        )
        .unwrap();
        let consumer = supply
            .store
            .create_user(
                &format!(
                    "delivery-allocation-{}@example.com",
                    uuid::Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let project_id = supply
            .store
            .create_project(&consumer.id, "Delivery allocation project", None, None)
            .unwrap()
            .project
            .id;
        let window = supply.offer.delivery_windows.first().unwrap();
        let deadline = DateTime::parse_from_rfc3339(&window.ends_at_utc)
            .unwrap()
            .with_timezone(&Utc)
            - Duration::seconds(1);
        let mut workload = workload();
        workload.deadline_at = deadline.to_rfc3339();
        let created = compute_federation_broker_service::create_job_for_project(
            &supply.store,
            &consumer.id,
            &project_id,
            CreateMyComputeJobRequest {
                job_id: format!("delivery-job-{}", consumer.id),
                idempotency_key: "delivery-job-create".into(),
                merchant_id: None,
                workload,
                provider_scope: ComputeProviderScope {
                    allowed_provider_ids: vec![supply.provider_id.clone()],
                    allowed_provider_kinds: vec!["user_node".into()],
                    excluded_provider_ids: Vec::new(),
                    required_trust_tier: "platform_verified".into(),
                    required_regions: vec!["cn-east".into()],
                },
                max_consumer_charge_micros: 100_000,
                currency: "CNY".into(),
            },
        )
        .unwrap();
        let candidates = compute_federation_broker_service::list_quote_candidates_for_project(
            &supply.store,
            &consumer.id,
            &project_id,
            &created.job.job_id,
            20,
        )
        .unwrap();
        let candidate = candidates
            .candidates
            .iter()
            .find(|candidate| {
                candidate.offer.offer_id == supply.offer.offer_id
                    && candidate.price_snapshot.snapshot_id == supply.binding.snapshot_id
            })
            .unwrap();
        let quoted = compute_federation_broker_service::quote_job_for_project(
            &supply.store,
            &consumer.id,
            &project_id,
            &created.job.job_id,
            QuoteMyComputeJobRequest {
                offer_id: candidate.offer.offer_id.clone(),
                price_snapshot_id: candidate.price_snapshot.snapshot_id.clone(),
                expected_job_revision: created.revision,
                expected_job_digest: created.job_digest,
            },
        )
        .unwrap();
        Self {
            supply,
            consumer_id: consumer.id,
            project_id,
            commitment,
            quoted,
        }
    }

    fn create_grant(
        &self,
        suffix: &str,
        confirm_grant: bool,
    ) -> anyhow::Result<ComputeDeliveryAllocationGrantWriteReceipt> {
        create_for_provider_owner(
            &self.supply.store,
            &self.supply.owner_id,
            &self.supply.provider_id,
            &self.supply.pool_id,
            &self.commitment.commitment.commitment_id,
            CreateDeliveryAllocationGrantBody {
                idempotency_key: format!("grant-{suffix}"),
                expected_commitment_revision: self.commitment.commitment.commitment_revision,
                expected_commitment_digest: self.commitment.commitment.commitment_digest.clone(),
                consumer_account_id: self.consumer_id.clone(),
                job_id: self.quoted.job.job_id.clone(),
                expected_job_revision: self.quoted.revision,
                expected_job_digest: self.quoted.job_digest.clone(),
                confirm_grant,
            },
        )
    }

    fn exercise(
        &self,
        grant: &ComputeDeliveryAllocationGrantWriteReceipt,
        reservation_id: &str,
        suffix: &str,
        confirm_financial_action: bool,
    ) -> anyhow::Result<crate::store::ComputeDeliveryAllocationExerciseWriteReceipt> {
        exercise_for_consumer(
            &self.supply.store,
            &self.consumer_id,
            &grant.grant.grant_id,
            ExerciseDeliveryAllocationGrantBody {
                reservation_id: reservation_id.into(),
                idempotency_key: format!("exercise-{suffix}"),
                expected_grant_revision: grant.grant.grant_revision,
                expected_grant_digest: grant.grant.grant_digest.clone(),
                confirm_financial_action,
            },
        )
    }

    fn recharge(&self, amount_fen: i64) {
        self.supply
            .store
            .billing_recharge(
                &self.consumer_id,
                amount_fen,
                "delivery_allocation_test",
                &self.supply.admin_id,
                None,
            )
            .unwrap();
    }

    fn balance(&self) -> i64 {
        self.supply
            .store
            .billing_get_balance(&self.consumer_id)
            .unwrap()
            .unwrap_or(0)
    }

    fn capacity(&self) -> ((i64, i64), (i64, i64)) {
        (
            self.supply.balance(&self.supply.token_bucket_id),
            self.supply.balance(&self.supply.concurrency_bucket_id),
        )
    }

    fn reservation_id(&self, suffix: &str) -> String {
        format!("delivery-reservation-{suffix}-{}", self.consumer_id)
    }

    fn table_count(&self, table: &str) -> i64 {
        self.supply
            .store
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn cleanup(self) {
        self.supply.cleanup();
    }
}
