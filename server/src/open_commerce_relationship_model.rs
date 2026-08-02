use serde::{Deserialize, Serialize};

pub(crate) const RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER: &str = "preference.remember";
pub(crate) const RELATIONSHIP_SCOPE_MEMBERSHIP_LINK: &str = "membership.link";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceConsumerRelationship {
    pub id: String,
    pub merchant_id: String,
    pub source_app_id: String,
    pub subject_alias: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    pub status: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerRelationshipRequest {
    pub merchant_id: String,
    #[serde(default = "default_source_app_id")]
    pub source_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RenewConsumerRelationshipRequest {
    #[serde(default = "default_source_app_id")]
    pub source_app_id: String,
    pub expires_at: String,
}

fn default_source_app_id() -> String {
    "pc-web".to_string()
}
