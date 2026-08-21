use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    ComputeCapacityMeterMode, ComputeCapacityMeterPolicy, ComputeCapacityPoolBinding,
    COMPUTE_CAPACITY_POOL_SCHEMA,
};
use crate::compute_federation::federation_historical_causal_reference::{
    AttemptLeaseSourceRef, AttemptSettlementRef, CapacityClaimVersionRef, CapacityPoolVersionRef,
    ExecutionReceiptRef, ExecutionSourceLineageV1, FinalizationRef, JobVersionRef, OfferVersionRef,
    PriceSnapshotRef, ProviderVersionRef, ReservationVersionRef, SettlementSourceLineageV1,
};
use crate::compute_federation::market::ComputeDeliveryWindowBinding;
use crate::store::compute_capacity_pool_queries::audit_legacy_compute_capacity_pool_digests;

use super::super::source_refs::{ExecutionSourceLinkFacts, SettlementSourceLinkFacts};

pub(super) fn audit_legacy_pool(
    binding: &ComputeCapacityPoolBinding,
    meter_policies: &[ComputeCapacityMeterPolicy],
) -> Result<()> {
    audit_legacy_compute_capacity_pool_digests(
        binding,
        "provider-a",
        "scope-digest-a",
        "profile-digest-a",
        "zone-a",
        meter_policies,
    )
}

pub(super) fn legacy_pool_facts() -> (ComputeCapacityPoolBinding, Vec<ComputeCapacityMeterPolicy>) {
    let mut meter_policies = vec![ComputeCapacityMeterPolicy {
        meter: "gpu_second".to_string(),
        meter_mode: ComputeCapacityMeterMode::Consumable,
        quantum_units: 1,
        policy_digest: String::new(),
    }];
    meter_policies[0].policy_digest = legacy_test_digest(&serde_json::json!({
        "meter": "gpu_second",
        "meter_mode": "consumable",
        "quantum_units": 1,
    }));

    let mut binding = ComputeCapacityPoolBinding {
        pool_id: "pool-a".to_string(),
        capacity_epoch: 1,
        pool_revision: 1,
        pool_digest: String::new(),
    };
    binding.pool_digest = legacy_test_digest(&serde_json::json!({
        "schema": COMPUTE_CAPACITY_POOL_SCHEMA,
        "pool_id": binding.pool_id.as_str(),
        "capacity_epoch": binding.capacity_epoch,
        "pool_revision": binding.pool_revision,
        "provider_id": "provider-a",
        "resource_scope_digest": "scope-digest-a",
        "resource_profile_digest": "profile-digest-a",
        "region_or_data_zone": "zone-a",
        "meter_policies": &meter_policies,
    }));
    (binding, meter_policies)
}

fn legacy_test_digest(value: &serde_json::Value) -> String {
    hex::encode(Sha256::digest(serde_json::to_vec(value).unwrap()))
}

pub(super) fn execution_facts() -> ExecutionSourceLinkFacts {
    let provider = provider_ref();
    let pool = pool_ref();
    let offer = offer_ref();
    let snapshot = snapshot_ref();
    let job = job_ref(1, "job-digest-a");
    let reservation = reservation_ref(1, "reservation-digest-a");
    let claim = claim_ref();
    let lease = lease_ref();
    let execution_receipt = execution_receipt_ref();
    let delivery_window = delivery_window_ref();
    ExecutionSourceLinkFacts {
        lineage: ExecutionSourceLineageV1 {
            execution_receipt,
            provider: provider.clone(),
            capacity_pool: pool.clone(),
            offer: offer.clone(),
            price_snapshot: snapshot.clone(),
            job: job.clone(),
            reservation: reservation.clone(),
            capacity_claim: claim.clone(),
            attempt_lease_source: lease.clone(),
        },
        audited_execution_receipt: execution_receipt_ref(),
        audited_provider: provider.clone(),
        offer_provider: provider.clone(),
        audited_pool: pool.clone(),
        pool_from_offer: pool.clone(),
        pool_from_claim: pool,
        pool_provider_id: provider.provider_id.clone(),
        snapshot_provider_id: provider.provider_id.clone(),
        audited_offer: offer.clone(),
        snapshot_offer: offer.clone(),
        job_offer: offer.clone(),
        job_price_snapshot_id: snapshot.price_snapshot_id.clone(),
        reservation_job: job.clone(),
        reservation_offer: offer.clone(),
        reservation_snapshot: snapshot,
        reservation_claim: claim.clone(),
        claim_delivery_window: delivery_window.clone(),
        snapshot_delivery_window: delivery_window.clone(),
        offer_delivery_windows: vec![delivery_window],
        candidate_provider_id: provider.provider_id.clone(),
        candidate_job: job.clone(),
        candidate_reservation: reservation.clone(),
        candidate_claim: claim.clone(),
        candidate_lease: lease.clone(),
        verification_provider_id: provider.provider_id.clone(),
        verification_job: job,
        verification_reservation: reservation,
        verification_claim: claim,
        verification_lease: lease.clone(),
        audited_lease: lease.clone(),
        receipt_job_id: "job-a".to_string(),
        receipt_reservation_id: "reservation-a".to_string(),
        receipt_lease_id: lease.lease_id,
        receipt_provider_id: provider.provider_id.clone(),
        receipt_offer: offer,
        receipt_attempt_no: 1,
        receipt_fencing_generation: 1,
        receipt_executor_id: "executor-a".to_string(),
        activation_job_id: "job-a".to_string(),
        activation_job: job_ref(1, "job-digest-a"),
        activation_reservation_id: "reservation-a".to_string(),
        activation_reservation: reservation_ref(1, "reservation-digest-a"),
        activation_claim: claim_ref(),
        activation_provider_id: provider.provider_id,
        activation_attempt_no: 1,
        activation_fencing_generation: 1,
        activation_executor_id: "executor-a".to_string(),
    }
}

pub(super) fn settlement_facts() -> SettlementSourceLinkFacts {
    let execution_receipt = execution_receipt_ref();
    let finalization = finalization_ref();
    let snapshot = snapshot_ref();
    let provider = provider_ref();
    let finalization_source_job = job_ref(2, "running-job-digest-a");
    let source_job = job_ref(3, "source-job-digest-a");
    let terminal_job = job_ref(4, "terminal-job-digest-a");
    let terminal_reservation = reservation_ref(2, "terminal-reservation-digest-a");
    let attempt_settlement = AttemptSettlementRef {
        settlement_receipt_id: "settlement-a".to_string(),
        settlement_receipt_digest: "settlement-digest-a".to_string(),
        settlement_event_digest: "settlement-event-a".to_string(),
    };
    SettlementSourceLinkFacts {
        lineage: SettlementSourceLineageV1 {
            attempt_settlement: attempt_settlement.clone(),
            execution_receipt: execution_receipt.clone(),
            execution_lineage_digest: "lineage-digest-a".to_string(),
            finalization: finalization.clone(),
            price_snapshot: snapshot.clone(),
            provider: provider.clone(),
            source_job: source_job.clone(),
            terminal_job: terminal_job.clone(),
            terminal_reservation: terminal_reservation.clone(),
        },
        audited_attempt_settlement: attempt_settlement,
        rebuilt_execution_receipt: execution_receipt.clone(),
        rebuilt_execution_lineage_digest: "lineage-digest-a".to_string(),
        settlement_execution_receipt: execution_receipt.clone(),
        audited_finalization: finalization,
        finalization_execution_receipt: execution_receipt,
        finalization_provider_id: provider.provider_id.clone(),
        finalization_source_job,
        finalization_terminal_job: source_job.clone(),
        finalization_terminal_reservation: terminal_reservation,
        settlement_price_snapshot: snapshot,
        audited_provider: provider.clone(),
        settlement_provider: provider.clone(),
        execution_provider_id: provider.provider_id,
        settlement_source_job: source_job,
        settlement_terminal_job: terminal_job,
        settlement_reservation_id: "reservation-a".to_string(),
        execution_reservation_id: "reservation-a".to_string(),
        settlement_lease_id: "lease-a".to_string(),
        execution_lease_id: "lease-a".to_string(),
        finalization_lease_id: "lease-a".to_string(),
        source_job_status: "verification_pending".to_string(),
        terminal_job_status: "settled".to_string(),
        settlement_balance_state: "pending".to_string(),
    }
}

fn provider_ref() -> ProviderVersionRef {
    ProviderVersionRef {
        provider_id: "provider-a".to_string(),
        policy_revision: 1,
        provider_digest: "provider-digest-a".to_string(),
    }
}

fn pool_ref() -> CapacityPoolVersionRef {
    CapacityPoolVersionRef {
        pool_id: "pool-a".to_string(),
        capacity_epoch: 1,
        pool_revision: 1,
        pool_digest: "pool-digest-a".to_string(),
    }
}

fn offer_ref() -> OfferVersionRef {
    OfferVersionRef {
        provider_id: "provider-a".to_string(),
        offer_id: "offer-a".to_string(),
        offer_version: 1,
        offer_digest: "offer-digest-a".to_string(),
    }
}

fn snapshot_ref() -> PriceSnapshotRef {
    PriceSnapshotRef {
        price_snapshot_id: "snapshot-a".to_string(),
        price_snapshot_digest: "snapshot-digest-a".to_string(),
    }
}

fn delivery_window_ref() -> ComputeDeliveryWindowBinding {
    ComputeDeliveryWindowBinding {
        window_id: "window-a".to_string(),
        window_digest: "window-digest-a".to_string(),
    }
}

fn job_ref(job_revision: u64, job_digest: &str) -> JobVersionRef {
    JobVersionRef {
        job_id: "job-a".to_string(),
        job_revision,
        job_digest: job_digest.to_string(),
    }
}

fn reservation_ref(reservation_revision: u64, reservation_digest: &str) -> ReservationVersionRef {
    ReservationVersionRef {
        reservation_id: "reservation-a".to_string(),
        reservation_revision,
        reservation_digest: reservation_digest.to_string(),
    }
}

fn claim_ref() -> CapacityClaimVersionRef {
    CapacityClaimVersionRef {
        claim_id: "claim-a".to_string(),
        claim_revision: 1,
        claim_digest: "claim-digest-a".to_string(),
    }
}

fn lease_ref() -> AttemptLeaseSourceRef {
    AttemptLeaseSourceRef {
        lease_id: "lease-a".to_string(),
        lease_revision: 1,
        lease_digest: "lease-digest-a".to_string(),
        fencing_generation: 1,
    }
}

fn execution_receipt_ref() -> ExecutionReceiptRef {
    ExecutionReceiptRef {
        execution_receipt_id: "execution-a".to_string(),
        execution_receipt_digest: "execution-digest-a".to_string(),
    }
}

fn finalization_ref() -> FinalizationRef {
    FinalizationRef {
        finalization_id: "finalization-a".to_string(),
        finalization_event_digest: "finalization-event-a".to_string(),
    }
}
