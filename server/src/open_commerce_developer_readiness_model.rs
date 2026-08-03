//! Read-only production-readiness contracts for developer Apps.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperProductionReadinessStep {
    pub code: &'static str,
    pub ready: bool,
    pub blocker_code: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperProductionReadinessSummary {
    pub schema: &'static str,
    pub app_record_id: String,
    pub app_id: String,
    pub manifest_revision: i64,
    pub admission_status: Option<String>,
    pub admission_revision: Option<i64>,
    pub production_credentials_enabled: bool,
    pub current_production_credential_present: bool,
    pub production_webhooks_enabled: bool,
    pub active_production_webhook_count: i64,
    pub production_invocation_ready: bool,
    pub production_webhook_ready: bool,
    pub next_action_code: Option<&'static str>,
    pub blocker_codes: Vec<&'static str>,
    pub steps: Vec<DeveloperProductionReadinessStep>,
    pub generated_at: String,
}
