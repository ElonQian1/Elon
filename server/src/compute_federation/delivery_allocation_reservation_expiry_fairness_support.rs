use chrono::{DateTime, Duration, Utc};

use crate::{
    compute_federation::{capacity_commitment_service, execution::ComputeProviderScope},
    compute_federation_broker_service::{
        self, control_plane_tests::workload, CreateMyComputeJobRequest, QuoteMyComputeJobRequest,
    },
};

use super::{
    super::{
        create_for_provider_owner, exercise_for_consumer, CreateDeliveryAllocationGrantBody,
        ExerciseDeliveryAllocationGrantBody,
    },
    Fixture,
};

pub(super) struct AdditionalReservation {
    pub(super) reservation_id: String,
    pub(super) expires_at: String,
}

pub(super) fn exercise_additional_reservation(
    fixture: &Fixture,
    suffix: &str,
) -> AdditionalReservation {
    let commitment = capacity_commitment_service::create_for_owner(
        &fixture.supply.store,
        &fixture.supply.owner_id,
        &fixture.supply.provider_id,
        &fixture.supply.pool_id,
        fixture
            .supply
            .create_body(&format!("commitment-{suffix}"), true),
    )
    .unwrap();

    let window = fixture.supply.offer.delivery_windows.first().unwrap();
    let mut requested_workload = workload();
    requested_workload.deadline_at = (DateTime::parse_from_rfc3339(&window.ends_at_utc)
        .unwrap()
        .with_timezone(&Utc)
        - Duration::seconds(1))
    .to_rfc3339();
    let created = compute_federation_broker_service::create_job_for_project(
        &fixture.supply.store,
        &fixture.consumer_id,
        &fixture.project_id,
        CreateMyComputeJobRequest {
            job_id: format!("delivery-job-{suffix}-{}", fixture.consumer_id),
            idempotency_key: format!("delivery-job-create-{suffix}"),
            merchant_id: None,
            workload: requested_workload,
            provider_scope: ComputeProviderScope {
                allowed_provider_ids: vec![fixture.supply.provider_id.clone()],
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
    let candidate = compute_federation_broker_service::list_quote_candidates_for_project(
        &fixture.supply.store,
        &fixture.consumer_id,
        &fixture.project_id,
        &created.job.job_id,
        20,
    )
    .unwrap()
    .candidates
    .into_iter()
    .find(|candidate| {
        candidate.offer.offer_id == fixture.supply.offer.offer_id
            && candidate.price_snapshot.snapshot_id == fixture.supply.binding.snapshot_id
    })
    .unwrap();
    let quoted = compute_federation_broker_service::quote_job_for_project(
        &fixture.supply.store,
        &fixture.consumer_id,
        &fixture.project_id,
        &created.job.job_id,
        QuoteMyComputeJobRequest {
            offer_id: candidate.offer.offer_id,
            price_snapshot_id: candidate.price_snapshot.snapshot_id,
            expected_job_revision: created.revision,
            expected_job_digest: created.job_digest,
        },
    )
    .unwrap();
    let grant = create_for_provider_owner(
        &fixture.supply.store,
        &fixture.supply.owner_id,
        &fixture.supply.provider_id,
        &fixture.supply.pool_id,
        &commitment.commitment.commitment_id,
        CreateDeliveryAllocationGrantBody {
            idempotency_key: format!("grant-{suffix}"),
            expected_commitment_revision: commitment.commitment.commitment_revision,
            expected_commitment_digest: commitment.commitment.commitment_digest,
            consumer_account_id: fixture.consumer_id.clone(),
            job_id: quoted.job.job_id.clone(),
            expected_job_revision: quoted.revision,
            expected_job_digest: quoted.job_digest,
            confirm_grant: true,
        },
    )
    .unwrap();
    let reservation_id = fixture.reservation_id(suffix);
    exercise_for_consumer(
        &fixture.supply.store,
        &fixture.consumer_id,
        &grant.grant.grant_id,
        ExerciseDeliveryAllocationGrantBody {
            reservation_id: reservation_id.clone(),
            idempotency_key: format!("exercise-{suffix}"),
            expected_grant_revision: grant.grant.grant_revision,
            expected_grant_digest: grant.grant.grant_digest,
            confirm_financial_action: true,
        },
    )
    .unwrap();
    let expires_at = fixture
        .supply
        .store
        .compute_reservation(&reservation_id)
        .unwrap()
        .reservation
        .expires_at;
    AdditionalReservation {
        reservation_id,
        expires_at,
    }
}

pub(super) fn assert_key_is_after(
    first_expires_at: &str,
    first_reservation_id: &str,
    later_expires_at: &str,
    later_reservation_id: &str,
) {
    let first = DateTime::parse_from_rfc3339(first_expires_at).unwrap();
    let later = DateTime::parse_from_rfc3339(later_expires_at).unwrap();
    assert!(
        later > first
            || (later == first
                && (later_expires_at > first_expires_at
                    || (later_expires_at == first_expires_at
                        && later_reservation_id > first_reservation_id)))
    );
}
