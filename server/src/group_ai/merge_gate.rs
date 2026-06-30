use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    group_ai::{
        merge_git::{cleanup_merge_state, display, git_lines, run_git, run_shell, CommandOutcome},
        review_gate::{review_gate_summary_for_merge, ReviewGateSummary},
        types::{ProjectAiMatter, ProjectAiMergeRequest, UpdateMergeRequestRequest},
    },
    store::ProjectAccess,
    types::AppState,
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApplyMergeRequestBody {
    #[serde(default, alias = "commitMessage")]
    pub commit_message: Option<String>,
    #[serde(default, alias = "verificationCommands")]
    pub verification_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MergeGateReport {
    pub merge_request: ProjectAiMergeRequest,
    pub review_gate: ReviewGateSummary,
    pub target_workspace_path: Option<String>,
    pub source_worktree_path: Option<String>,
    pub source_ref: Option<String>,
    pub can_apply: bool,
    pub checks: Vec<MergeGateCheck>,
    pub verification_commands: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MergeApplyReport {
    pub merge_request: ProjectAiMergeRequest,
    pub gate: MergeGateReport,
    pub commit_sha: Option<String>,
    pub verification_results: Vec<MergeGateCheck>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MergeGateCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
    pub output: Vec<String>,
}

pub(crate) fn check_merge_gate(
    state: &AppState,
    access: &ProjectAccess,
    project_id: &str,
    matter_id: &str,
    merge_request_id: &str,
) -> Result<MergeGateReport> {
    let matter = require_matter(state, project_id, matter_id)?;
    let merge_request = require_merge_request(state, project_id, matter_id, merge_request_id)?;
    let review_gate =
        review_gate_summary_for_merge(state, project_id, matter_id, merge_request_id)?;
    let mut checks = Vec::new();
    let mut warnings = Vec::new();
    let target = clean_path(access.workspace_path.as_deref())
        .or_else(|| clean_path(access.storage_worktree_path.as_deref()));
    let source = clean_path(merge_request.worktree_path.as_deref());
    let target_path = target.as_deref().map(Path::new);
    let source_path = source.as_deref().map(Path::new);

    let target_ok = check_git_dir("target_workspace", target_path.as_deref(), &mut checks);
    let source_ok = check_git_dir("source_worktree", source_path.as_deref(), &mut checks);
    let source_ref = target_path
        .as_deref()
        .filter(|_| target_ok)
        .and_then(|target| {
            resolve_source_ref(
                target,
                source_path.as_deref(),
                merge_request.branch_name.as_deref(),
                source_ok,
                &mut checks,
            )
        });
    let target_clean = target_path
        .as_deref()
        .map(|path| check_clean_worktree(path, &mut checks))
        .unwrap_or(false);
    warn_if_source_has_uncommitted_changes(source_path.as_deref(), &mut checks, &mut warnings);
    let dry_run_ok = if target_clean {
        match (target_path.as_deref(), source_ref.as_deref()) {
            (Some(target), Some(source_ref)) => dry_run_merge(target, source_ref, &mut checks),
            _ => false,
        }
    } else {
        false
    };
    let verification_commands = merge_verification_commands(&matter, &[]);
    let can_apply = dry_run_ok
        && review_gate.status != "blocked"
        && matches!(merge_request.status.as_str(), "open" | "approved")
        && target_clean;
    Ok(MergeGateReport {
        merge_request,
        review_gate,
        target_workspace_path: target,
        source_worktree_path: source,
        source_ref,
        can_apply,
        checks,
        verification_commands,
        warnings,
    })
}

pub(crate) fn apply_merge_request(
    state: &AppState,
    access: &ProjectAccess,
    project_id: &str,
    matter_id: &str,
    merge_request_id: &str,
    actor_user_id: &str,
    body: ApplyMergeRequestBody,
) -> Result<MergeApplyReport> {
    let matter = require_matter(state, project_id, matter_id)?;
    let gate = check_merge_gate(state, access, project_id, matter_id, merge_request_id)?;
    if !gate.can_apply {
        bail!("合并门禁未通过，不能执行合并");
    }
    let target = gate
        .target_workspace_path
        .as_deref()
        .map(Path::new)
        .ok_or_else(|| anyhow!("缺少目标 workspace"))?;
    let source_ref = gate
        .source_ref
        .as_deref()
        .ok_or_else(|| anyhow!("缺少可合并 source ref"))?;
    let message = clean_text(body.commit_message.as_deref()).unwrap_or_else(|| {
        format!(
            "Merge group AI assignment {}",
            gate.merge_request.assignment_id
        )
    });
    let merge = run_git(target, &["merge", "--no-ff", "--no-commit", source_ref]);
    if !merge.success {
        cleanup_merge_state(target);
        record_merge_event(
            state,
            project_id,
            matter_id,
            actor_user_id,
            "merge_apply_failed",
            &gate.merge_request,
            &merge,
        );
        bail!("合并失败: {}", merge.detail);
    }

    let commands = merge_verification_commands(&matter, &body.verification_commands);
    let verification_results = run_verification_commands(target, &commands);
    if verification_results
        .iter()
        .any(|result| result.status == "failed")
    {
        cleanup_merge_state(target);
        let _ = state.store.update_project_ai_merge_request(
            project_id,
            matter_id,
            merge_request_id,
            UpdateMergeRequestRequest {
                status: Some("approved".to_string()),
                review_status: Some("verification_failed".to_string()),
                risk_level: Some("high".to_string()),
                notes: Some("合并验证失败，已回滚未提交 merge。".to_string()),
            },
        );
        bail!("合并验证失败，已回滚未提交 merge");
    }

    let commit = run_git(target, &["commit", "-m", &message]);
    if !commit.success {
        cleanup_merge_state(target);
        bail!("合并提交失败: {}", commit.detail);
    }
    let commit_sha = git_lines(target, &["rev-parse", "HEAD"])
        .ok()
        .and_then(|lines| lines.first().cloned());
    let updated = state.store.update_project_ai_merge_request(
        project_id,
        matter_id,
        merge_request_id,
        UpdateMergeRequestRequest {
            status: Some("merged".to_string()),
            review_status: Some(if verification_results.is_empty() {
                "merged_no_verification".to_string()
            } else {
                "verified".to_string()
            }),
            risk_level: Some("low".to_string()),
            notes: commit_sha
                .as_ref()
                .map(|sha| format!("已合并并提交: {sha}")),
        },
    )?;
    state.store.insert_project_ai_event(
        project_id,
        matter_id,
        Some(actor_user_id),
        "merge_request_applied",
        json!({
            "merge_request_id": merge_request_id,
            "assignment_id": updated.assignment_id,
            "commit_sha": commit_sha,
            "verification_results": verification_results,
        }),
    )?;
    crate::project_events::publish_group_ai_matter_event(
        state,
        project_id,
        matter_id,
        Some(actor_user_id),
        "merge_request_applied",
        "群体 AI 合并项已通过验证并执行本地合并。",
    );
    Ok(MergeApplyReport {
        merge_request: updated,
        gate,
        commit_sha,
        verification_results,
        message: "合并已完成并生成本地 merge commit。".to_string(),
    })
}

fn require_matter(state: &AppState, project_id: &str, matter_id: &str) -> Result<ProjectAiMatter> {
    state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))
}

fn require_merge_request(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    merge_request_id: &str,
) -> Result<ProjectAiMergeRequest> {
    let request = state
        .store
        .get_project_ai_merge_request(merge_request_id)?
        .ok_or_else(|| anyhow!("Merge request 不存在"))?;
    if request.project_id != project_id || request.matter_id != matter_id {
        bail!("Merge request 不属于当前 Matter");
    }
    Ok(request)
}

fn check_git_dir(name: &str, path: Option<&Path>, checks: &mut Vec<MergeGateCheck>) -> bool {
    let Some(path) = path else {
        checks.push(check(name, "failed", "路径为空", Vec::new()));
        return false;
    };
    if !path.is_dir() {
        checks.push(check(name, "failed", "路径不是目录", vec![display(path)]));
        return false;
    }
    let result = run_git(path, &["rev-parse", "--is-inside-work-tree"]);
    checks.push(check(
        name,
        if result.success { "passed" } else { "failed" },
        result.detail,
        result.output,
    ));
    result.success
}

fn resolve_source_ref(
    target: &Path,
    source: Option<&Path>,
    branch_name: Option<&str>,
    source_ok: bool,
    checks: &mut Vec<MergeGateCheck>,
) -> Option<String> {
    if let Some(branch) = clean_text(branch_name) {
        let branch_ref = format!("{branch}^{{commit}}");
        let verify = run_git(target, &["rev-parse", "--verify", branch_ref.as_str()]);
        if verify.success {
            checks.push(check(
                "source_ref",
                "passed",
                "目标仓库存在目标分支",
                vec![branch.clone()],
            ));
            return Some(branch);
        }
    }
    if !source_ok {
        checks.push(check(
            "source_ref",
            "failed",
            "源 worktree 不可用",
            Vec::new(),
        ));
        return None;
    }
    let source = source?;
    let head = run_git(source, &["rev-parse", "HEAD"]);
    if !head.success {
        checks.push(check("source_ref", "failed", head.detail, head.output));
        return None;
    }
    let sha = head.output.first()?.clone();
    let fetch = run_git(target, &["fetch", "--no-tags", &display(source), &sha]);
    checks.push(check(
        "source_fetch",
        if fetch.success { "passed" } else { "failed" },
        fetch.detail,
        fetch.output,
    ));
    fetch.success.then(|| "FETCH_HEAD".to_string())
}

fn check_clean_worktree(path: &Path, checks: &mut Vec<MergeGateCheck>) -> bool {
    match git_lines(path, &["status", "--porcelain"]) {
        Ok(lines) if lines.is_empty() => {
            checks.push(check(
                "target_clean",
                "passed",
                "目标工作区干净",
                Vec::new(),
            ));
            true
        }
        Ok(lines) => {
            checks.push(check(
                "target_clean",
                "failed",
                "目标工作区有未提交改动",
                lines,
            ));
            false
        }
        Err(error) => {
            checks.push(check("target_clean", "failed", error, Vec::new()));
            false
        }
    }
}

fn warn_if_source_has_uncommitted_changes(
    source_path: Option<&Path>,
    checks: &mut Vec<MergeGateCheck>,
    warnings: &mut Vec<String>,
) {
    let source_status = source_path
        .and_then(|path| git_lines(path, &["status", "--short"]).ok())
        .unwrap_or_default();
    if source_status.is_empty() {
        return;
    }
    warnings.push("源 worktree 仍有未提交改动，合并只会包含已提交 HEAD。".to_string());
    checks.push(MergeGateCheck {
        name: "source_status".to_string(),
        status: "warning".to_string(),
        detail: "源 worktree 有未提交改动".to_string(),
        output: source_status,
    });
}

fn dry_run_merge(path: &Path, source_ref: &str, checks: &mut Vec<MergeGateCheck>) -> bool {
    let result = run_git(path, &["merge", "--no-ff", "--no-commit", source_ref]);
    let success = result.success;
    checks.push(check(
        "merge_dry_run",
        if success { "passed" } else { "failed" },
        result.detail,
        result.output,
    ));
    cleanup_merge_state(path);
    success
}

fn run_verification_commands(path: &Path, commands: &[String]) -> Vec<MergeGateCheck> {
    if commands.is_empty() {
        let result = run_git(path, &["diff", "--check"]);
        return vec![check(
            "verification:git diff --check",
            if result.success { "passed" } else { "failed" },
            result.detail,
            result.output,
        )];
    }
    commands
        .iter()
        .take(3)
        .map(|command| run_shell_check(path, command))
        .collect()
}

fn run_shell_check(path: &Path, command: &str) -> MergeGateCheck {
    let command = command.chars().take(240).collect::<String>();
    let output = run_shell(path, &command);
    check(
        &format!("verification:{command}"),
        if output.success { "passed" } else { "failed" },
        output.detail,
        output.output,
    )
}

fn record_merge_event(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    event_type: &str,
    merge_request: &ProjectAiMergeRequest,
    outcome: &CommandOutcome,
) {
    let _ = state.store.insert_project_ai_event(
        project_id,
        matter_id,
        Some(actor_user_id),
        event_type,
        json!({
            "merge_request_id": merge_request.id,
            "assignment_id": merge_request.assignment_id,
            "detail": outcome.detail,
            "output": outcome.output
        }),
    );
    crate::project_events::publish_group_ai_matter_event(
        state,
        project_id,
        matter_id,
        Some(actor_user_id),
        event_type,
        "群体 AI 合并项执行失败，等待人工处理。",
    );
}

fn merge_verification_commands(
    matter: &ProjectAiMatter,
    override_commands: &[String],
) -> Vec<String> {
    let commands = if override_commands.is_empty() {
        matter
            .plan
            .get("verification_commands")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        override_commands.to_vec()
    };
    commands
        .into_iter()
        .map(|value| value.trim().chars().take(240).collect::<String>())
        .filter(|value| !value.is_empty())
        .take(3)
        .collect()
}

fn check(
    name: &str,
    status: &str,
    detail: impl Into<String>,
    output: Vec<String>,
) -> MergeGateCheck {
    MergeGateCheck {
        name: name.to_string(),
        status: status.to_string(),
        detail: detail.into(),
        output,
    }
}

fn clean_path(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
