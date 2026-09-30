use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferenceQuote {
    pub quote_id: String,
    pub source: String,
    pub observed_at_ms: i64,
    pub valid_until_ms: i64,
    pub usdt_per_esk_base_units: String,
    pub cny_per_usdt_base_units: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ReferenceValuation {
    pub usdt_base_units: String,
    pub cny_base_units: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CenterAsset {
    pub total_base_units: String,
    pub reserved_base_units: String,
    pub remaining_base_units: String,
    pub entry_count: String,
    pub snapshot_digest: String,
    pub entries: Vec<CenterPurchase>,
    pub history_next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CenterPurchase {
    pub entry_id: String,
    pub allocation_id: String,
    pub amount_base_units: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UsageSource {
    pub billing_source: String,
    pub total_tokens: String,
    pub input_tokens: String,
    pub cached_input_tokens: String,
    pub output_tokens: String,
    pub call_count: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CenterBill {
    pub id: String,
    pub token_usage_event_id: Option<String>,
    pub task_reference: Option<String>,
    pub feature: Option<String>,
    pub model: Option<String>,
    pub input_tokens: String,
    pub cached_input_tokens: String,
    pub output_tokens: String,
    pub cost_fen: String,
    pub price_rule_version: Option<i64>,
    pub price_source: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CenterHold {
    pub id: String,
    pub task_reference: String,
    pub feature: String,
    pub model: Option<String>,
    pub reserved_fen: String,
    pub status: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CenterSnapshot {
    pub asset: CenterAsset,
    pub legacy_balance_fen: Option<String>,
    pub month_cost_fen: String,
    pub usage_sources: Vec<UsageSource>,
    pub bills: Vec<CenterBill>,
    pub bills_has_more: bool,
    pub holds: Vec<CenterHold>,
    pub holds_has_more: bool,
}
