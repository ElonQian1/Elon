use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::store::rust_cache::gc_requests::RustCacheGcOptions;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateGcRequest {
    #[serde(default)]
    pub(super) options: RustCacheGcOptions,
    pub(super) acknowledge_remote_gc: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApproveGcRequest {
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) acknowledgement: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeGcPlanUpload {
    pub(super) summary: GcPlanSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GcPlanSummary {
    pub(super) schema: String,
    pub(super) request_id: String,
    pub(super) plan_id: String,
    pub(super) plan_digest: String,
    pub(super) node_id: String,
    pub(super) generated_at_utc: String,
    pub(super) expires_at_utc: String,
    pub(super) options: RustCacheGcOptions,
    pub(super) action_count: u64,
    pub(super) reclaim_bytes: u64,
    pub(super) active_writer_count: u64,
    pub(super) reasons: Vec<GcReasonCount>,
    pub(super) security: GcPlanSecurity,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GcReasonCount {
    pub(super) reason: String,
    pub(super) count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GcPlanSecurity {
    pub(super) absolute_paths_included: bool,
    pub(super) secrets_included: bool,
    pub(super) destructive_actions_authorized: bool,
    pub(super) approval_binds_plan_digest: bool,
    pub(super) target_rescan_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeGcResultUpload {
    pub(super) schema: String,
    pub(super) request_id: String,
    pub(super) node_id: String,
    pub(super) phase: String,
    pub(super) status: String,
    pub(super) plan_id: Option<String>,
    pub(super) plan_digest: Option<String>,
    pub(super) completed_at_utc: String,
    pub(super) approved_action_count: u64,
    pub(super) removed_action_count: u64,
    pub(super) reclaimed_bytes: u64,
    pub(super) failure_code: Option<String>,
    pub(super) security: GcResultSecurity,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GcResultSecurity {
    pub(super) absolute_paths_included: bool,
    pub(super) secrets_included: bool,
    pub(super) execution_bound_to_plan_digest: bool,
    pub(super) local_rescan_completed: bool,
}

pub(super) fn validate_plan_summary(
    summary: &GcPlanSummary,
    node_id: &str,
    request_id: &str,
    options: RustCacheGcOptions,
) -> Result<()> {
    let reason_count = summary
        .reasons
        .iter()
        .try_fold(0_u64, |total, reason| total.checked_add(reason.count));
    if summary.schema != "elon.rust_cache.gc_plan_summary.v1"
        || summary.node_id != node_id
        || summary.request_id != request_id
        || !valid_hex(&summary.plan_id, 32)
        || !valid_hex(&summary.plan_digest, 64)
        || summary.options.force_aged != options.force_aged
        || summary.options.workspace_only != options.workspace_only
        || summary.options.recover_missing_workspaces != options.recover_missing_workspaces
        || summary.options.shared_aliases_only != options.shared_aliases_only
        || summary.action_count > 100_000
        || summary.reasons.len() > 64
        || summary
            .reasons
            .iter()
            .any(|reason| !valid_reason(&reason.reason))
        || reason_count != Some(summary.action_count)
        || summary.security.absolute_paths_included
        || summary.security.secrets_included
        || summary.security.destructive_actions_authorized
        || !summary.security.approval_binds_plan_digest
        || !summary.security.target_rescan_required
    {
        return Err(anyhow!("invalid Rust cache GC plan summary"));
    }
    let generated = chrono::DateTime::parse_from_rfc3339(&summary.generated_at_utc)?;
    let expires = chrono::DateTime::parse_from_rfc3339(&summary.expires_at_utc)?;
    let now = chrono::Utc::now();
    if generated > now + chrono::Duration::minutes(5)
        || expires <= now
        || expires <= generated
        || expires - generated > chrono::Duration::hours(24)
    {
        return Err(anyhow!("invalid Rust cache GC plan lifetime"));
    }
    Ok(())
}

pub(super) fn validate_result(
    result: &NodeGcResultUpload,
    node_id: &str,
    request_id: &str,
    expected_plan_id: Option<&str>,
    expected_plan_digest: Option<&str>,
) -> Result<()> {
    let phase_valid = matches!(result.phase.as_str(), "plan" | "apply");
    let status_valid = matches!(result.status.as_str(), "completed" | "partial" | "failed");
    if result.schema != "elon.rust_cache.gc_result_summary.v1"
        || result.node_id != node_id
        || result.request_id != request_id
        || !phase_valid
        || !status_valid
        || result
            .failure_code
            .as_deref()
            .is_some_and(|code| !valid_failure_code(code))
        || result.security.absolute_paths_included
        || result.security.secrets_included
        || (result.status == "completed" && result.failure_code.is_some())
        || (result.status == "failed" && result.failure_code.is_none())
        || result.removed_action_count > result.approved_action_count
    {
        return Err(anyhow!("invalid Rust cache GC result summary"));
    }
    chrono::DateTime::parse_from_rfc3339(&result.completed_at_utc)?;
    if result.phase == "apply" {
        if result.plan_id.as_deref() != expected_plan_id
            || result.plan_digest.as_deref() != expected_plan_digest
            || !result.security.execution_bound_to_plan_digest
            || (result.status != "failed" && !result.security.local_rescan_completed)
        {
            return Err(anyhow!("GC apply result is not bound to the approved plan"));
        }
    } else if result.status != "failed" || result.plan_id.is_some() || result.plan_digest.is_some()
    {
        return Err(anyhow!("GC planning result must be a plan-phase failure"));
    }
    Ok(())
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value == value.to_ascii_lowercase()
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn valid_reason(reason: &str) -> bool {
    matches!(
        reason,
        "missing-workspace-recovery"
            | "orphaned-task-worktree"
            | "retired-domain"
            | "retired-shared-alias"
            | "old-toolchain-epoch"
            | "disk-watermark"
            | "disk-watermark-lru"
            | "forced-aged-cleanup"
    )
}

fn valid_failure_code(code: &str) -> bool {
    matches!(
        code,
        "local-plan-failed"
            | "local-plan-invalid"
            | "local-apply-refused"
            | "local-receipt-invalid"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_cache_gc_contract_rejects_path_shaped_reason() {
        let now = chrono::Utc::now();
        let mut summary = GcPlanSummary {
            schema: "elon.rust_cache.gc_plan_summary.v1".into(),
            request_id: "a".repeat(32),
            plan_id: "b".repeat(32),
            plan_digest: "c".repeat(64),
            node_id: "node-a".into(),
            generated_at_utc: now.to_rfc3339(),
            expires_at_utc: (now + chrono::Duration::minutes(30)).to_rfc3339(),
            options: RustCacheGcOptions::default(),
            action_count: 1,
            reclaim_bytes: 42,
            active_writer_count: 0,
            reasons: vec![GcReasonCount {
                reason: "orphaned-task-worktree".into(),
                count: 1,
            }],
            security: GcPlanSecurity {
                absolute_paths_included: false,
                secrets_included: false,
                destructive_actions_authorized: false,
                approval_binds_plan_digest: true,
                target_rescan_required: true,
            },
        };
        assert!(validate_plan_summary(
            &summary,
            "node-a",
            &"a".repeat(32),
            RustCacheGcOptions::default()
        )
        .is_ok());

        summary.reasons[0].reason = r"C:\private\cache".into();
        assert!(validate_plan_summary(
            &summary,
            "node-a",
            &"a".repeat(32),
            RustCacheGcOptions::default()
        )
        .is_err());
    }
}
