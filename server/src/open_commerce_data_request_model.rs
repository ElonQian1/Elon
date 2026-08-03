use serde::{Deserialize, Serialize};

pub(crate) const DATA_REQUEST_TYPE_ERASURE: &str = "erase_linked_data";
pub(crate) const DATA_REQUEST_STATUS_REQUESTED: &str = "requested";
pub(crate) const DATA_REQUEST_STATUS_IN_PROGRESS: &str = "in_progress";
pub(crate) const DATA_REQUEST_STATUS_COMPLETED: &str = "completed";
pub(crate) const DATA_REQUEST_STATUS_REJECTED: &str = "rejected";
pub(crate) const DATA_REQUEST_STATUS_WITHDRAWN: &str = "withdrawn";
pub(crate) const DATA_REQUEST_FOLLOWUP_ACTION_REMINDER: &str = "reminder";
pub(crate) const DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE: &str = "escalate_attention";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OpenCommerceConsumerDataRequest {
    pub id: String,
    pub relationship_id: String,
    pub merchant_id: String,
    pub subject_alias: String,
    pub request_type: String,
    pub status: String,
    pub resolution_kind: Option<String>,
    pub resolution_note: Option<String>,
    pub requested_at: String,
    pub accepted_at: Option<String>,
    pub resolved_at: Option<String>,
    pub withdrawn_at: Option<String>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operational_target_at: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_operationally_overdue: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reminder_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reminded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_reminder_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer_escalated_at: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub can_send_reminder: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub can_escalate_attention: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerDataErasureRequest {
    pub relationship_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DecideConsumerDataRequest {
    pub action: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FollowUpConsumerDataRequest {
    pub action: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub note: String,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}
