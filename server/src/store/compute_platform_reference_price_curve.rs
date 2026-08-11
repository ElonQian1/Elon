//! Store-private proposal, review, atomic application, and v171 binding authority for a governed
//! platform reference fallback curve.
//!
//! Submit and review do not register a Price Snapshot, create a Job, reserve capacity, move funds,
//! or prove an external market observation.

use anyhow::{bail, Result};
use rusqlite::Connection;

mod apply;
mod canonical;
mod query;
mod read;
mod review;
mod snapshot;
mod submit;
mod types;

pub(super) use types::ComputePlatformReferencePriceCurveSnapshotBindingReceipt;
pub(crate) use types::{
    ApplyComputePlatformReferencePriceCurveBatch,
    ComputePlatformReferencePriceCurveApplicationReceipt,
    ComputePlatformReferencePriceCurveBatchDetailReceipt,
    ComputePlatformReferencePriceCurveBatchReceipt,
    ComputePlatformReferencePriceCurveReviewReceipt, ReviewComputePlatformReferencePriceCurveBatch,
    SubmitComputePlatformReferencePriceCurveBatch,
    PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION,
    PLATFORM_REFERENCE_PRICE_CURVE_REVIEW_CONFIRMATION,
};

pub(super) fn audited_platform_reference_snapshot_binding_on(
    conn: &Connection,
    snapshot_id: &str,
    expected_binding_id: &str,
    expected_binding_digest: &str,
) -> Result<Option<ComputePlatformReferencePriceCurveSnapshotBindingReceipt>> {
    for (label, value) in [
        ("v171 Snapshot ID", snapshot_id),
        ("v223 Snapshot binding ID", expected_binding_id),
        ("v223 Snapshot binding digest", expected_binding_digest),
    ] {
        if value.trim().is_empty() || value != value.trim() {
            bail!("{label} is empty or not exact");
        }
    }
    let binding = read::snapshot_binding_by_snapshot_on(conn, snapshot_id)?;
    if binding.as_ref().is_some_and(|binding| {
        binding.binding_id != expected_binding_id
            || binding.binding_digest != expected_binding_digest
            || binding.snapshot_id != snapshot_id
    }) {
        bail!("v223 Snapshot binding does not match the expected exact authority");
    }
    Ok(binding)
}

#[cfg(test)]
#[path = "compute_platform_reference_price_curve_tests.rs"]
mod tests;
