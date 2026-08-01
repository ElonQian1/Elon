use serde::Serialize;

pub(crate) const APP_ACTIVITY_STATUS_NORMAL: &str = "normal";
pub(crate) const APP_ACTIVITY_STATUS_ATTENTION: &str = "attention";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAppActivityHealth {
    pub merchant_id: String,
    pub requester_app_id: String,
    pub status: String,
    pub total_invocations_24h: i64,
    pub succeeded_invocations_24h: i64,
    pub failed_invocations_24h: i64,
    pub rate_limited_invocations_24h: i64,
    pub grant_budget_rejections_24h: i64,
    pub recovered_invocations_24h: i64,
    pub last_invoked_at: String,
    pub attention_codes: Vec<String>,
}
