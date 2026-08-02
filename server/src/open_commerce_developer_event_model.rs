use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct DeveloperTerminalEventQuery {
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperTerminalEventPage {
    pub schema: &'static str,
    pub app_id: String,
    pub events: Vec<DeveloperTerminalEventSummary>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperTerminalEventSummary {
    pub schema: &'static str,
    pub event_id: String,
    pub event_type: &'static str,
    pub invocation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub idempotency_key: String,
    pub status: String,
    pub result_available: bool,
    pub error_code: Option<String>,
    pub units: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub funds_moved: bool,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperTerminalEventDetail {
    pub schema: &'static str,
    pub event: DeveloperTerminalEventSummary,
    pub result: Option<Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct DeveloperTerminalEventRecord {
    pub sequence: i64,
    pub invocation: crate::open_commerce_model::OpenCommerceInvocation,
}
