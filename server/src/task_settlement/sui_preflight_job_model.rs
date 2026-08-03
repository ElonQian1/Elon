use serde::{Deserialize, Serialize};

use super::{
    sui_adapter_handoff_model::SuiAdapterHandoffBundle, sui_preflight_model::SuiPreflightReport,
};

pub(crate) const SUI_PREFLIGHT_JOB_SCHEMA: &str = "task_economy.sui_preflight_job.v1";
pub(super) const SUI_PREFLIGHT_JOB_LIST_SCHEMA: &str = "task_economy.sui_preflight_job_list.v1";
pub(super) const SUI_PREFLIGHT_JOB_POLL_SCHEMA: &str = "task_economy.sui_preflight_job_poll.v1";
pub(super) const SUI_PREFLIGHT_JOB_ISSUE_SCHEMA: &str = "task_economy.sui_preflight_job_issue.v1";
pub(super) const SUI_PREFLIGHT_JOB_RENEW_SCHEMA: &str = "task_economy.sui_preflight_job_renew.v1";
pub(super) const SUI_PREFLIGHT_JOB_RELEASE_SCHEMA: &str =
    "task_economy.sui_preflight_job_release.v1";
pub(super) const SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA: &str =
    "task_economy.sui_preflight_job_complete.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJob {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub package_kind: String,
    pub projection_package_id: String,
    pub target_network: String,
    pub handoff_digest: String,
    pub projection_digest: String,
    pub status: String,
    pub adapter_id: Option<String>,
    pub credential_version: Option<i64>,
    pub attempt_no: i64,
    pub lease_token_hint: Option<String>,
    pub lease_started_at: Option<String>,
    pub lease_expires_at: Option<String>,
    pub lease_deadline_at: Option<String>,
    pub report_id: Option<String>,
    pub last_error: Option<String>,
    pub created_by_user_id: String,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobList {
    pub schema: &'static str,
    pub project_id: String,
    pub jobs: Vec<SuiPreflightJob>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobIssue {
    pub schema: &'static str,
    pub job: SuiPreflightJob,
    pub lease_token: String,
    pub lease_token_visible_once: bool,
    pub handoff: SuiAdapterHandoffBundle,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobPoll {
    pub schema: &'static str,
    pub claimed: bool,
    pub issue: Option<SuiPreflightJobIssue>,
    pub retry_after_seconds: i64,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobRenew {
    pub schema: &'static str,
    pub renewed: bool,
    pub job: SuiPreflightJob,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobRelease {
    pub schema: &'static str,
    pub released: bool,
    pub job: SuiPreflightJob,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiPreflightJobComplete {
    pub schema: &'static str,
    pub completed: bool,
    pub job: SuiPreflightJob,
    pub report: SuiPreflightReport,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueueSuiPreflightJobRequest {
    pub package_kind: String,
    pub projection_package_id: String,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelSuiPreflightJobRequest {
    pub reason: String,
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaimSuiPreflightJobRequest {
    #[serde(default = "default_lease_seconds")]
    pub lease_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenewSuiPreflightJobRequest {
    pub lease_token: String,
    pub extend_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseSuiPreflightJobRequest {
    pub lease_token: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteSuiPreflightJobRequest {
    pub lease_token: String,
    pub outcome: String,
    pub summary: String,
    pub tool_version: String,
    pub idempotency_key: String,
}

fn default_lease_seconds() -> i64 {
    300
}
