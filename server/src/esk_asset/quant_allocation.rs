use serde::{Deserialize, Serialize};

use super::format_esk_amount;

pub(crate) const ESK_QUANT_RISK_DISCLOSURE_REVISION: &str = "esk-quant-paper-allocation-v2";
pub(crate) const ESK_QUANT_REQUEST_CONFIRMATION: &str = "REQUEST PAPER ESK QUANT ALLOCATION";
pub(crate) const ESK_QUANT_CANCEL_CONFIRMATION: &str = "CANCEL PAPER ESK QUANT ALLOCATION";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskQuantAllocationInput {
    pub user_id: String,
    pub amount_base_units: i64,
    pub idempotency_key: String,
    pub risk_disclosure_revision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct EskQuantAllocationRecord {
    pub request_id: String,
    pub user_id: String,
    pub amount_base_units: i64,
    pub idempotency_key: String,
    pub risk_disclosure_revision: String,
    pub status: String,
    pub revision: i64,
    pub submitted_at: String,
    pub updated_at: String,
    pub replayed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateEskQuantAllocationBody {
    pub amount: String,
    pub idempotency_key: String,
    pub risk_disclosure_revision: String,
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelEskQuantAllocationBody {
    pub confirmation: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EskQuantAllocationListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub(crate) struct EskQuantAllocationView {
    pub request_id: String,
    pub amount: String,
    pub amount_base_units: String,
    pub risk_disclosure_revision: String,
    pub status: String,
    pub revision: i64,
    pub submitted_at: String,
    pub updated_at: String,
    pub simulated: bool,
    pub funds_moved: bool,
    pub position_created: bool,
    pub replayed: bool,
}

pub(crate) fn view(record: EskQuantAllocationRecord) -> EskQuantAllocationView {
    EskQuantAllocationView {
        request_id: record.request_id,
        amount: format_esk_amount(record.amount_base_units),
        amount_base_units: record.amount_base_units.to_string(),
        risk_disclosure_revision: record.risk_disclosure_revision,
        status: record.status,
        revision: record.revision,
        submitted_at: record.submitted_at,
        updated_at: record.updated_at,
        simulated: true,
        funds_moved: false,
        position_created: false,
        replayed: record.replayed,
    }
}
