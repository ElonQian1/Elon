//! Paginated, machine-readable CI health report for shared project-navigation memory.

use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_native_context::{validate_evidence_current, ProjectContextMemory},
    project_document_native_context_conflict::{inspect_shared_set, NativeContextConflict},
    project_document_native_context_git::{
        relocation_candidates_from_index, relocation_index, GitRelocationIndex,
    },
};

const DEFAULT_HEALTH_LIMIT: usize = 50;
const MAX_HEALTH_LIMIT: usize = 200;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MemoryHealthOptions {
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_failure_policy")]
    pub failure_policy: String,
    #[serde(default)]
    pub include_capabilities: bool,
}

impl Default for MemoryHealthOptions {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: DEFAULT_HEALTH_LIMIT,
            failure_policy: default_failure_policy(),
            include_capabilities: false,
        }
    }
}

pub(crate) fn shared_memory_health(
    workspace: &Path,
    options: &MemoryHealthOptions,
) -> Result<Value> {
    let manifest_path = workspace.join(SECTION_CONFIG_PATH);
    let manifest = if manifest_path.is_file() {
        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("读取共享项目记忆失败：{}", manifest_path.display()))?;
        parse_manifest(Some(&content))?
    } else {
        parse_manifest(None)?
    };
    let mut report =
        memory_health_report_with_options(workspace, &manifest.context_memories, options)?;
    report["receipt_automation"] = json!({
        "node_policy_enabled": crate::node_agent_project_memory_hook_config::enabled(""),
        "trust_mode": "codex_non_managed_hook_review",
        "trust_bypass_enabled": false,
        "runtime_execution_observation_adapter_available": true,
        "runtime_execution_observed": false,
        "status_rule": "Configured does not mean trusted or executed; verify with Codex /hooks and an explicit app-server observation adapter."
    });
    if options.include_capabilities {
        report["capabilities"] = crate::project_document_native_context_capabilities::manifest();
        report["runtime_observation"] =
            crate::project_document_native_context_observation::overview(workspace, None)
                .unwrap_or_else(
                    |error| json!({"measurement_status":"unavailable","error":format!("{error:#}")}),
                );
    }
    Ok(report)
}

#[allow(dead_code)]
pub(crate) fn memory_health_report(workspace: &Path, memories: &[ProjectContextMemory]) -> Value {
    memory_health_report_with_options(workspace, memories, &MemoryHealthOptions::default())
        .unwrap_or_else(|error| json!({"error": format!("{error:#}")}))
}

pub(crate) fn memory_health_report_with_options(
    workspace: &Path,
    memories: &[ProjectContextMemory],
    options: &MemoryHealthOptions,
) -> Result<Value> {
    let options = normalize_options(options)?;
    let relocation_index = relocation_index(workspace);
    let shared_conflicts = inspect_shared_set(memories);
    let today = Utc::now().date_naive();
    let all_items = memories
        .iter()
        .map(|memory| {
            health_item(
                workspace,
                memory,
                &relocation_index,
                shared_conflicts
                    .get(&memory.candidate_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                today,
            )
        })
        .collect::<Vec<_>>();

    let healthy_count = all_items
        .iter()
        .filter(|item| item["issues"].as_array().is_some_and(Vec::is_empty))
        .count();
    let current_count = all_items
        .iter()
        .filter(|item| {
            item["issues"].as_array().is_some_and(|issues| {
                !issues.iter().any(|issue| {
                    matches!(
                        issue.as_str(),
                        Some("evidence_drift")
                            | Some("expired_memory")
                            | Some("potential_conflict")
                    )
                })
            })
        })
        .count();
    let drifted_count = count_issue(&all_items, "evidence_drift");
    let relocation_count = count_status(&all_items, "relocation_suggested");
    let expired_count = count_issue(&all_items, "expired_memory");
    let overdue_count = count_issue(&all_items, "review_overdue");
    let incomplete_count = count_issue(&all_items, "governance_incomplete");
    let conflict_count = count_issue(&all_items, "potential_conflict");
    let total = all_items.len();
    let items = all_items
        .into_iter()
        .skip(options.offset)
        .take(options.limit)
        .collect::<Vec<_>>();
    let returned = items.len();
    let next_offset = (options.offset + returned < total).then_some(options.offset + returned);
    let issue_count = total.saturating_sub(healthy_count);
    let (outcome, exit_code, failure_reasons) = policy_outcome(
        &options.failure_policy,
        drifted_count,
        expired_count,
        overdue_count,
        incomplete_count,
        conflict_count,
    );

    Ok(json!({
        "schema": "elon.project_context_memory_health.v2",
        "checked_count": total,
        "current_count": current_count,
        "healthy_count": healthy_count,
        "issue_count": issue_count,
        "drifted_count": drifted_count,
        "relocation_suggested_count": relocation_count,
        "expired_count": expired_count,
        "review_overdue_count": overdue_count,
        "governance_incomplete_count": incomplete_count,
        "potential_conflict_count": conflict_count,
        "pagination": {
            "offset": options.offset,
            "limit": options.limit,
            "returned": returned,
            "total": total,
            "next_offset": next_offset,
            "next_cursor": next_offset.map(|offset| format!("offset:{offset}")),
        },
        "truncated": next_offset.is_some(),
        "items": items,
        "failure_policy": options.failure_policy,
        "policy_outcome": {
            "status": outcome,
            "recommended_exit_code": exit_code,
            "reasons": failure_reasons,
            "process_was_terminated": false,
        },
        "authority": "diagnostic_only",
        "repair_policy": "Re-open current files with native tools and create a reviewed replacement; never rewrite evidence paths automatically.",
        "source_bodies_returned": 0,
    }))
}

fn health_item(
    workspace: &Path,
    memory: &ProjectContextMemory,
    relocation_index: &GitRelocationIndex,
    conflicts: &[NativeContextConflict],
    today: NaiveDate,
) -> Value {
    let mut drifted_paths = Vec::new();
    let mut relocations = BTreeSet::new();
    for evidence in &memory.evidence {
        if validate_evidence_current(workspace, evidence).is_err() {
            drifted_paths.push(evidence.path.clone());
            relocations.extend(relocation_candidates_from_index(evidence, relocation_index));
        }
    }
    let mut issues = lifecycle_issues(memory, today);
    if !drifted_paths.is_empty() {
        issues.insert(0, "evidence_drift");
    }
    if !conflicts.is_empty() {
        issues.push("potential_conflict");
    }
    let status = if !drifted_paths.is_empty() && !relocations.is_empty() {
        "relocation_suggested"
    } else if !drifted_paths.is_empty() {
        "drifted"
    } else if issues.contains(&"expired_memory") {
        "expired"
    } else if issues.contains(&"review_overdue") {
        "review_overdue"
    } else if issues.contains(&"potential_conflict") {
        "potential_conflict"
    } else if issues.contains(&"governance_incomplete") {
        "governance_incomplete"
    } else {
        "current"
    };
    let relocation_candidates = relocations.into_iter().take(3).collect::<Vec<_>>();
    let repair_plan = repair_plan(memory, &issues, &drifted_paths, &relocation_candidates);
    json!({
        "candidate_id": memory.candidate_id,
        "status": status,
        "owner": memory.owner,
        "scope": memory.scope,
        "review": memory.review,
        "issues": issues,
        "drifted_paths": drifted_paths,
        "relocation_candidates": relocation_candidates,
        "conflicts": conflicts,
        "repair_plan": repair_plan,
        "source_bodies_returned": 0,
    })
}

fn lifecycle_issues(memory: &ProjectContextMemory, today: NaiveDate) -> Vec<&'static str> {
    let mut issues = Vec::new();
    if date(&memory.review.expires_at).is_some_and(|expires| expires < today) {
        issues.push("expired_memory");
    }
    if let (Some(reviewed), days) = (
        date(&memory.review.reviewed_on),
        memory.review.review_interval_days,
    ) {
        if days > 0 && reviewed + Duration::days(i64::from(days)) < today {
            issues.push("review_overdue");
        }
    }
    if memory.owner.trim().is_empty()
        || memory.scope.kind.trim().is_empty()
        || memory.review.reviewed_on.trim().is_empty()
        || memory.review.reviewed_by.trim().is_empty()
        || memory.review.review_interval_days == 0
    {
        issues.push("governance_incomplete");
    }
    issues
}

fn repair_plan(
    memory: &ProjectContextMemory,
    issues: &[&str],
    drifted_paths: &[String],
    relocation_candidates: &[String],
) -> Vec<Value> {
    issues
        .iter()
        .map(|issue| match *issue {
        "evidence_drift" => json!({
            "code": "reverify_and_replace",
            "automatic": false,
            "open_paths": drifted_paths,
            "candidate_paths": relocation_candidates,
            "action": "Open current files with native tools, then submit a reviewed replacement candidate with fresh evidence identity."
        }),
        "expired_memory" => json!({
            "code": "renew_or_retire",
            "automatic": false,
            "action": "Reverify the fact and set a new expiry, or remove it through the reviewed suggestions/apply flow."
        }),
        "review_overdue" => json!({
            "code": "schedule_review",
            "automatic": false,
            "action": "Reverify evidence and renew reviewed_on through the reviewed suggestions/apply flow."
        }),
        "governance_incomplete" => json!({
            "code": "complete_lifecycle_metadata",
            "automatic": false,
            "missing_fields": missing_lifecycle_fields(memory),
            "action": "Assign owner, scope, reviewer, reviewed_on and review interval in a reviewed suggestion."
        }),
        "potential_conflict" => json!({
            "code": "resolve_shared_conflict",
            "automatic": false,
            "action": "Compare current source and binding documents, then keep one reviewed fact or create an explicit reviewed replacement."
        }),
        _ => Value::Null,
    })
        .filter(|value| !value.is_null())
        .collect()
}

fn missing_lifecycle_fields(memory: &ProjectContextMemory) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if memory.owner.trim().is_empty() {
        fields.push("owner");
    }
    if memory.scope.kind.trim().is_empty() {
        fields.push("scope.kind");
    }
    if memory.review.reviewed_on.trim().is_empty() {
        fields.push("review.reviewed_on");
    }
    if memory.review.reviewed_by.trim().is_empty() {
        fields.push("review.reviewed_by");
    }
    if memory.review.review_interval_days == 0 {
        fields.push("review.review_interval_days");
    }
    fields
}

fn normalize_options(options: &MemoryHealthOptions) -> Result<MemoryHealthOptions> {
    if options.offset > 10_000 {
        bail!("memory health offset 不能超过 10000");
    }
    if options.limit == 0 || options.limit > MAX_HEALTH_LIMIT {
        bail!("memory health limit 必须在 1..={MAX_HEALTH_LIMIT}");
    }
    let failure_policy = match options.failure_policy.trim() {
        "" | "advisory" => "advisory",
        "fail_on_drift" => "fail_on_drift",
        "strict" => "strict",
        _ => bail!("memory health failure_policy 仅支持 advisory、fail_on_drift 或 strict"),
    };
    Ok(MemoryHealthOptions {
        offset: options.offset,
        limit: options.limit,
        failure_policy: failure_policy.to_string(),
        include_capabilities: options.include_capabilities,
    })
}

fn policy_outcome(
    policy: &str,
    drifted: usize,
    expired: usize,
    overdue: usize,
    incomplete: usize,
    conflicts: usize,
) -> (&'static str, u8, Vec<&'static str>) {
    let mut reasons = Vec::new();
    if drifted > 0 {
        reasons.push("evidence_drift");
    }
    if expired > 0 {
        reasons.push("expired_memory");
    }
    if overdue > 0 {
        reasons.push("review_overdue");
    }
    if incomplete > 0 {
        reasons.push("governance_incomplete");
    }
    if conflicts > 0 {
        reasons.push("potential_conflict");
    }
    let fail = match policy {
        "strict" => !reasons.is_empty(),
        "fail_on_drift" => drifted > 0 || expired > 0,
        _ => false,
    };
    if fail {
        ("fail", 1, reasons)
    } else if reasons.is_empty() {
        ("pass", 0, reasons)
    } else {
        ("warn", 0, reasons)
    }
}

fn count_status(items: &[Value], status: &str) -> usize {
    items
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some(status))
        .count()
}

fn count_issue(items: &[Value], issue: &str) -> usize {
    items
        .iter()
        .filter(|item| {
            item["issues"]
                .as_array()
                .is_some_and(|issues| issues.iter().any(|value| value.as_str() == Some(issue)))
        })
        .count()
}

fn date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

fn default_limit() -> usize {
    DEFAULT_HEALTH_LIMIT
}

fn default_failure_policy() -> String {
    "advisory".to_string()
}
