use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDeveloperApp {
    pub id: String,
    pub project_id: String,
    pub owner_user_id: String,
    pub app_id: String,
    pub display_name: String,
    pub environment: String,
    pub status: String,
    pub token_hint: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDeveloperAppCredential {
    pub schema: &'static str,
    pub app: OpenCommerceDeveloperApp,
    pub test_token: String,
    pub token_visible_once: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateDeveloperAppRequest {
    pub app_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAuthorizationRequest {
    pub id: String,
    pub merchant_project_id: String,
    pub merchant_id: String,
    pub requester_user_id: String,
    pub requester_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    pub status: String,
    pub decided_by_user_id: Option<String>,
    pub decision_reason: Option<String>,
    pub grant_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateAuthorizationRequest {
    pub merchant_id: String,
    pub requester_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DecideAuthorizationRequest {
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeveloperInvokeRequest {
    pub merchant_id: String,
    pub capability_key: String,
    #[serde(default)]
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    #[serde(default = "empty_object")]
    pub input: Value,
}

fn empty_object() -> Value {
    serde_json::json!({})
}
