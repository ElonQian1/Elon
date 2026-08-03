use serde::{Deserialize, Serialize};

use super::{
    capacity::ComputeCapacityClaimBinding,
    market::ComputePriceSnapshot,
    workload::{ComputeArtifactRef, ComputeWorkloadSpec},
};

pub(crate) const COMPUTE_JOB_SCHEMA: &str = "compute_federation.job.v1";
pub(crate) const COMPUTE_RESERVATION_SCHEMA: &str = "compute_federation.reservation.v1";
pub(crate) const COMPUTE_ATTEMPT_LEASE_SCHEMA: &str = "compute_federation.attempt_lease.v1";

pub(crate) const JOB_STATUS_SUBMITTED: &str = "submitted";
pub(crate) const JOB_STATUS_QUOTED: &str = "quoted";
pub(crate) const JOB_STATUS_RESERVED: &str = "reserved";
pub(crate) const JOB_STATUS_RUNNING: &str = "running";
pub(crate) const JOB_STATUS_VERIFICATION_PENDING: &str = "verification_pending";
pub(crate) const JOB_STATUS_SETTLED: &str = "settled";
pub(crate) const JOB_STATUS_FAILED: &str = "failed";
pub(crate) const JOB_STATUS_CANCELED: &str = "canceled";

pub(crate) const RESERVATION_STATUS_PENDING: &str = "pending";
pub(crate) const RESERVATION_STATUS_ACTIVE: &str = "active";
pub(crate) const RESERVATION_STATUS_CONSUMED: &str = "consumed";
pub(crate) const RESERVATION_STATUS_RELEASED: &str = "released";
pub(crate) const RESERVATION_STATUS_EXPIRED: &str = "expired";

pub(crate) const ATTEMPT_STATUS_OFFERED: &str = "offered";
pub(crate) const ATTEMPT_STATUS_STAGING: &str = "staging";
pub(crate) const ATTEMPT_STATUS_RUNNING: &str = "running";
pub(crate) const ATTEMPT_STATUS_RESULT_REPORTED: &str = "result_reported";
pub(crate) const ATTEMPT_STATUS_VERIFYING: &str = "verifying";
pub(crate) const ATTEMPT_STATUS_TERMINAL: &str = "terminal";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProviderScope {
    pub allowed_provider_ids: Vec<String>,
    pub allowed_provider_kinds: Vec<String>,
    pub excluded_provider_ids: Vec<String>,
    pub required_trust_tier: String,
    pub required_regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOfferBinding {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeJobVersionBinding {
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
}

/// Stable demand identity. Individual machine executions are attempt leases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeJob {
    pub schema: String,
    pub job_id: String,
    pub project_id: Option<String>,
    pub merchant_id: Option<String>,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub workload: ComputeWorkloadSpec,
    pub provider_scope: ComputeProviderScope,
    pub status: String,
    pub selected_offer: Option<ComputeOfferBinding>,
    pub price_snapshot_id: Option<String>,
    pub max_consumer_charge_micros: i64,
    pub currency: String,
    pub submitted_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeReservedCapacity {
    pub meter: String,
    pub quantity: i64,
}

/// Atomically binds capacity, consumer authorization and an immutable price.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeReservation {
    pub schema: String,
    pub reservation_id: String,
    pub job: ComputeJobVersionBinding,
    pub idempotency_key: String,
    pub offer: ComputeOfferBinding,
    pub price_snapshot: ComputePriceSnapshot,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub reserved_capacity: Vec<ComputeReservedCapacity>,
    pub consumer_authorization_ref: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCheckpointRef {
    pub checkpoint_id: String,
    pub artifact: ComputeArtifactRef,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub created_at: String,
}

/// The credential itself is issued out-of-band; this model contains only a reference and hint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeAttemptLease {
    pub schema: String,
    pub lease_id: String,
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_no: i64,
    pub shard_id: Option<String>,
    pub provider_id: String,
    pub executor_id: String,
    pub status: String,
    pub fencing_generation: i64,
    pub lease_credential_ref: String,
    pub lease_credential_hint: String,
    pub latest_checkpoint: Option<ComputeCheckpointRef>,
    pub issued_at: String,
    pub last_heartbeat_at: Option<String>,
    pub expires_at: String,
    pub hard_deadline_at: String,
    pub terminal_reason_code: Option<String>,
}
