use anyhow::{bail, Context, Result};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimBinding, ComputeCapacityPoolBinding},
    execution::{ComputeJobVersionBinding, ComputeOfferBinding},
    federation_historical_causal_reference::{
        AttemptLeaseSourceRef, AttemptSettlementRef, CapacityClaimVersionRef,
        CapacityPoolVersionRef, ExecutionReceiptRef, ExecutionSourceLineageV1, FinalizationRef,
        JobVersionRef, OfferVersionRef, PriceSnapshotRef, ProviderVersionRef,
        ReservationVersionRef, SettlementSourceLineageV1,
    },
    market::ComputeDeliveryWindowBinding,
};

#[derive(Clone)]
pub(super) struct ExecutionSourceLinkFacts {
    pub(super) lineage: ExecutionSourceLineageV1,
    pub(super) audited_execution_receipt: ExecutionReceiptRef,
    pub(super) audited_provider: ProviderVersionRef,
    pub(super) offer_provider: ProviderVersionRef,
    pub(super) audited_pool: CapacityPoolVersionRef,
    pub(super) pool_from_offer: CapacityPoolVersionRef,
    pub(super) pool_from_claim: CapacityPoolVersionRef,
    pub(super) pool_provider_id: String,
    pub(super) snapshot_provider_id: String,
    pub(super) audited_offer: OfferVersionRef,
    pub(super) snapshot_offer: OfferVersionRef,
    pub(super) job_offer: OfferVersionRef,
    pub(super) job_price_snapshot_id: String,
    pub(super) reservation_job: JobVersionRef,
    pub(super) reservation_offer: OfferVersionRef,
    pub(super) reservation_snapshot: PriceSnapshotRef,
    pub(super) reservation_claim: CapacityClaimVersionRef,
    pub(super) claim_delivery_window: ComputeDeliveryWindowBinding,
    pub(super) snapshot_delivery_window: ComputeDeliveryWindowBinding,
    pub(super) offer_delivery_windows: Vec<ComputeDeliveryWindowBinding>,
    pub(super) candidate_provider_id: String,
    pub(super) candidate_job: JobVersionRef,
    pub(super) candidate_reservation: ReservationVersionRef,
    pub(super) candidate_claim: CapacityClaimVersionRef,
    pub(super) candidate_lease: AttemptLeaseSourceRef,
    pub(super) verification_provider_id: String,
    pub(super) verification_job: JobVersionRef,
    pub(super) verification_reservation: ReservationVersionRef,
    pub(super) verification_claim: CapacityClaimVersionRef,
    pub(super) verification_lease: AttemptLeaseSourceRef,
    pub(super) audited_lease: AttemptLeaseSourceRef,
    pub(super) receipt_job_id: String,
    pub(super) receipt_reservation_id: String,
    pub(super) receipt_lease_id: String,
    pub(super) receipt_provider_id: String,
    pub(super) receipt_offer: OfferVersionRef,
    pub(super) receipt_attempt_no: u64,
    pub(super) receipt_fencing_generation: u64,
    pub(super) receipt_executor_id: String,
    pub(super) activation_job_id: String,
    pub(super) activation_job: JobVersionRef,
    pub(super) activation_reservation_id: String,
    pub(super) activation_reservation: ReservationVersionRef,
    pub(super) activation_claim: CapacityClaimVersionRef,
    pub(super) activation_provider_id: String,
    pub(super) activation_attempt_no: u64,
    pub(super) activation_fencing_generation: u64,
    pub(super) activation_executor_id: String,
}

pub(super) fn validate_execution_source_links(facts: &ExecutionSourceLinkFacts) -> Result<()> {
    let lineage = &facts.lineage;
    if lineage.execution_receipt != facts.audited_execution_receipt
        || lineage.provider != facts.audited_provider
        || lineage.provider != facts.offer_provider
        || lineage.capacity_pool != facts.audited_pool
        || lineage.capacity_pool != facts.pool_from_offer
        || lineage.capacity_pool != facts.pool_from_claim
        || facts.pool_provider_id != lineage.provider.provider_id
        || facts.snapshot_provider_id != lineage.provider.provider_id
        || lineage.offer != facts.audited_offer
        || lineage.offer != facts.snapshot_offer
        || lineage.offer != facts.job_offer
        || lineage.offer != facts.reservation_offer
        || facts.job_price_snapshot_id != lineage.price_snapshot.price_snapshot_id
        || lineage.price_snapshot != facts.reservation_snapshot
        || lineage.job != facts.reservation_job
        || lineage.job != facts.candidate_job
        || lineage.job != facts.verification_job
        || lineage.reservation != facts.candidate_reservation
        || lineage.reservation != facts.verification_reservation
        || lineage.capacity_claim != facts.reservation_claim
        || lineage.capacity_claim != facts.candidate_claim
        || lineage.capacity_claim != facts.verification_claim
        || lineage.attempt_lease_source != facts.candidate_lease
        || lineage.attempt_lease_source != facts.verification_lease
        || lineage.attempt_lease_source != facts.audited_lease
        || facts.claim_delivery_window != facts.snapshot_delivery_window
        || !facts
            .offer_delivery_windows
            .iter()
            .any(|window| window == &facts.snapshot_delivery_window)
    {
        bail!(
            "execution source 的 Provider/Pool/Offer/Snapshot/Job/Reservation/Claim/Lease 链不一致"
        );
    }
    if facts.candidate_provider_id != lineage.provider.provider_id
        || facts.verification_provider_id != lineage.provider.provider_id
        || facts.receipt_job_id != lineage.job.job_id
        || facts.receipt_reservation_id != lineage.reservation.reservation_id
        || facts.receipt_lease_id != lineage.attempt_lease_source.lease_id
        || facts.receipt_provider_id != lineage.provider.provider_id
        || facts.receipt_offer != lineage.offer
        || facts.receipt_fencing_generation != lineage.attempt_lease_source.fencing_generation
        || facts.activation_job_id != lineage.job.job_id
        || facts.activation_job != lineage.job
        || facts.activation_reservation_id != lineage.reservation.reservation_id
        || facts.activation_reservation != lineage.reservation
        || facts.activation_claim != lineage.capacity_claim
        || facts.activation_provider_id != lineage.provider.provider_id
        || facts.activation_fencing_generation != lineage.attempt_lease_source.fencing_generation
        || facts.activation_attempt_no != facts.receipt_attempt_no
        || facts.activation_executor_id != facts.receipt_executor_id
    {
        bail!("execution source 的 v185/v189/v192/v193 身份或 fencing 链不一致");
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct SettlementSourceLinkFacts {
    pub(super) lineage: SettlementSourceLineageV1,
    pub(super) audited_attempt_settlement: AttemptSettlementRef,
    pub(super) rebuilt_execution_receipt: ExecutionReceiptRef,
    pub(super) rebuilt_execution_lineage_digest: String,
    pub(super) settlement_execution_receipt: ExecutionReceiptRef,
    pub(super) audited_finalization: FinalizationRef,
    pub(super) finalization_execution_receipt: ExecutionReceiptRef,
    pub(super) finalization_provider_id: String,
    pub(super) finalization_source_job: JobVersionRef,
    pub(super) finalization_terminal_job: JobVersionRef,
    pub(super) finalization_terminal_reservation: ReservationVersionRef,
    pub(super) settlement_price_snapshot: PriceSnapshotRef,
    pub(super) audited_provider: ProviderVersionRef,
    pub(super) settlement_provider: ProviderVersionRef,
    pub(super) execution_provider_id: String,
    pub(super) settlement_source_job: JobVersionRef,
    pub(super) settlement_terminal_job: JobVersionRef,
    pub(super) settlement_reservation_id: String,
    pub(super) execution_reservation_id: String,
    pub(super) settlement_lease_id: String,
    pub(super) execution_lease_id: String,
    pub(super) finalization_lease_id: String,
    pub(super) source_job_status: String,
    pub(super) terminal_job_status: String,
    pub(super) settlement_balance_state: String,
}

pub(super) fn validate_settlement_source_links(facts: &SettlementSourceLinkFacts) -> Result<()> {
    let lineage = &facts.lineage;
    if lineage.attempt_settlement != facts.audited_attempt_settlement
        || lineage.execution_receipt != facts.rebuilt_execution_receipt
        || lineage.execution_receipt != facts.settlement_execution_receipt
        || lineage.execution_lineage_digest != facts.rebuilt_execution_lineage_digest
        || lineage.finalization != facts.audited_finalization
        || lineage.execution_receipt != facts.finalization_execution_receipt
        || facts.finalization_provider_id != lineage.provider.provider_id
        || lineage.source_job != facts.finalization_terminal_job
        || lineage.source_job != facts.settlement_source_job
        || lineage.terminal_job != facts.settlement_terminal_job
        || lineage.terminal_reservation != facts.finalization_terminal_reservation
        || lineage.price_snapshot != facts.settlement_price_snapshot
        || lineage.provider != facts.audited_provider
        || lineage.provider != facts.settlement_provider
        || facts.execution_provider_id != lineage.provider.provider_id
        || lineage.terminal_reservation.reservation_id != facts.settlement_reservation_id
        || facts.settlement_reservation_id != facts.execution_reservation_id
        || facts.settlement_lease_id != facts.execution_lease_id
        || facts.settlement_lease_id != facts.finalization_lease_id
        || facts.finalization_source_job.job_id != lineage.source_job.job_id
        || facts.finalization_source_job.job_revision.checked_add(1)
            != Some(lineage.source_job.job_revision)
        || lineage.source_job.job_id != lineage.terminal_job.job_id
        || lineage.source_job.job_revision.checked_add(1) != Some(lineage.terminal_job.job_revision)
        || facts.source_job_status != "verification_pending"
        || facts.terminal_job_status != "settled"
        || facts.settlement_balance_state != "pending"
    {
        bail!("settlement source 的 v193/v194/v195 cross-link 不一致");
    }
    Ok(())
}

pub(super) fn provider_ref(
    provider_id: &str,
    policy_revision: i64,
    provider_digest: &str,
) -> Result<ProviderVersionRef> {
    Ok(ProviderVersionRef {
        provider_id: provider_id.to_string(),
        policy_revision: positive_u64("Provider policy revision", policy_revision)?,
        provider_digest: provider_digest.to_string(),
    })
}

pub(super) fn pool_ref(binding: &ComputeCapacityPoolBinding) -> Result<CapacityPoolVersionRef> {
    Ok(CapacityPoolVersionRef {
        pool_id: binding.pool_id.clone(),
        capacity_epoch: positive_u64("Capacity Pool epoch", binding.capacity_epoch)?,
        pool_revision: positive_u64("Capacity Pool revision", binding.pool_revision)?,
        pool_digest: binding.pool_digest.clone(),
    })
}

pub(super) fn offer_ref(binding: &ComputeOfferBinding) -> Result<OfferVersionRef> {
    Ok(OfferVersionRef {
        provider_id: binding.provider_id.clone(),
        offer_id: binding.offer_id.clone(),
        offer_version: positive_u64("Offer version", binding.offer_version)?,
        offer_digest: binding.offer_digest.clone(),
    })
}

pub(super) fn price_snapshot_ref(
    price_snapshot_id: &str,
    price_snapshot_digest: &str,
) -> PriceSnapshotRef {
    PriceSnapshotRef {
        price_snapshot_id: price_snapshot_id.to_string(),
        price_snapshot_digest: price_snapshot_digest.to_string(),
    }
}

pub(super) fn job_ref(binding: &ComputeJobVersionBinding) -> Result<JobVersionRef> {
    Ok(JobVersionRef {
        job_id: binding.job_id.clone(),
        job_revision: positive_u64("Job revision", binding.job_revision)?,
        job_digest: binding.job_digest.clone(),
    })
}

pub(super) fn reservation_ref(
    reservation_id: &str,
    reservation_revision: i64,
    reservation_digest: &str,
) -> Result<ReservationVersionRef> {
    Ok(ReservationVersionRef {
        reservation_id: reservation_id.to_string(),
        reservation_revision: positive_u64("Reservation revision", reservation_revision)?,
        reservation_digest: reservation_digest.to_string(),
    })
}

pub(super) fn claim_ref(binding: &ComputeCapacityClaimBinding) -> Result<CapacityClaimVersionRef> {
    Ok(CapacityClaimVersionRef {
        claim_id: binding.claim_id.clone(),
        claim_revision: positive_u64("Capacity Claim revision", binding.claim_revision)?,
        claim_digest: binding.claim_digest.clone(),
    })
}

pub(super) fn lease_ref(
    lease_id: &str,
    lease_revision: i64,
    lease_digest: &str,
    fencing_generation: i64,
) -> Result<AttemptLeaseSourceRef> {
    Ok(AttemptLeaseSourceRef {
        lease_id: lease_id.to_string(),
        lease_revision: positive_u64("Attempt Lease revision", lease_revision)?,
        lease_digest: lease_digest.to_string(),
        fencing_generation: positive_u64("Attempt Lease fencing generation", fencing_generation)?,
    })
}

pub(super) fn execution_receipt_ref(
    execution_receipt_id: &str,
    execution_receipt_digest: &str,
) -> ExecutionReceiptRef {
    ExecutionReceiptRef {
        execution_receipt_id: execution_receipt_id.to_string(),
        execution_receipt_digest: execution_receipt_digest.to_string(),
    }
}

pub(super) fn finalization_ref(
    finalization_id: &str,
    finalization_event_digest: &str,
) -> FinalizationRef {
    FinalizationRef {
        finalization_id: finalization_id.to_string(),
        finalization_event_digest: finalization_event_digest.to_string(),
    }
}

pub(super) fn settlement_ref(
    settlement_receipt_id: &str,
    settlement_receipt_digest: &str,
    settlement_event_digest: &str,
) -> AttemptSettlementRef {
    AttemptSettlementRef {
        settlement_receipt_id: settlement_receipt_id.to_string(),
        settlement_receipt_digest: settlement_receipt_digest.to_string(),
        settlement_event_digest: settlement_event_digest.to_string(),
    }
}

pub(super) fn positive_u64(label: &str, value: i64) -> Result<u64> {
    u64::try_from(value)
        .with_context(|| format!("{label} must be a positive federation historical integer"))
        .and_then(|value| {
            if value == 0 {
                bail!("{label} must be a positive federation historical integer");
            }
            Ok(value)
        })
}
