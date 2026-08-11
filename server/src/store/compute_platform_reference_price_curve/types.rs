use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::compute_federation::platform_reference_price_curve::{
    ComputePlatformReferencePriceCurveBatchEnvelope,
    ComputePlatformReferencePriceCurveEntryEnvelope, ComputePlatformReferencePriceCurveEntryIntent,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA,
};

pub(super) const PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA: &str =
    "compute_federation.platform_reference_price_curve_review.v1";
pub(super) const PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA: &str =
    "compute_federation.platform_reference_price_curve_application.v1";
pub(super) const PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA: &str =
    "compute_federation.platform_reference_price_curve_snapshot_binding.v1";
pub(super) const PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION: &str =
    "confirm_platform_reference_price_curve_review";
pub(super) const PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION: &str =
    "confirm_platform_reference_price_curve_apply";

pub(super) const BATCH_STATUS_SUBMITTED: &str = "submitted";
pub(super) const REVIEW_DECISION_APPROVED: &str = "approved";
pub(super) const REVIEW_DECISION_CHANGES_REQUESTED: &str = "changes_requested";
pub(super) const REVIEW_DECISION_REJECTED: &str = "rejected";
pub(super) const APPLICATION_STATUS_APPLIED: &str = "applied";
pub(super) const SNAPSHOT_BINDING_STATUS_REGISTERED: &str = "snapshot_registered";
pub(in crate::store) struct SubmitComputePlatformReferencePriceCurveBatch {
    pub submitted_by_admin_user_id: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub methodology_kind: String,
    pub valid_from: String,
    pub valid_until: String,
    pub quote_ttl_seconds: i64,
    pub rounding_mode: String,
    pub entries: Vec<ComputePlatformReferencePriceCurveEntryIntent>,
    pub idempotency_key: String,
    pub confirmation: String,
    pub submission_note: String,
    pub idempotency_scope: String,
}
pub(in crate::store) struct ReviewComputePlatformReferencePriceCurveBatch {
    pub batch_id: String,
    pub expected_batch_digest: String,
    pub expected_batch_material_digest: String,
    pub decision: String,
    pub review_confirmation: String,
    pub review_note: Option<String>,
    pub reviewed_by_admin_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}
pub(in crate::store) struct ApplyComputePlatformReferencePriceCurveBatch {
    pub batch_id: String,
    pub expected_batch_digest: String,
    pub expected_batch_material_digest: String,
    pub expected_review_id: String,
    pub expected_review_digest: String,
    pub applied_by_admin_user_id: String,
    pub apply_confirmation: String,
    pub apply_note: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}
#[derive(Clone, Serialize)]
pub(in crate::store) struct ComputePlatformReferencePriceCurveEntryReceipt {
    pub schema: &'static str,
    pub batch_id: String,
    pub batch_digest: String,
    pub entry_id: String,
    pub entry_digest: String,
    pub ordinal: i64,
    pub entry_key: String,
    pub offer_id: String,
    pub offer_version: i64,
}
#[derive(Clone, Serialize)]
pub(in crate::store) struct ComputePlatformReferencePriceCurveBatchReceipt {
    pub schema: &'static str,
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub entry_set_digest: String,
    pub entries: Vec<ComputePlatformReferencePriceCurveEntryReceipt>,
    pub status: String,
    pub submitted_by_admin_user_id: String,
    pub submitted_at: String,
    pub replayed: bool,
    pub market_effect: &'static str,
}
#[derive(Clone, Serialize)]
pub(in crate::store) struct ComputePlatformReferencePriceCurveReviewReceipt {
    pub schema: &'static str,
    pub review_id: String,
    pub review_digest: String,
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub entry_set_digest: String,
    pub decision: String,
    pub reviewed_by_admin_user_id: String,
    pub reviewed_at: String,
    pub replayed: bool,
    pub market_effect: &'static str,
}
#[derive(Clone, Serialize)]
pub(in crate::store) struct ComputePlatformReferencePriceCurveSnapshotBindingReceipt {
    pub schema: &'static str,
    pub binding_id: String,
    pub binding_digest: String,
    pub application_id: String,
    pub batch_id: String,
    pub review_id: String,
    pub entry_id: String,
    pub entry_digest: String,
    pub ordinal: i64,
    pub snapshot_id: String,
    pub snapshot_digest: String,
    pub quote_id: String,
    pub source_kind: String,
    pub source_id: String,
    pub source_version: i64,
    pub source_digest: String,
    pub quoted_at: String,
    pub expires_at: String,
    pub status: String,
}
#[derive(Clone, Serialize)]
pub(in crate::store) struct ComputePlatformReferencePriceCurveApplicationReceipt {
    pub schema: &'static str,
    pub application_id: String,
    pub application_digest: String,
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub binding_set_digest: String,
    pub bindings: Vec<ComputePlatformReferencePriceCurveSnapshotBindingReceipt>,
    pub submitted_by_admin_user_id: String,
    pub reviewed_by_admin_user_id: String,
    pub applied_by_admin_user_id: String,
    pub status: String,
    pub applied_at: String,
    pub replayed: bool,
    pub market_effect: &'static str,
    pub job_effect: &'static str,
    pub reservation_effect: &'static str,
    pub capacity_effect: &'static str,
    pub funds_effect: &'static str,
    pub settlement_effect: &'static str,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredReviewEnvelope {
    pub schema: String,
    pub review_id: String,
    pub review_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub review: StoredReviewMaterial,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredReviewMaterial {
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub entry_set_digest: String,
    pub decision: String,
    pub review_confirmation: String,
    pub review_note: Option<String>,
    pub reviewed_by_admin_user_id: String,
    pub reviewed_at: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredApplicationEnvelope {
    pub schema: String,
    pub application_id: String,
    pub application_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub application: StoredApplicationMaterial,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredApplicationMaterial {
    pub batch_id: String,
    pub batch_digest: String,
    pub batch_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub binding_digests: Vec<String>,
    pub binding_set_digest: String,
    pub submitted_by_admin_user_id: String,
    pub reviewed_by_admin_user_id: String,
    pub applied_by_admin_user_id: String,
    pub apply_confirmation: String,
    pub apply_note: String,
    pub applied_at: String,
    pub status: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredSnapshotBindingEnvelope {
    pub schema: String,
    pub binding_id: String,
    pub binding_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub binding: StoredSnapshotBindingMaterial,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredSnapshotBindingMaterial {
    pub application_id: String,
    pub batch_id: String,
    pub batch_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub entry_id: String,
    pub entry_digest: String,
    pub ordinal: i64,
    pub entry_key: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub snapshot_id: String,
    pub snapshot_digest: String,
    pub quote_id: String,
    pub source_kind: String,
    pub source_id: String,
    pub source_version: i64,
    pub source_digest: String,
    pub quoted_at: String,
    pub expires_at: String,
    pub status: String,
}
pub(super) struct StoredBatch {
    pub envelope: ComputePlatformReferencePriceCurveBatchEnvelope,
    pub batch_json: String,
    pub status: String,
    pub reviewed_by_admin_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub applied_by_admin_user_id: Option<String>,
    pub applied_at: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}
pub(super) struct StoredEntry {
    pub envelope: ComputePlatformReferencePriceCurveEntryEnvelope,
    pub entry_json: String,
    pub components_json: String,
    pub fee_rules_json: String,
}
pub(super) struct StoredReview {
    pub envelope: StoredReviewEnvelope,
    pub review_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}
pub(super) struct StoredApplication {
    pub envelope: StoredApplicationEnvelope,
    pub application_json: String,
    pub binding_digests_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}
pub(super) struct StoredSnapshotBinding {
    pub envelope: StoredSnapshotBindingEnvelope,
    pub binding_json: String,
}

impl StoredEntry {
    pub(super) fn into_receipt(self) -> ComputePlatformReferencePriceCurveEntryReceipt {
        let entry = self.envelope.entry;
        ComputePlatformReferencePriceCurveEntryReceipt {
            schema: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA,
            batch_id: self.envelope.batch_id,
            batch_digest: self.envelope.batch_digest,
            entry_id: self.envelope.entry_id,
            entry_digest: self.envelope.entry_digest,
            ordinal: self.envelope.ordinal,
            entry_key: entry.entry_key,
            offer_id: entry.offer_id,
            offer_version: entry.offer_version,
        }
    }
}

impl StoredBatch {
    pub(super) fn into_receipt(
        self,
        entries: Vec<ComputePlatformReferencePriceCurveEntryReceipt>,
        replayed: bool,
    ) -> ComputePlatformReferencePriceCurveBatchReceipt {
        let batch = self.envelope.batch;
        ComputePlatformReferencePriceCurveBatchReceipt {
            schema: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA,
            batch_id: self.envelope.batch_id,
            batch_digest: self.envelope.batch_digest,
            batch_material_digest: self.envelope.batch_material_digest,
            curve_id: batch.curve_id,
            curve_version: batch.curve_version,
            entry_set_digest: batch.entry_set_digest,
            entries,
            status: self.status,
            submitted_by_admin_user_id: batch.submitted_by_admin_user_id,
            submitted_at: batch.submitted_at,
            replayed,
            market_effect: "none",
        }
    }
}

impl StoredReview {
    pub(super) fn into_receipt(
        self,
        replayed: bool,
    ) -> ComputePlatformReferencePriceCurveReviewReceipt {
        let review = self.envelope.review;
        ComputePlatformReferencePriceCurveReviewReceipt {
            schema: PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_SCHEMA,
            review_id: self.envelope.review_id,
            review_digest: self.envelope.review_digest,
            batch_id: review.batch_id,
            batch_digest: review.batch_digest,
            batch_material_digest: review.batch_material_digest,
            curve_id: review.curve_id,
            curve_version: review.curve_version,
            entry_set_digest: review.entry_set_digest,
            decision: review.decision,
            reviewed_by_admin_user_id: review.reviewed_by_admin_user_id,
            reviewed_at: review.reviewed_at,
            replayed,
            market_effect: "none",
        }
    }
}

impl StoredSnapshotBinding {
    pub(super) fn into_receipt(self) -> ComputePlatformReferencePriceCurveSnapshotBindingReceipt {
        let binding = self.envelope.binding;
        ComputePlatformReferencePriceCurveSnapshotBindingReceipt {
            schema: PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA,
            binding_id: self.envelope.binding_id,
            binding_digest: self.envelope.binding_digest,
            application_id: binding.application_id,
            batch_id: binding.batch_id,
            review_id: binding.review_id,
            entry_id: binding.entry_id,
            entry_digest: binding.entry_digest,
            ordinal: binding.ordinal,
            snapshot_id: binding.snapshot_id,
            snapshot_digest: binding.snapshot_digest,
            quote_id: binding.quote_id,
            source_kind: binding.source_kind,
            source_id: binding.source_id,
            source_version: binding.source_version,
            source_digest: binding.source_digest,
            quoted_at: binding.quoted_at,
            expires_at: binding.expires_at,
            status: binding.status,
        }
    }
}

impl StoredApplication {
    pub(super) fn into_receipt(
        self,
        bindings: Vec<ComputePlatformReferencePriceCurveSnapshotBindingReceipt>,
        replayed: bool,
    ) -> ComputePlatformReferencePriceCurveApplicationReceipt {
        let application = self.envelope.application;
        ComputePlatformReferencePriceCurveApplicationReceipt {
            schema: PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA,
            application_id: self.envelope.application_id,
            application_digest: self.envelope.application_digest,
            batch_id: application.batch_id,
            batch_digest: application.batch_digest,
            batch_material_digest: application.batch_material_digest,
            review_id: application.review_id,
            review_digest: application.review_digest,
            curve_id: application.curve_id,
            curve_version: application.curve_version,
            binding_set_digest: application.binding_set_digest,
            bindings,
            submitted_by_admin_user_id: application.submitted_by_admin_user_id,
            reviewed_by_admin_user_id: application.reviewed_by_admin_user_id,
            applied_by_admin_user_id: application.applied_by_admin_user_id,
            status: application.status,
            applied_at: application.applied_at,
            replayed,
            market_effect: "quote_candidate_enabled",
            job_effect: "none",
            reservation_effect: "none",
            capacity_effect: "none",
            funds_effect: "none",
            settlement_effect: "none",
        }
    }
}

pub(super) fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("platform reference price curve timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
