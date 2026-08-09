//! Linear installation and promotion contract for one durably healthy candidate.
//!
//! This boundary ends at durable installed-slot custody. It deliberately cannot start a runtime,
//! mint a `ReadyCapability`, or admit work.

mod adoption;
mod authorization;
mod capability;
mod receipts;
mod recovery;
mod revalidation;
mod store;

pub(in crate::node_agent_compute_plugin_host) use adoption::adopt_recovered_candidate_promotion;
pub(in crate::node_agent_compute_plugin_host) use authorization::{
    authorize_candidate_promotion, AuthorizedCandidatePromotion,
    CandidatePromotionAuthorizationFailure, ValidatedCandidatePromotionStorePermit,
};
pub(in crate::node_agent_compute_plugin_host) use capability::{
    CandidatePromotionRevalidationCustody, DurableInstalledPluginSlot,
    PendingCandidatePromotionRevalidation, RevalidatedCandidatePromotion,
};
pub(in crate::node_agent_compute_plugin_host) use receipts::{
    CandidatePromotionReceiptPair, ComputePluginActivationGenerationTransition,
    ComputePluginAuthorityRevisionTransition, ComputePluginInstallGenerationTransition,
    ComputePluginInstallReceipt, ComputePluginPreviousActiveSlot, ComputePluginPromotionReceipt,
    HashedComputePluginInstallReceipt, HashedComputePluginPromotionReceipt,
};
pub(in crate::node_agent_compute_plugin_host) use recovery::{
    CandidatePromotionOutcomeUncertainCustody, CandidatePromotionRecoveryAdoption,
    CandidatePromotionRecoveryAdoptionFailure, CandidatePromotionRecoveryAdoptionPhase,
    CandidatePromotionStoreFailure, CandidatePromotionStorePhase,
    ComputePluginCandidatePromotionExpectation, ComputePluginCandidatePromotionRecoveryKey,
    ComputePluginCandidatePromotionRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use revalidation::{
    begin_candidate_promotion_revalidation, complete_candidate_promotion_revalidation,
    CandidatePromotionRevalidationFailure, CandidatePromotionRevalidationPhase,
};
pub(in crate::node_agent_compute_plugin_host) use store::persist_authorized_candidate_promotion;

pub(super) const INSTALL_RECEIPT_SCHEMA: &str = "elon.compute_plugin.install_receipt.v1";
pub(super) const HASHED_INSTALL_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_install_receipt.v1";
pub(super) const PROMOTION_RECEIPT_SCHEMA: &str = "elon.compute_plugin.promotion_receipt.v1";
pub(super) const HASHED_PROMOTION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_promotion_receipt.v1";
pub(super) const RECEIPT_CANONICALIZATION: &str = "RFC8785-JCS";
pub(super) const RECEIPT_DIGEST_ALGORITHM: &str = "sha256";
