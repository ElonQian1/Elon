//! Store-private external-pool Adapter release request, review, and staged admission ledger.
//!
//! The administrator service/API may call this facade. A staged admission does not mint v213
//! authority, resolve an artifact, verify a credential verifier, or make an Adapter executable.

mod apply;
mod canonical;
mod query;
mod read;
mod review;
mod submit;
mod types;

pub(in crate::store) use read::admission_by_id_on;
pub(in crate::store) use types::ExternalPoolAdapterReleaseArtifactSourceAdmission;

#[cfg(test)]
#[path = "compute_external_pool_adapter_release_tests.rs"]
mod tests;

pub(crate) use types::{
    ApplyExternalPoolAdapterRelease, ExternalPoolAdapterReleaseAdmissionReceipt,
    ExternalPoolAdapterReleaseDetailReceipt, ExternalPoolAdapterReleaseRequestReceipt,
    ExternalPoolAdapterReleaseReviewReceipt, ReviewExternalPoolAdapterReleaseRequest,
    SubmitExternalPoolAdapterReleaseRequest, EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION,
};
