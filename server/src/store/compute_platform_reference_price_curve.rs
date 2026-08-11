//! Store-private proposal, review, atomic application, and v171 binding authority for a governed
//! platform reference fallback curve.
//!
//! Submit and review do not register a Price Snapshot, create a Job, reserve capacity, move funds,
//! or prove an external market observation.

mod apply;
mod canonical;
mod query;
mod read;
mod review;
mod snapshot;
mod submit;
mod types;

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

#[cfg(test)]
#[path = "compute_platform_reference_price_curve_tests.rs"]
mod tests;
