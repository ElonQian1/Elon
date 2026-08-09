use anyhow::{bail, Context, Result};

use crate::node_agent_compute_plugin_host::candidate_promotion_contract::{
    CandidatePromotionReceiptPair, ComputePluginInstallReceipt, ComputePluginPromotionReceipt,
    HashedComputePluginInstallReceipt, HashedComputePluginPromotionReceipt,
};

pub(super) fn decode_receipt_pair(
    install_json: &str,
    install_digest: &str,
    promotion_json: &str,
    promotion_digest: &str,
) -> Result<CandidatePromotionReceiptPair> {
    let install_body: ComputePluginInstallReceipt = serde_json::from_str(install_json)
        .context("COMPUTE_PLUGIN_CANDIDATE_INSTALL_RECEIPT_DECODE")?;
    let promotion_body: ComputePluginPromotionReceipt = serde_json::from_str(promotion_json)
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_DECODE")?;
    if serde_json::to_string(&install_body)? != install_json
        || serde_json::to_string(&promotion_body)? != promotion_json
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_NOT_CANONICAL_BODY");
    }
    let install = HashedComputePluginInstallReceipt::from_store_readback(
        install_body,
        install_digest.to_string(),
    )?;
    let promotion = HashedComputePluginPromotionReceipt::from_store_readback(
        promotion_body,
        promotion_digest.to_string(),
    )?;
    CandidatePromotionReceiptPair::new(install, promotion)
}
