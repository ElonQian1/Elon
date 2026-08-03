use serde::{Deserialize, Serialize};

pub(crate) const SUI_PREFLIGHT_ADAPTER_SCHEMA: &str = "task_economy.sui_preflight_adapter.v1";
pub(crate) const SUI_PREFLIGHT_ADAPTER_ISSUE_SCHEMA: &str =
    "task_economy.sui_preflight_adapter_issue.v1";
pub(super) const SUI_PREFLIGHT_ADAPTER_LIST_SCHEMA: &str =
    "task_economy.sui_preflight_adapter_list.v1";
pub(crate) const SUI_PREFLIGHT_REPORT_SCHEMA: &str = "task_economy.sui_preflight_report.v1";
pub(super) const SUI_PREFLIGHT_REPORT_LIST_SCHEMA: &str =
    "task_economy.sui_preflight_report_list.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightAdapter {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub display_name: String,
    pub status: String,
    pub allowed_networks: Vec<String>,
    pub allowed_package_kinds: Vec<String>,
    pub token_hint: String,
    pub credential_version: i64,
    pub created_by_user_id: String,
    pub last_used_at: Option<String>,
    pub expires_at: String,
    pub is_expired: bool,
    pub disabled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightAdapterIssue {
    pub schema: &'static str,
    pub adapter: SuiPreflightAdapter,
    pub adapter_token: String,
    pub token_visible_once: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightAdapterList {
    pub schema: &'static str,
    pub project_id: String,
    pub runtime_enabled: bool,
    pub adapters: Vec<SuiPreflightAdapter>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightReport {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub adapter_id: String,
    pub credential_version: i64,
    pub package_kind: String,
    pub projection_package_id: String,
    pub target_network: String,
    pub handoff_digest: String,
    pub projection_digest: String,
    pub outcome: String,
    pub summary: String,
    pub tool_version: String,
    pub idempotency_key: String,
    pub report_digest: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightReportList {
    pub schema: &'static str,
    pub project_id: String,
    pub runtime_enabled: bool,
    pub reports: Vec<SuiPreflightReport>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateSuiPreflightAdapterRequest {
    pub display_name: String,
    pub allowed_networks: Vec<String>,
    pub allowed_package_kinds: Vec<String>,
    pub expires_in_days: i64,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RotateSuiPreflightAdapterRequest {
    pub expires_in_days: i64,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfirmSuiPreflightAdapterChangeRequest {
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordSuiPreflightReportRequest {
    pub package_kind: String,
    pub projection_package_id: String,
    pub handoff_digest: String,
    pub outcome: String,
    pub summary: String,
    pub tool_version: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy)]
pub(crate) struct CreateSuiPreflightReport<'a> {
    pub project_id: &'a str,
    pub adapter_id: &'a str,
    pub credential_version: i64,
    pub package_kind: &'a str,
    pub projection_package_id: &'a str,
    pub target_network: &'a str,
    pub handoff_digest: &'a str,
    pub projection_digest: &'a str,
    pub outcome: &'a str,
    pub summary: &'a str,
    pub tool_version: &'a str,
    pub idempotency_key: &'a str,
    pub report_digest: &'a str,
}

pub(crate) struct PreparedSuiPreflightReport {
    pub project_id: String,
    pub adapter_id: String,
    pub credential_version: i64,
    pub package_kind: String,
    pub projection_package_id: String,
    pub target_network: String,
    pub handoff_digest: String,
    pub projection_digest: String,
    pub outcome: String,
    pub summary: String,
    pub tool_version: String,
    pub idempotency_key: String,
    pub report_digest: String,
}

impl PreparedSuiPreflightReport {
    pub(crate) fn as_create(&self) -> CreateSuiPreflightReport<'_> {
        CreateSuiPreflightReport {
            project_id: &self.project_id,
            adapter_id: &self.adapter_id,
            credential_version: self.credential_version,
            package_kind: &self.package_kind,
            projection_package_id: &self.projection_package_id,
            target_network: &self.target_network,
            handoff_digest: &self.handoff_digest,
            projection_digest: &self.projection_digest,
            outcome: &self.outcome,
            summary: &self.summary,
            tool_version: &self.tool_version,
            idempotency_key: &self.idempotency_key,
            report_digest: &self.report_digest,
        }
    }
}
