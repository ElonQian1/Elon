//! Store-private proposal, review, atomic application, and v171 binding authority for a governed
//! platform reference fallback curve.
//!
//! No service or API is wired here. Submit and review do not register a Price Snapshot, create a
//! Job, reserve capacity, move funds, or prove an external market observation.

mod apply;
mod canonical;
mod read;
mod review;
mod snapshot;
mod submit;
mod types;

pub(super) use types::{
    ApplyComputePlatformReferencePriceCurveBatch,
    ComputePlatformReferencePriceCurveApplicationReceipt,
    ComputePlatformReferencePriceCurveBatchReceipt,
    ComputePlatformReferencePriceCurveReviewReceipt, ReviewComputePlatformReferencePriceCurveBatch,
    SubmitComputePlatformReferencePriceCurveBatch,
};
