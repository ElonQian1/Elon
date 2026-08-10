//! Store-private external-pool onboarding request, review, and application ledger.
//!
//! No service or API is wired in this batch. These facades do not mint v213 route authority.

mod apply;
mod canonical;
mod read;
mod review;
mod submit;
mod types;

pub(super) use types::{
    ApplyExternalPoolOnboarding, ExternalPoolOnboardingApplicationReceipt,
    ExternalPoolOnboardingRequestReceipt, ExternalPoolOnboardingReviewReceipt,
    ReviewExternalPoolOnboardingRequest, SubmitExternalPoolOnboardingRequest,
};
