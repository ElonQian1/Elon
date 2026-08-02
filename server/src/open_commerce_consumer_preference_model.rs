use serde::{Deserialize, Serialize};

use crate::open_commerce_consumer_model::ConsumerPreferences;

pub(crate) const PREFERENCE_FIELD_CATEGORIES: &str = "categories";
pub(crate) const PREFERENCE_FIELD_TAGS: &str = "tags";
pub(crate) const PREFERENCE_FIELD_CITY: &str = "city";
pub(crate) const PREFERENCE_FIELD_MAX_UNIT_PRICE: &str = "max_unit_price_micros";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPreferenceProfile {
    pub preferences: ConsumerPreferences,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpsertConsumerPreferenceProfileRequest {
    pub preferences: ConsumerPreferences,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeleteConsumerPreferenceProfileResult {
    pub deleted_profile: bool,
    pub removed_disclosures: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct DisclosedConsumerPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_unit_price_micros: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPreferenceDisclosure {
    pub relationship_id: String,
    pub merchant_id: String,
    pub subject_alias: String,
    pub relationship_status: String,
    pub shared_fields: Vec<String>,
    pub preferences: DisclosedConsumerPreferences,
    pub profile_revision: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpsertConsumerPreferenceDisclosureRequest {
    pub shared_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeleteConsumerPreferenceDisclosureResult {
    pub relationship_id: String,
    pub deleted: bool,
}
