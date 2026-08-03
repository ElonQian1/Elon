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
    pub homepage_url: Option<String>,
    pub privacy_policy_url: Option<String>,
    pub terms_url: Option<String>,
    pub support_email: Option<String>,
    pub requested_scopes: Vec<String>,
    pub manifest_status: String,
    pub manifest_revision: i64,
    pub submitted_at: Option<String>,
    pub reviewed_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub review_note: Option<String>,
    pub domain_verification_status: String,
    pub domain_verification_host: Option<String>,
    pub domain_verification_revision: Option<i64>,
    pub domain_verification_expires_at: Option<String>,
    pub domain_verification_attempted_at: Option<String>,
    pub domain_verified_at: Option<String>,
    pub domain_verification_error_code: Option<String>,
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

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateDeveloperAppManifestRequest {
    pub expected_manifest_revision: i64,
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[serde(default)]
    pub privacy_policy_url: Option<String>,
    #[serde(default)]
    pub terms_url: Option<String>,
    #[serde(default)]
    pub support_email: Option<String>,
    #[serde(default)]
    pub requested_scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SubmitDeveloperAppManifestRequest {
    pub expected_manifest_revision: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewDeveloperAppManifestRequest {
    pub expected_manifest_revision: i64,
    pub decision: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IssueDeveloperAppDomainChallengeRequest {
    pub expected_manifest_revision: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperAppDomainChallengeCredential {
    pub schema: &'static str,
    pub app: OpenCommerceDeveloperApp,
    pub verification_url: String,
    pub verification_content: String,
    pub content_visible_once: bool,
    pub expires_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DeveloperAppDomainChallengeState {
    pub app_record_id: String,
    pub project_id: String,
    pub manifest_revision: i64,
    pub verification_host: String,
    pub challenge_hash: String,
    pub expires_at: String,
    pub status: String,
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
    pub grant_expires_at: Option<String>,
    pub grant_max_invocations: Option<i64>,
    pub grant_max_amount_micros: Option<i64>,
    pub grant_budget_currency: Option<String>,
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
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub max_invocations: Option<i64>,
    #[serde(default)]
    pub max_amount_micros: Option<i64>,
    #[serde(default = "default_budget_currency")]
    pub budget_currency: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeveloperInvokeRequest {
    pub merchant_id: String,
    pub capability_key: String,
    #[serde(default)]
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    #[serde(default)]
    pub action_confirmation_id: Option<String>,
    #[serde(default = "empty_object")]
    pub input: Value,
}

fn empty_object() -> Value {
    serde_json::json!({})
}

fn default_budget_currency() -> String {
    "CNY".to_string()
}
