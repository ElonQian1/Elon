use serde::{Deserialize, Serialize};

use crate::open_commerce_developer_model::OpenCommerceAuthorizationRequest;

pub(crate) const PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA: &str =
    "open_commerce.portability_relationship_mapping.v1";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreatePortabilityRelationshipMappingRequest {
    pub import_id: String,
    pub source_relationship_id: String,
    pub target_merchant_id: String,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PortabilityRelationshipMapping {
    pub schema: String,
    pub id: String,
    pub import_id: String,
    pub source_relationship_id: String,
    pub source_merchant_id: String,
    pub target_merchant_id: String,
    pub target_merchant_project_id: String,
    pub status: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreatePortabilityReauthorizationRequest {
    pub requester_app_id: String,
    pub scopes: Vec<String>,
    pub purpose: String,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PortabilityReauthorizationResult {
    pub schema: &'static str,
    pub mapping: PortabilityRelationshipMapping,
    pub authorization_request: OpenCommerceAuthorizationRequest,
    pub old_grant_restored: bool,
}
