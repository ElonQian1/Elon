use serde::{Deserialize, Serialize};

use crate::open_commerce_developer_event_model::DeveloperTerminalEventSummary;

pub(crate) const DEVELOPER_WEBHOOK_SUBSCRIPTION_SCHEMA: &str =
    "open_commerce.developer_webhook_subscription.v2";
pub(crate) const DEVELOPER_WEBHOOK_DELIVERY_SCHEMA: &str =
    "open_commerce.developer_webhook_delivery.v2";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperWebhookSubscription {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub app_record_id: String,
    pub app_id: String,
    pub environment: String,
    pub callback_url: String,
    pub signing_key_id: String,
    pub signing_secret_version: i64,
    pub deliver_on_succeeded: bool,
    pub deliver_on_failed: bool,
    pub status: String,
    pub verification_status: String,
    pub verification_attempted_at: Option<String>,
    pub verification_error_code: Option<String>,
    pub verified_at: Option<String>,
    pub consecutive_failures: i64,
    pub last_delivery_at: Option<String>,
    pub last_error_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub disabled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperWebhookCredential {
    pub schema: &'static str,
    pub subscription: DeveloperWebhookSubscription,
    pub signing_secret: String,
    pub signing_secret_visible_once: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperWebhookDelivery {
    pub schema: &'static str,
    pub id: String,
    pub subscription_id: String,
    pub invocation_id: String,
    pub event_sequence: i64,
    pub event_type: String,
    pub enqueue_source: String,
    pub status: String,
    pub attempt_count: i64,
    pub manual_retry_count: i64,
    pub next_attempt_at: String,
    pub response_status: Option<i64>,
    pub error_code: Option<String>,
    pub created_at: String,
    pub last_attempt_at: Option<String>,
    pub last_manual_retry_at: Option<String>,
    pub history_replay_requested_at: Option<String>,
    pub delivered_at: Option<String>,
    pub dead_letter_acknowledged_at: Option<String>,
    pub dead_letter_acknowledged_by_user_id: Option<String>,
    pub dead_letter_acknowledgement_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DeveloperWebhookDeliveryClaim {
    pub delivery: DeveloperWebhookDelivery,
    pub owner_user_id: String,
    pub app_id: String,
    pub environment: String,
    pub callback_url: String,
    pub signing_key_id: String,
    pub signing_secret_version: i64,
    pub lease_owner: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDeveloperWebhookRequest {
    pub callback_url: String,
    pub environment: Option<String>,
    pub deliver_on_succeeded: Option<bool>,
    pub deliver_on_failed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeveloperWebhookHistoryReplayRequest {
    pub after_sequence: i64,
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AcknowledgeDeveloperWebhookDeadLetterRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeveloperWebhookHistoryReplayResult {
    pub schema: &'static str,
    pub subscription_id: String,
    pub after_sequence: i64,
    pub processed_through_sequence: i64,
    pub eligible_count: usize,
    pub enqueued_count: usize,
    pub already_present_count: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeveloperWebhookEnvelope {
    pub schema: &'static str,
    pub delivery_id: String,
    pub subscription_id: String,
    pub app_id: String,
    pub environment: String,
    pub emitted_at: String,
    pub event: DeveloperTerminalEventSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeveloperWebhookVerificationEnvelope {
    pub schema: &'static str,
    pub subscription_id: String,
    pub environment: String,
    pub challenge: String,
    pub issued_at: String,
    pub expires_at: String,
}
