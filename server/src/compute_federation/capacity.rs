use serde::{Deserialize, Serialize};

use super::market::ComputeDeliveryWindowBinding;

mod reducer;

pub(crate) use reducer::{
    apply_capacity_transaction, expand_capacity_ledger_legs, validate_capacity_claim,
    validate_capacity_transaction,
};

pub(crate) const COMPUTE_CAPACITY_POOL_SCHEMA: &str = "compute_federation.capacity_pool.v1";
pub(crate) const COMPUTE_CAPACITY_BUCKET_SCHEMA: &str = "compute_federation.capacity_bucket.v1";
pub(crate) const COMPUTE_CAPACITY_CLAIM_SCHEMA: &str = "compute_federation.capacity_claim.v1";
pub(crate) const COMPUTE_CAPACITY_TRANSACTION_SCHEMA: &str =
    "compute_federation.capacity_transaction.v1";

/// One immutable revision of a shared physical capacity boundary.
///
/// `capacity_epoch` changes only after the previous supply has drained or retired. A revision
/// changes the immutable pool envelope inside one epoch; neither field contains a live balance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityPoolBinding {
    pub pool_id: String,
    pub capacity_epoch: i64,
    pub pool_revision: i64,
    pub pool_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityPoolStatus {
    Registering,
    Active,
    Draining,
    Retired,
    Quarantined,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityMeterMode {
    Consumable,
    Reusable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityMeterPolicy {
    pub meter: String,
    pub meter_mode: ComputeCapacityMeterMode,
    pub quantum_units: i64,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityPool {
    pub schema: String,
    pub binding: ComputeCapacityPoolBinding,
    pub provider_id: String,
    pub resource_scope_digest: String,
    pub status: ComputeCapacityPoolStatus,
    pub resource_profile_digest: String,
    pub region_or_data_zone: String,
    pub meter_policies: Vec<ComputeCapacityMeterPolicy>,
    pub created_at: String,
}

/// Exact bucket identity shared by Offers, Claims, ledger movements and balance projections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityBucketBinding {
    pub bucket_id: String,
    pub bucket_digest: String,
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub meter: String,
    pub meter_mode: ComputeCapacityMeterMode,
    pub quantum_units: i64,
    pub meter_policy_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityBucketStatus {
    Open,
    Closed,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityBucket {
    pub schema: String,
    pub binding: ComputeCapacityBucketBinding,
    pub status: ComputeCapacityBucketStatus,
    pub issued_units: i64,
    pub created_at: String,
}

/// Fixed ledger accounts. `Issuance` is an external source, represented as positive
/// `issued_units` in the balance projection; all other accounts must remain non-negative.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityAccount {
    Issuance,
    Available,
    Held,
    Active,
    Consumed,
    Retired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityEventKind {
    SupplyAdded,
    SupplyWithdrawn,
    ReservationHeld,
    AttemptActivated,
    AttemptReturned,
    UsageConsumed,
    ReservationReleased,
    ReservationExpired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityClaimKind {
    QuoteHold,
    Reservation,
    CapacityCommitment,
    DeliveryAllocation,
    Attempt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityClaimState {
    Pending,
    Held,
    Active,
    Consumed,
    Released,
    Expired,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityClaimLine {
    pub line_no: i64,
    pub bucket: ComputeCapacityBucketBinding,
    pub quantity_units: i64,
}

/// One Claim owns several meters in one exact pool/window. Lines are immutable; state and revision
/// are the mutable projection used for fenced release and expiry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityClaim {
    pub schema: String,
    pub claim_id: String,
    pub claim_digest: String,
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub claim_kind: ComputeCapacityClaimKind,
    pub state: ComputeCapacityClaimState,
    pub revision: i64,
    pub parent_claim_id: Option<String>,
    pub subject_kind: String,
    pub subject_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub lines: Vec<ComputeCapacityClaimLine>,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub terminal_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityOfferBinding {
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityClaimEffectBinding {
    pub claim_id: String,
    pub claim_effect: String,
    pub claim_effect_key: String,
}

/// Business identities frozen into a ledger transaction for later audit and reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityCausalBinding {
    pub offer: Option<ComputeCapacityOfferBinding>,
    pub job_id: Option<String>,
    pub reservation_id: Option<String>,
    pub attempt_lease_id: Option<String>,
    pub fencing_generation: Option<i64>,
}

/// One positive movement inside an atomic multi-meter transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityMovementLine {
    pub line_no: i64,
    pub bucket: ComputeCapacityBucketBinding,
    pub quantity_units: i64,
    pub from_account: ComputeCapacityAccount,
    pub to_account: ComputeCapacityAccount,
}

/// Append-only capacity movement. Corrections are new transactions that reference the original;
/// an existing transaction or movement is never replaced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityLedgerTransaction {
    pub schema: String,
    pub transaction_id: String,
    pub transaction_digest: String,
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub ledger_sequence: i64,
    pub event_kind: ComputeCapacityEventKind,
    pub claim_effect: Option<ComputeCapacityClaimEffectBinding>,
    pub causal_binding: ComputeCapacityCausalBinding,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub causal_transaction_id: Option<String>,
    pub movements: Vec<ComputeCapacityMovementLine>,
    pub occurred_at: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeCapacityLegRole {
    From,
    To,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityLedgerLeg {
    pub line_no: i64,
    pub leg_role: ComputeCapacityLegRole,
    pub bucket: ComputeCapacityBucketBinding,
    pub account: ComputeCapacityAccount,
    pub delta_units: i64,
}

/// Mutable, rebuildable projection used for atomic conditional updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCapacityBucketBalance {
    pub binding: ComputeCapacityBucketBinding,
    pub status: ComputeCapacityBucketStatus,
    pub issued_units: i64,
    pub available_units: i64,
    pub held_units: i64,
    pub active_units: i64,
    pub consumed_units: i64,
    pub retired_units: i64,
    pub balance_revision: i64,
    pub through_ledger_sequence: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ComputeCapacityContractError {
    EmptyClaim,
    InvalidClaimRevision(i64),
    SelfParentClaim(String),
    EmptyTransaction,
    InvalidLedgerSequence(i64),
    InvalidLineNumber(i64),
    DuplicateLineNumber(i64),
    DuplicateBucket(String),
    DuplicateMeter(String),
    NonPositiveQuantity(i64),
    InvalidQuantum {
        meter: String,
        quantum_units: i64,
        quantity_units: i64,
    },
    PoolBindingMismatch(String),
    DeliveryWindowMismatch(String),
    InvalidAccountTransition {
        event_kind: ComputeCapacityEventKind,
        from: ComputeCapacityAccount,
        to: ComputeCapacityAccount,
    },
    InvalidCausalBinding,
    MissingBucket(String),
    BucketBindingMismatch(String),
    ClosedBucket(String),
    NonMonotonicLedgerSequence {
        bucket_id: String,
        previous: i64,
        current: i64,
    },
    ArithmeticOverflow,
    NegativeBalance {
        bucket_id: String,
        account: ComputeCapacityAccount,
        balance_units: i128,
    },
    ConservationViolation {
        bucket_id: String,
        issued_units: i128,
        projected_units: i128,
    },
    ReusableConsumedBalance {
        bucket_id: String,
        consumed_units: i128,
    },
}
