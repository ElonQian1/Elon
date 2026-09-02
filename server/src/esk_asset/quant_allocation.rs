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
    pub binding_id: Option<String>,
    pub receipt_id: Option<String>,
    pub receipt_digest: Option<String>,
    pub receipt_key_id: Option<String>,
    pub quant_binding_revision: Option<i64>,
    pub occurred_at_unix: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EskQuantAllocationReceiptInput {
    pub user_id: String,
    pub participant_ref: String,
    pub request_id: String,
    pub amount_base_units: i64,
    pub risk_disclosure_revision: String,
    pub event: String,
    pub binding_id: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub receipt_key_id: String,
    pub previous_receipt_digest: Option<String>,
    pub quant_binding_revision: i64,
    pub occurred_at_unix: i64,
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
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyEskQuantAllocationReceiptBody {
    pub receipt_token: String,
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
    pub allocation_binding_created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quant_binding_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurred_at_unix: Option<i64>,
    pub replayed: bool,
}

pub(crate) fn view(record: EskQuantAllocationRecord) -> EskQuantAllocationView {
    let allocation_binding_created = matches!(record.status.as_str(), "accepted" | "released");
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
        allocation_binding_created,
        binding_id: record.binding_id,
        quant_binding_revision: record.quant_binding_revision,
        occurred_at_unix: record.occurred_at_unix,
        replayed: record.replayed,
    }
}
