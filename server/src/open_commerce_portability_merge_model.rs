use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::open_commerce_consumer_model::ConsumerPreferences;

pub(crate) const CONSUMER_PORTABILITY_MERGE_PLAN_SCHEMA: &str =
    "open_commerce.consumer_portability_merge_plan.v1";
pub(crate) const CONSUMER_PORTABILITY_MERGE_ADOPTION_SCHEMA: &str =
    "open_commerce.consumer_portability_merge_adoption.v1";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerPortabilityMergePlanRequest {
    #[serde(default)]
    pub import_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityMergeSource {
    pub import_id: String,
    pub source_operator: String,
    pub source_package_id: String,
    pub source_package_schema: String,
    pub envelope_sha256: String,
    pub payload_sha256: String,
    pub trust_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityMergeCandidate {
    pub import_id: String,
    pub source_operator: String,
    pub source_package_id: String,
    pub trust_status: String,
    pub imported_value: Value,
    pub differs_from_current: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityMergeField {
    pub field: String,
    pub current_value: Value,
    pub candidates: Vec<ConsumerPortabilityMergeCandidate>,
    pub distinct_candidate_count: usize,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityMergePlan {
    pub schema: String,
    pub current_profile_revision: Option<i64>,
    pub sources: Vec<ConsumerPortabilityMergeSource>,
    pub fields: Vec<ConsumerPortabilityMergeField>,
    pub automatic_conflict_resolution: bool,
    pub automatic_relationship_restore: bool,
    pub automatic_business_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityFieldSource {
    pub field: String,
    pub import_id: String,
    pub source_operator: String,
    pub source_package_id: String,
    pub envelope_sha256: String,
    pub payload_sha256: String,
    pub trust_status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ConsumerPortabilityFieldSelection {
    pub field: String,
    pub import_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApplyConsumerPortabilityMergeRequest {
    #[serde(default)]
    pub import_ids: Vec<String>,
    pub expected_current_revision: Option<i64>,
    #[serde(default)]
    pub selections: Vec<ConsumerPortabilityFieldSelection>,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RollbackConsumerPortabilityMergeRequest {
    pub expected_current_revision: i64,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityMergeAdoption {
    pub schema: String,
    pub id: String,
    pub source_import_ids: Vec<String>,
    pub field_sources: Vec<ConsumerPortabilityFieldSource>,
    pub before_preferences: Option<ConsumerPreferences>,
    pub before_revision: Option<i64>,
    pub applied_preferences: ConsumerPreferences,
    pub resulting_revision: i64,
    pub status: String,
    pub applied_at: String,
    pub rolled_back_at: Option<String>,
    pub rollback_revision: Option<i64>,
}
