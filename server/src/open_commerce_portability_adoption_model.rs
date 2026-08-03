use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::open_commerce_consumer_model::ConsumerPreferences;

pub(crate) const CONSUMER_PORTABILITY_ADOPTION_PLAN_SCHEMA: &str =
    "open_commerce.consumer_portability_adoption_plan.v1";
pub(crate) const CONSUMER_PORTABILITY_ADOPTION_SCHEMA: &str =
    "open_commerce.consumer_portability_adoption.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityPreferenceChange {
    pub field: String,
    pub current_value: Value,
    pub imported_value: Value,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityRelationshipCandidate {
    pub source_relationship_id: String,
    pub source_merchant_id: String,
    pub source_status: String,
    pub requested_scopes: Vec<String>,
    pub purpose: String,
    pub requires_reauthorization: bool,
    pub source_identity_key_ids: Vec<String>,
    pub verified_target_merchant_ids: Vec<String>,
    pub identity_match_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityAdoptionPlan {
    pub schema: String,
    pub import_id: String,
    pub import_trust_status: String,
    pub source_package_schema: String,
    pub imported_profile_available: bool,
    pub current_profile_revision: Option<i64>,
    pub preference_changes: Vec<ConsumerPortabilityPreferenceChange>,
    pub relationship_candidates: Vec<ConsumerPortabilityRelationshipCandidate>,
    pub automatic_relationship_restore: bool,
    pub automatic_business_write: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApplyConsumerPortabilityPreferencesRequest {
    pub expected_current_revision: Option<i64>,
    #[serde(default)]
    pub selected_fields: Vec<String>,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RollbackConsumerPortabilityAdoptionRequest {
    pub expected_current_revision: i64,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityAdoption {
    pub schema: String,
    pub id: String,
    pub import_id: String,
    pub kind: String,
    pub before_preferences: Option<ConsumerPreferences>,
    pub before_revision: Option<i64>,
    pub applied_preferences: ConsumerPreferences,
    #[serde(default)]
    pub selected_fields: Vec<String>,
    pub resulting_revision: i64,
    pub status: String,
    pub applied_at: String,
    pub rolled_back_at: Option<String>,
    pub rollback_revision: Option<i64>,
}
