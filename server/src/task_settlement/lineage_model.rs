use serde::Serialize;

use super::model::{SettlementCorrectionDetail, SettlementReceipt};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SettlementCorrectionLineage {
    pub schema: &'static str,
    pub project_id: String,
    pub requested_receipt: SettlementReceipt,
    pub requested_position: String,
    pub root_receipt: SettlementReceipt,
    pub effective_receipt: SettlementReceipt,
    pub posted_corrections: Vec<SettlementCorrectionDetail>,
    pub non_posted_corrections: Vec<SettlementCorrectionDetail>,
    pub depth: usize,
    pub effective_has_blocking_dispute: bool,
    pub shadow_only: bool,
}
