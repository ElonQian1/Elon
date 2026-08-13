use serde::Serialize;
use serde_json::Value;

use crate::open_commerce_merchant_evidence_model::MerchantBusinessReceipt;

pub(crate) const CONSUMER_ORDER_CLOSURE_SCHEMA: &str = "open_commerce.consumer_order_closure.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerOrderClosure {
    pub schema: &'static str,
    pub scope: &'static str,
    pub invocation: ConsumerOrderInvocation,
    pub merchant_order: MerchantBusinessReceipt,
    pub merchant_statement_authority: &'static str,
    pub result: Value,
    pub platform_meter: ConsumerOrderPlatformMeter,
    pub erp_handoff: Option<ConsumerOrderErpHandoff>,
    pub closure_status: &'static str,
    pub funds_moved: bool,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerOrderInvocation {
    pub invocation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_app_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerOrderPlatformMeter {
    pub units: i64,
    pub unit_price_micros: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub settlement_status: String,
    pub funds_moved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerOrderErpHandoff {
    pub status: String,
    pub target_domain: String,
    pub target_reference_sha256: Option<String>,
    pub error_code: Option<String>,
    pub assertion_authority: String,
    pub completed_at: String,
    pub funds_moved: bool,
}
