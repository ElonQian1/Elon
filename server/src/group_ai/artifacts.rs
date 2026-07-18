use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

use crate::{
    group_ai::types::{
        ProjectAiAssignmentArtifact, ProjectAiEvent, ProjectAiMatterAssignment,
        ProjectAiMergeRequest,
    },
    store::{NodeComputeRun, NodeQualityScore, ProjectExecutionSession},
    types::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct AssignmentArtifact {
    pub assignment: ProjectAiMatterAssignment,
    pub latest_event: Option<ProjectAiEvent>,
    pub compute_call_id: Option<String>,
    pub pc_req_id: Option<String>,
    pub execution_session: Option<ProjectExecutionSession>,
    pub compute_run: Option<NodeComputeRun>,
    pub node_quality: Option<NodeQualityScore>,
    pub uploaded_artifacts: Vec<ProjectAiAssignmentArtifact>,
    pub merge_requests: Vec<ProjectAiMergeRequest>,
    pub merge: ArtifactMergeGuide,
    pub local_diff: LocalDiffProbe,
}

#[derive(Debug, Serialize)]
pub(crate) struct ArtifactMergeGuide {
    pub manual_merge_required: bool,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub merge_status: Option<String>,
    pub can_server_diff: bool,
    pub recommended_action: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct LocalDiffProbe {
    pub available: bool,
    pub reason: Option<String>,
    pub status_short: Vec<String>,
    pub diff_stat: Vec<String>,
}

pub(crate) fn assignment_artifact(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
) -> Result<AssignmentArtifact> {
    if state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .is_none()
    {
        anyhow::bail!("Matter 不存在");
    }
    let assignment = state
        .store
        .get_project_ai_matter_assignment(assignment_id)?
        .ok_or_else(|| anyhow!("Assignment 不存在"))?;
    if assignment.matter_id != matter_id {
        anyhow::bail!("Assignment 不属于当前 Matter");
    }
    let events = state
        .store
        .list_project_ai_matter_events(project_id, matter_id)?;
    let latest_event = latest_assignment_event(&events, assignment_id);
    let compute_call_id = latest_event
        .as_ref()
        .and_then(|event| payload_string(&event.payload, "compute_call_id"))
        .or_else(|| latest_payload_string(&events, assignment_id, "compute_call_id"));
    let pc_req_id = latest_event
        .as_ref()
        .and_then(|event| payload_string(&event.payload, "pc_req_id"))
        .or_else(|| latest_payload_string(&events, assignment_id, "pc_req_id"));
    let execution_session = pc_req_id.as_deref().and_then(|request_id| {
        state
            .store
            .get_project_execution_session_by_request_id(request_id)
            .ok()
            .flatten()
    });
    let compute_run = compute_call_id.as_deref().and_then(|id| {
        state
            .store
            .get_node_compute_run_by_compute_call_id(id)
            .ok()
            .flatten()
    });
    let node_quality = state
        .store
        .node_quality_scores()
        .ok()
        .and_then(|scores| scores.get(&assignment.node_id).cloned());
    let uploaded_artifacts =
        state
            .store
            .list_project_ai_assignment_artifacts(project_id, matter_id, assignment_id)?;
    let merge_requests = state
        .store
        .list_project_ai_merge_requests(project_id, matter_id)?
        .into_iter()
        .filter(|request| request.assignment_id == assignment_id)
        .collect::<Vec<_>>();
    let worktree_path = execution_session
        .as_ref()
        .and_then(|session| session.active_workspace_path.clone())
        .or_else(|| assignment.worktree_path.clone())
        .or_else(|| {
            latest_event
                .as_ref()
                .and_then(|event| payload_string(&event.payload, "worktree_path"))
        });
    let branch_name = execution_session
        .as_ref()
        .and_then(|session| session.branch.clone())
        .or_else(|| assignment.branch_name.clone());
    let local_diff = probe_local_diff(worktree_path.as_deref());
    let merge = ArtifactMergeGuide {
        manual_merge_required: manual_merge_required(
            &assignment.status,
            &worktree_path,
            &branch_name,
        ),
        worktree_path,
        branch_name,
        merge_status: execution_session
            .as_ref()
            .and_then(|session| session.merge_status.clone()),
        can_server_diff: local_diff.available,
        recommended_action: recommended_action(&assignment.status, local_diff.available),
    };
    Ok(AssignmentArtifact {
        assignment,
        latest_event,
        compute_call_id,
        pc_req_id,
        execution_session,
        compute_run,
        node_quality,
        uploaded_artifacts,
        merge_requests,
        merge,
        local_diff,
    })
}

fn latest_assignment_event(
    events: &[ProjectAiEvent],
    assignment_id: &str,
) -> Option<ProjectAiEvent> {
    events
        .iter()
        .rev()
        .find(|event| {
            payload_string(&event.payload, "assignment_id").as_deref() == Some(assignment_id)
        })
        .cloned()
}

fn latest_payload_string(
    events: &[ProjectAiEvent],
    assignment_id: &str,
    key: &str,
) -> Option<String> {
    events.iter().rev().find_map(|event| {
        if payload_string(&event.payload, "assignment_id").as_deref() == Some(assignment_id) {
            payload_string(&event.payload, key)
        } else {
            None
        }
    })
}

fn payload_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn probe_local_diff(worktree_path: Option<&str>) -> LocalDiffProbe {
    let Some(worktree_path) = worktree_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return diff_unavailable("assignment 暂无 worktree_path");
    };
    if !Path::new(worktree_path).is_dir() {
        return diff_unavailable("worktree_path 不在当前服务器文件系统上，需在节点 PC 上查看");
    }
    let status_short = match git_lines(worktree_path, &["status", "--short"]) {
        Ok(lines) => lines,
        Err(reason) => return diff_unavailable(reason),
    };
    let diff_stat = git_lines(worktree_path, &["diff", "--stat", "HEAD", "--"]).unwrap_or_default();
    LocalDiffProbe {
        available: true,
        reason: None,
        status_short,
        diff_stat,
    }
}

fn git_lines(worktree_path: &str, args: &[&str]) -> Result<Vec<String>, String> {
    let output = crate::git_command_error::git_command()
        .arg("-C")
        .arg(worktree_path)
        .args(args)
        .output()
        .map_err(|error| format!("git 执行失败: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git 返回失败: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .take(40)
        .map(|line| line.chars().take(240).collect())
        .collect())
}

fn diff_unavailable(reason: impl Into<String>) -> LocalDiffProbe {
    LocalDiffProbe {
        available: false,
        reason: Some(reason.into()),
        status_short: Vec::new(),
        diff_stat: Vec::new(),
    }
}

fn manual_merge_required(
    status: &str,
    worktree_path: &Option<String>,
    branch_name: &Option<String>,
) -> bool {
    matches!(status, "completed" | "settled" | "settled_no_provider")
        && (worktree_path.is_some() || branch_name.is_some())
}

fn recommended_action(status: &str, can_server_diff: bool) -> String {
    match (status, can_server_diff) {
        ("completed" | "settled" | "settled_no_provider", true) => {
            "查看 diff_stat/status_short，人工确认后从登记的 worktree/branch 合并。".to_string()
        }
        ("completed" | "settled" | "settled_no_provider", false) => {
            "到节点 PC 打开 worktree_path 或 branch_name 查看 diff，人工确认后合并。".to_string()
        }
        ("running", _) => "Assignment 仍在执行，等待完成事件后再合并。".to_string(),
        ("failed", _) => "先查看失败事件和节点输出，修复后重试该 Assignment。".to_string(),
        _ => "先执行该 Assignment，生成产物后再合并。".to_string(),
    }
}
