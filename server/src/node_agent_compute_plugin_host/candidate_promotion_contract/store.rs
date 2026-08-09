use super::{
    AuthorizedCandidatePromotion, CandidatePromotionOutcomeUncertainCustody,
    CandidatePromotionStoreFailure, CandidatePromotionStorePhase,
    ComputePluginCandidatePromotionRecoveryKey, DurableInstalledPluginSlot,
    ValidatedCandidatePromotionStorePermit,
};

/// Consumes the sole promotion authority and lets Store borrow only a short-lived validated
/// permit plus the already sealed canonical receipt pair. No caller can retain a raw permit,
/// duplicate receipt custody, or use this boundary to start a runtime or admit work.
pub(in crate::node_agent_compute_plugin_host) fn persist_authorized_candidate_promotion<'root>(
    authorized: AuthorizedCandidatePromotion<'root, '_>,
) -> std::result::Result<DurableInstalledPluginSlot<'root>, CandidatePromotionStoreFailure<'root>> {
    let recovery_key = ComputePluginCandidatePromotionRecoveryKey::from_authorized(&authorized);
    let store_result = {
        let permit = ValidatedCandidatePromotionStorePermit::new(&authorized);
        authorized
            .authority_session()
            .persist_candidate_promotion(permit)
    };
    if let Err(error) = store_result {
        let (revalidated, _) = authorized.into_parts();
        return Err(CandidatePromotionStoreFailure::new(
            CandidatePromotionStorePhase::StoreOutcomeUncertain,
            error,
            CandidatePromotionOutcomeUncertainCustody::new(revalidated, recovery_key),
        ));
    }
    let (revalidated, receipts) = authorized.into_parts();
    Ok(DurableInstalledPluginSlot::new(revalidated, receipts))
}
