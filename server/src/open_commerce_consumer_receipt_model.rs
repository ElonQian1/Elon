use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const CONSUMER_RECEIPT_SCHEMA: &str = "open_commerce.consumer_invocation_receipt.v1";
pub(crate) const CONSUMER_RECEIPT_PAYLOAD_SCHEMA: &str =
    "open_commerce.consumer_invocation_receipt_payload.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerInvocationReceiptPayload {
    pub schema: String,
    pub invocation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_app_id: String,
    pub request_shape: ConsumerInvocationRequestShape,
    pub status: String,
    pub result: Option<Value>,
    pub error_code: Option<String>,
    pub units: i64,
    pub unit_price_micros: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub funds_moved: bool,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerInvocationReceipt {
    pub schema: String,
    pub payload_sha256: String,
    pub payload_json: String,
    pub payload: ConsumerInvocationReceiptPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerInvocationRequestShape {
    pub input_fields: Vec<String>,
    pub input_bytes: u64,
    pub contains_raw_values: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerInvocationReceiptSummary {
    pub invocation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_app_id: String,
    pub status: String,
    pub result_available: bool,
    pub error_code: Option<String>,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub funds_moved: bool,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerInvocationReceiptList {
    pub schema: &'static str,
    pub scope: &'static str,
    pub receipts: Vec<ConsumerInvocationReceiptSummary>,
}
