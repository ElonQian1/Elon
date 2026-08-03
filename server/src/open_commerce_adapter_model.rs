use serde::{Deserialize, Serialize};

pub(crate) const ADAPTER_CREDENTIAL_SCHEMA: &str = "open_commerce.adapter_credential.v1";
pub(crate) const ADAPTER_CREDENTIAL_ISSUE_SCHEMA: &str =
    "open_commerce.adapter_credential_issue.v1";
pub(crate) const ADAPTER_CREDENTIAL_LIST_SCHEMA: &str = "open_commerce.adapter_credential_list.v1";
pub(crate) const ADAPTER_HANDOFF_SCOPE: &str = "business_handoff.write";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterCredential {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub integration_id: String,
    pub status: String,
    pub scopes: Vec<String>,
    pub token_hint: String,
    pub credential_version: i64,
    pub created_by_user_id: String,
    pub last_used_at: Option<String>,
    pub expires_at: String,
    pub is_expired: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterCredentialIssue {
    pub schema: &'static str,
    pub credential: OpenCommerceAdapterCredential,
    pub adapter_token: String,
    pub token_visible_once: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterCredentialList {
    pub schema: &'static str,
    pub project_id: String,
    pub credentials: Vec<OpenCommerceAdapterCredential>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdapterBusinessHandoffReceiptRequest {
    pub invocation_id: String,
    pub receipt_key: String,
    pub status: String,
    pub target_domain: String,
    pub evidence_result_sha256: String,
    #[serde(default)]
    pub target_reference: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmedAdapterCredentialChangeRequest {
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RotateAdapterCredentialRequest {
    pub confirmed_by_user: bool,
    pub expires_in_days: i64,
}
