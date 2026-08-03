use serde::{Deserialize, Serialize};

use crate::open_commerce_directory_model::{
    OpenCommerceDirectoryCapability, OpenCommerceDirectoryMerchant,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ConsumerPreferences {
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub max_unit_price_micros: Option<i64>,
    #[serde(default)]
    pub prefer_public: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConsumerDiscoveryRequest {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub requester_app_id: String,
    #[serde(default)]
    pub ranking_policy: Option<String>,
    #[serde(default)]
    pub include_ranking_receipt: bool,
    #[serde(default)]
    pub require_current_declaration: bool,
    #[serde(default)]
    pub require_internal_sync_receipt: bool,
    #[serde(default)]
    pub source_provider_key: Option<String>,
    #[serde(default)]
    pub source_data_domain: Option<String>,
    #[serde(default)]
    pub preferences: ConsumerPreferences,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerAuthorizationState {
    pub required: bool,
    pub status: String,
    pub grant_id: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerDiscoveryMatch {
    pub merchant: OpenCommerceDirectoryMerchant,
    pub capability: OpenCommerceDirectoryCapability,
    pub score: i64,
    pub reasons: Vec<String>,
    pub authorization: ConsumerAuthorizationState,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConsumerDiscoveryResponse {
    pub schema: &'static str,
    pub capability_contract_profile: &'static str,
    pub requester_app_id: String,
    pub ranking_policy: String,
    pub ranking_policy_label: String,
    pub ranking_explanation: String,
    pub ranking_is_paid: bool,
    pub ranking_is_user_selected: bool,
    pub freshness_requirement: &'static str,
    pub source_requirement: &'static str,
    pub source_filter: ConsumerSourceFilter,
    pub available_ranking_policies: Vec<ConsumerRankingPolicyDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_receipt: Option<ConsumerRankingReceipt>,
    pub matches: Vec<ConsumerDiscoveryMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerRankingPolicyDescriptor {
    pub key: String,
    pub label: String,
    pub explanation: String,
    pub paid_placement: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerSourceFilter {
    pub provider_key: Option<String>,
    pub data_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerRankingReceipt {
    pub schema: &'static str,
    pub hash_algorithm: &'static str,
    pub canonical_payload_json: String,
    pub payload_sha256: String,
    pub signed_by_operator: bool,
}

fn default_limit() -> usize {
    10
}
