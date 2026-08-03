use serde::Serialize;

pub(crate) const WEBHOOK_HEALTH_IDLE: &str = "idle";
pub(crate) const WEBHOOK_HEALTH_HEALTHY: &str = "healthy";
pub(crate) const WEBHOOK_HEALTH_PROCESSING: &str = "processing";
pub(crate) const WEBHOOK_HEALTH_ATTENTION: &str = "attention";
pub(crate) const WEBHOOK_HEALTH_ACTION_REQUIRED: &str = "action_required";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperWebhookEnvironmentHealth {
    pub environment: String,
    pub status: String,
    pub subscription_count: i64,
    pub active_subscription_count: i64,
    pub verified_subscription_count: i64,
    pub pending_delivery_count: i64,
    pub retry_delivery_count: i64,
    pub delivering_delivery_count: i64,
    pub dead_delivery_count: i64,
    pub oldest_queued_at: Option<String>,
    pub latest_delivery_at: Option<String>,
    pub latest_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperWebhookHealthSummary {
    pub schema: &'static str,
    pub app_record_id: String,
    pub app_id: String,
    pub production_webhooks_enabled: bool,
    pub production_credentials_enabled: bool,
    pub production_credential_eligible: bool,
    pub production_ready: bool,
    pub production_blocker_code: Option<String>,
    pub environments: Vec<DeveloperWebhookEnvironmentHealth>,
    pub generated_at: String,
}
