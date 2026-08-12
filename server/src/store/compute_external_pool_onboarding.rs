//! Store-private external-pool onboarding request, review, and application ledger.
//!
//! The owner/admin service/API may call these facades. They do not mint v213 route authority.

mod apply;
mod cancel;
mod canonical;
mod query;
mod read;
mod review;
mod submit;
mod types;

pub(in crate::store) use read::{
    current_external_pool_onboarding_application_authority_on,
    historical_external_pool_onboarding_application_authority_on,
};
pub(in crate::store) use types::{
    CurrentExternalPoolOnboardingApplicationAuthority,
    HistoricalExternalPoolOnboardingApplicationAuthority,
};

#[cfg(test)]
#[path = "compute_external_pool_onboarding_tests.rs"]
mod tests;

pub(crate) use types::{
    ApplyExternalPoolOnboarding, CancelExternalPoolOnboardingRequest,
    ExternalPoolOnboardingApplicationReceipt, ExternalPoolOnboardingDetailReceipt,
    ExternalPoolOnboardingRequestReceipt, ExternalPoolOnboardingReviewReceipt,
    ReviewExternalPoolOnboardingRequest, SubmitExternalPoolOnboardingRequest,
    EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION,
};
