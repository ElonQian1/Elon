//! Administrator-only orchestration for governed platform fallback price curve batches.

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{
    ApplyComputePlatformReferencePriceCurveBatch,
    ComputePlatformReferencePriceCurveApplicationReceipt,
    ComputePlatformReferencePriceCurveBatchDetailReceipt,
    ComputePlatformReferencePriceCurveBatchReceipt,
    ComputePlatformReferencePriceCurveReviewReceipt, ReviewComputePlatformReferencePriceCurveBatch,
    Store, SubmitComputePlatformReferencePriceCurveBatch,
    PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION,
    PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
};

use super::platform_reference_price_curve::{
    ComputePlatformReferencePriceCurveEntryIntent,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY,
    COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitPlatformReferencePriceCurveBody {
    pub idempotency_key: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub valid_from: String,
    pub valid_until: String,
    pub quote_ttl_seconds: i64,
    pub entries: Vec<ComputePlatformReferencePriceCurveEntryIntent>,
    #[serde(default)]
    pub submission_note: String,
    pub confirm_submission: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewPlatformReferencePriceCurveBody {
    pub idempotency_key: String,
    pub expected_batch_digest: String,
    pub expected_batch_material_digest: String,
    pub decision: String,
    #[serde(default)]
    pub review_note: Option<String>,
    pub confirm_review: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyPlatformReferencePriceCurveBody {
    pub idempotency_key: String,
    pub expected_batch_digest: String,
    pub expected_batch_material_digest: String,
    pub expected_review_id: String,
    pub expected_review_digest: String,
    #[serde(default)]
    pub apply_note: String,
    pub confirm_application: bool,
}

#[derive(Clone, Serialize)]
pub(crate) struct PlatformReferencePriceCurvePreflightReport {
    pub schema: &'static str,
    pub batch_id: String,
    pub batch_digest: String,
    pub curve_id: String,
    pub curve_version: i64,
    pub submitted_by_admin_user_id: String,
    pub batch_status: String,
    pub checked_at: String,
    pub entry_count: usize,
    pub review_present: bool,
    pub application_present: bool,
    pub admin_review_allowed: bool,
    pub admin_apply_allowed: bool,
    pub blockers: Vec<String>,
    pub market_effect: &'static str,
}

pub(crate) fn submit_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: SubmitPlatformReferencePriceCurveBody,
) -> Result<ComputePlatformReferencePriceCurveBatchReceipt> {
    if !body.confirm_submission {
        bail!("提交平台参考价格回退曲线前必须显式确认");
    }
    store.submit_compute_platform_reference_price_curve_batch(
        SubmitComputePlatformReferencePriceCurveBatch {
            submitted_by_admin_user_id: admin_user_id.to_string(),
            curve_id: body.curve_id,
            curve_version: body.curve_version,
            methodology_kind: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY.to_string(),
            valid_from: body.valid_from,
            valid_until: body.valid_until,
            quote_ttl_seconds: body.quote_ttl_seconds,
            rounding_mode: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE.to_string(),
            entries: body.entries,
            idempotency_key: body.idempotency_key,
            confirmation: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION.to_string(),
            submission_note: body.submission_note,
            idempotency_scope: operation_scope("submit", admin_user_id),
        },
    )
}

pub(crate) fn review_for_admin(
    store: &Store,
    admin_user_id: &str,
    batch_id: &str,
    body: ReviewPlatformReferencePriceCurveBody,
) -> Result<ComputePlatformReferencePriceCurveReviewReceipt> {
    if !body.confirm_review {
        bail!("复核平台参考价格回退曲线前必须显式确认");
    }
    store.review_compute_platform_reference_price_curve_batch(
        ReviewComputePlatformReferencePriceCurveBatch {
            batch_id: batch_id.to_string(),
            expected_batch_digest: body.expected_batch_digest,
            expected_batch_material_digest: body.expected_batch_material_digest,
            decision: body.decision,
            review_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION.to_string(),
            review_note: body.review_note,
            reviewed_by_admin_user_id: admin_user_id.to_string(),
            idempotency_scope: operation_scope("review", admin_user_id),
            idempotency_key: body.idempotency_key,
        },
    )
}

pub(crate) fn apply_for_admin(
    store: &Store,
    admin_user_id: &str,
    batch_id: &str,
    body: ApplyPlatformReferencePriceCurveBody,
) -> Result<ComputePlatformReferencePriceCurveApplicationReceipt> {
    if !body.confirm_application {
        bail!("应用平台参考价格回退曲线前必须显式确认");
    }
    store.apply_compute_platform_reference_price_curve_batch(
        ApplyComputePlatformReferencePriceCurveBatch {
            batch_id: batch_id.to_string(),
            expected_batch_digest: body.expected_batch_digest,
            expected_batch_material_digest: body.expected_batch_material_digest,
            expected_review_id: body.expected_review_id,
            expected_review_digest: body.expected_review_digest,
            applied_by_admin_user_id: admin_user_id.to_string(),
            apply_confirmation: PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION.to_string(),
            apply_note: body.apply_note,
            idempotency_scope: operation_scope("apply", admin_user_id),
            idempotency_key: body.idempotency_key,
        },
    )
}

pub(crate) fn get_for_admin(
    store: &Store,
    batch_id: &str,
) -> Result<ComputePlatformReferencePriceCurveBatchDetailReceipt> {
    store.platform_reference_price_curve_batch(batch_id)
}

pub(crate) fn list_for_admin(
    store: &Store,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputePlatformReferencePriceCurveBatchDetailReceipt>> {
    store.list_platform_reference_price_curve_batches_for_admin(status, limit)
}

pub(crate) fn preflight_for_admin(
    store: &Store,
    admin_user_id: &str,
    batch_id: &str,
) -> Result<PlatformReferencePriceCurvePreflightReport> {
    preflight(get_for_admin(store, batch_id)?, admin_user_id)
}

fn preflight(
    detail: ComputePlatformReferencePriceCurveBatchDetailReceipt,
    admin_user_id: &str,
) -> Result<PlatformReferencePriceCurvePreflightReport> {
    let status = detail.batch.status.as_str();
    let review_approved = detail
        .review
        .as_ref()
        .is_some_and(|review| review.decision == "approved");
    let admin_review_allowed =
        status == "submitted" && detail.batch.submitted_by_admin_user_id != admin_user_id;
    let admin_apply_allowed =
        status == "approved" && review_approved && detail.application.is_none();
    let blockers = match status {
        "submitted" if !admin_review_allowed => {
            vec!["current_admin_cannot_review_own_submission".to_string()]
        }
        "submitted" | "approved" => Vec::new(),
        "changes_requested" => vec!["changes_requested_requires_new_batch".to_string()],
        "rejected" => vec!["reference_curve_batch_rejected".to_string()],
        "applied" => vec!["reference_curve_batch_already_applied".to_string()],
        _ => bail!("platform reference price curve batch status is unsupported"),
    };
    Ok(PlatformReferencePriceCurvePreflightReport {
        schema: "compute_federation.platform_reference_price_curve_preflight.v1",
        batch_id: detail.batch.batch_id,
        batch_digest: detail.batch.batch_digest,
        curve_id: detail.batch.curve_id,
        curve_version: detail.batch.curve_version,
        submitted_by_admin_user_id: detail.batch.submitted_by_admin_user_id,
        batch_status: detail.batch.status,
        checked_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
        entry_count: detail.batch.entries.len(),
        review_present: detail.review.is_some(),
        application_present: detail.application.is_some(),
        admin_review_allowed,
        admin_apply_allowed,
        blockers,
        market_effect: "none",
    })
}

fn operation_scope(operation: &str, admin_user_id: &str) -> String {
    format!("platform-reference-price-curve:{operation}:{admin_user_id}")
}
