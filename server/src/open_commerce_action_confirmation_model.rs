use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::open_commerce_model::InvokeCapabilityRequest;

pub(crate) const ACTION_CONFIRMATION_PHRASE: &str = "CONFIRM_ACTION";
pub(crate) const ACTION_CANCELLATION_PHRASE: &str = "CANCEL_ACTION";
pub(crate) const ACTION_CONFIRMATION_TTL_SECONDS: i64 = 300;
pub(crate) const ACTION_CONFIRMATION_RETENTION_DAYS: i64 = 7;
pub(crate) const MAX_ACTIVE_ACTION_CONFIRMATIONS_PER_APP: i64 = 20;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceActionConfirmation {
    pub id: String,
    #[serde(skip_serializing)]
    pub project_id: String,
    pub merchant_id: String,
    #[serde(skip_serializing)]
    pub capability_id: String,
    pub capability_key: String,
    #[serde(skip_serializing)]
    pub requester_user_id: String,
    pub requester_app_id: String,
    pub grant_id: Option<String>,
    pub idempotency_key: String,
    #[serde(skip_serializing)]
    pub request_hash: String,
    pub request_shape: Value,
    pub status: String,
    pub expires_at: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub consumed_at: Option<String>,
    pub canceled_at: Option<String>,
    pub invocation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfirmActionConfirmationRequest {
    pub confirmation_phrase: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvokeCapabilityEnvelope {
    #[serde(flatten)]
    pub invocation: InvokeCapabilityRequest,
    #[serde(default)]
    pub action_confirmation_id: Option<String>,
}
