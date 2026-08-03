//! Merchant-controlled links between public capabilities and internal sync receipts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceCapabilitySourceLink {
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub capability_id: String,
    pub capability_key: String,
    pub capability_version: i64,
    pub current_capability_version: i64,
    pub integration_id: String,
    pub sync_receipt_id: String,
    pub data_domain: String,
    pub provider_key: String,
    pub connection_mode: String,
    pub integration_status: String,
    pub sync_kind: String,
    pub receipt_status: String,
    pub receipt_sha256: String,
    pub receipt_completed_at: String,
    pub revision: i64,
    pub linked_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub publishable: bool,
    pub blocking_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LinkCapabilitySourceRequest {
    pub integration_id: String,
    pub sync_receipt_id: String,
    pub data_domain: String,
}
