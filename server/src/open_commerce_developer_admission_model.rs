//! Contracts for reviewable developer-App public-network admission.

use serde::{Deserialize, Serialize};

use crate::open_commerce_developer_model::OpenCommerceDeveloperApp;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperAppAdmission {
    pub schema: &'static str,
    pub id: String,
    pub app_record_id: String,
    pub project_id: String,
    pub manifest_revision: i64,
    pub organization_name: String,
    pub jurisdiction: String,
    pub registration_id: String,
    pub attested_at: String,
    pub status: String,
    pub requested_at: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub review_note: Option<String>,
    pub risk_tier: Option<String>,
    pub suspended_at: Option<String>,
    pub production_credential_issued: bool,
    pub network_access_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperAppAdmissionReviewItem {
    pub app: OpenCommerceDeveloperApp,
    pub admission: DeveloperAppAdmission,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SubmitDeveloperAppAdmissionRequest {
    pub expected_manifest_revision: i64,
    pub organization_name: String,
    pub jurisdiction: String,
    pub registration_id: String,
    #[serde(default)]
    pub information_attested: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewDeveloperAppAdmissionRequest {
    pub expected_manifest_revision: i64,
    pub decision: String,
    #[serde(default)]
    pub risk_tier: String,
    #[serde(default)]
    pub note: String,
}
