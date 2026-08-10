use anyhow::{bail, Result};
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

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceActionCancellationResponse {
    pub schema: &'static str,
    pub confirmation_id: String,
    pub merchant_id: String,
    pub capability_key: String,
    pub requester_app_id: String,
    pub status: &'static str,
    pub canceled_at: String,
    pub invocation_created: bool,
    pub next_step: &'static str,
}

impl TryFrom<OpenCommerceActionConfirmation> for OpenCommerceActionCancellationResponse {
    type Error = anyhow::Error;

    fn try_from(confirmation: OpenCommerceActionConfirmation) -> Result<Self> {
        let Some(canceled_at) = confirmation.canceled_at else {
            bail!("动作确认没有主动取消证据");
        };
        if confirmation.invocation_id.is_some() {
            bail!("已创建调用的动作确认不能投影为取消成功");
        }
        Ok(Self {
            schema: "open_commerce.consumer_action_confirmation_cancellation.v1",
            confirmation_id: confirmation.id,
            merchant_id: confirmation.merchant_id,
            capability_key: confirmation.capability_key,
            requester_app_id: confirmation.requester_app_id,
            status: "canceled",
            canceled_at,
            invocation_created: false,
            next_step: "stop",
        })
    }
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
