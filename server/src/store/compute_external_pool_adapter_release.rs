//! Store-private external-pool Adapter release request, review, and staged admission ledger.
//!
//! No service or API is wired here. A staged admission does not mint v213 authority, resolve an
//! artifact, verify a credential verifier, or make an Adapter executable.

mod apply;
mod canonical;
mod read;
mod review;
mod submit;
mod types;

#[cfg(test)]
#[path = "compute_external_pool_adapter_release_tests.rs"]
mod tests;

pub(super) use types::{
    ApplyExternalPoolAdapterRelease, ExternalPoolAdapterReleaseAdmissionReceipt,
    ExternalPoolAdapterReleaseRequestReceipt, ExternalPoolAdapterReleaseReviewReceipt,
    ReviewExternalPoolAdapterReleaseRequest, SubmitExternalPoolAdapterReleaseRequest,
};
