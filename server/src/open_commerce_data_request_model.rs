use serde::{Deserialize, Serialize};

pub(crate) const DATA_REQUEST_TYPE_ERASURE: &str = "erase_linked_data";
pub(crate) const DATA_REQUEST_STATUS_REQUESTED: &str = "requested";
pub(crate) const DATA_REQUEST_STATUS_IN_PROGRESS: &str = "in_progress";
pub(crate) const DATA_REQUEST_STATUS_COMPLETED: &str = "completed";
pub(crate) const DATA_REQUEST_STATUS_REJECTED: &str = "rejected";
pub(crate) const DATA_REQUEST_STATUS_WITHDRAWN: &str = "withdrawn";

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
