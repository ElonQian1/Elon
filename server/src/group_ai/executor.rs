use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::Arc;

use crate::{
    ai_cli::{
        run_pc_agent_workspace_capture, truncate_chars, AiCliRequestMode, NativeSessionScope,
        PcAgentWorkspaceCaptureRequest, PcAgentWorkspaceCaptureResult,
    },
    group_ai::{
        actions::{matter_detail, MatterDetail},
        execution_prompt::build_assignment_execution_prompt,
        execution_recording::{
            assignment_status_from_compute_run, compute_run_payload, insert_event, session_payload,
            write_channel_notice,
        },
        review_result::record_review_from_assignment_result,
        types::{
            ProjectAiMatter, ProjectAiMatterAssignment, RecordAssignmentArtifactInput,
            MATTER_STATUS_FAILED, MATTER_STATUS_REVIEW_READY, MATTER_STATUS_RUNNING,
        },
    },
    store::{ProjectAccess, ProjectExecutionSession},
    types::AppState,
};

pub(crate) fn schedule_assignment_run(
    state: Arc<AppState>,
    access: ProjectAccess,
    matter: ProjectAiMatter,
    assignment: ProjectAiMatterAssignment,
    actor_user_id: String,
    comment: Option<String>,
) -> Result<MatterDetail> {
    if assignment.runtime_route != "pc_node_cli" {
        anyhow::bail!("第一版群体 AI 执行只支持 pc_node_cli Assignment");
    }
    if !assignment_can_be_dispatched(&assignment.status) {
        anyhow::bail!("当前 Assignment 状态不能重复执行");
    }

    let workspace_path = resolve_workspace_path(&access, &assignment)?;
    let runtime_permission =
        runtime_permission_for_assignment(&state, &access, &matter.project_id, &assignment)?;
    let prompt = build_assignment_execution_prompt(
        &matter,
        &assignment,
        &workspace_path,
        &runtime_permission,
    );
    let running_summary = comment
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("已派发到 PC 节点执行，等待节点返回结果。");

    state.store.update_project_ai_matter_assignment_execution(
        &assignment.id,
        "running",
        Some(running_summary),
        Some(&workspace_path),
        assignment.branch_name.as_deref(),
    )?;
    state.store.update_project_ai_matter_status(
        &matter.project_id,
        &matter.id,
        MATTER_STATUS_RUNNING,
        Some(&actor_user_id),
        Some("assignment_dispatched"),
    )?;
    insert_event(
        &state,
        &matter,
        Some(&actor_user_id),
        "assignment_dispatch_requested",
        json!({
            "assignment_id": assignment.id,
            "role": assignment.role,
            "node_id": assignment.node_id,
            "cli": assignment.cli_name,
            "workspace_path": workspace_path,
            "runtime_permission": runtime_permission,
            "comment": comment
        }),
    );
    write_channel_notice(
        &state,
        &matter,
        &actor_user_id,
        &format!(
            "Assignment {} 已派发到节点 {}，等待执行结果。",
            assignment.role, assignment.node_id
        ),
    );

    let detail_project_id = matter.project_id.clone();
    let detail_matter_id = matter.id.clone();
    let job = AssignmentRunJob {
        state: state.clone(),
        project_id: matter.project_id.clone(),
        matter_id: matter.id.clone(),
        actor_user_id,
        matter,
        assignment,
        workspace_path,
        runtime_permission,
        prompt,
    };
    tokio::spawn(async move {
        execute_assignment(job).await;
    });

    matter_detail(&state, &detail_project_id, &detail_matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))
}

#[derive(Clone)]
struct AssignmentRunJob {
    state: Arc<AppState>,
    project_id: String,
    matter_id: String,
    actor_user_id: String,
    matter: ProjectAiMatter,
    assignment: ProjectAiMatterAssignment,
    workspace_path: String,
    runtime_permission: String,
    prompt: String,
}

async fn execute_assignment(job: AssignmentRunJob) {
    insert_event(
        &job.state,
        &job.matter,
        Some(&job.actor_user_id),
        "assignment_execution_started",
        json!({
            "assignment_id": job.assignment.id,
            "node_id": job.assignment.node_id,
            "cli": job.assignment.cli_name,
            "workspace_path": job.workspace_path,
            "runtime_permission": job.runtime_permission
        }),
    );

    let capture = run_pc_agent_workspace_capture(
        PcAgentWorkspaceCaptureRequest {
            agent_id: job.assignment.node_id.clone(),
            user_id: job.actor_user_id.clone(),
            workspace_path: job.workspace_path.clone(),
            user_message: job.prompt.clone(),
            preflight_note: Some(
                "群体 AI Assignment 自动派发：执行后只回报产物和验证，不 push、不部署。"
                    .to_string(),
            ),
            request_mode: AiCliRequestMode::Execute,
            native_session_scope: Some(NativeSessionScope {
                project_id: job.project_id.clone(),
                user_id: job.actor_user_id.clone(),
                conversation_id: format!("group-ai:{}:{}", job.matter_id, job.assignment.id),
                runtime_permission: job.runtime_permission.clone(),
            }),
            cli_name: job.assignment.cli_name.clone(),
            copilot_model: None,
            codex_reasoning_effort: None,
            model_label: Some(job.assignment.cli_name.clone()),
        },
        &job.state,
    )
    .await;

    if capture.success {
        finish_assignment_success(&job, capture);
    } else {
        finish_assignment_failure(&job, capture);
    }
}

fn finish_assignment_success(job: &AssignmentRunJob, capture: PcAgentWorkspaceCaptureResult) {
    let session = execution_session(&job.state, capture.pc_req_id.as_deref());
    let compute_run = capture
        .compute_call_id
        .as_deref()
        .and_then(|compute_call_id| {
            job.state
                .store
                .get_node_compute_run_by_compute_call_id(compute_call_id)
                .ok()
                .flatten()
        });
    let status = assignment_status_from_compute_run(compute_run.as_ref());
    let summary = result_summary_from_capture(&capture);
    let worktree_path = session
        .as_ref()
        .and_then(|session| session.active_workspace_path.as_deref())
        .unwrap_or(job.workspace_path.as_str());
    let branch_name = session
        .as_ref()
        .and_then(|session| session.branch.as_deref())
        .or(job.assignment.branch_name.as_deref());

    match job
        .state
        .store
        .update_project_ai_matter_assignment_execution(
            &job.assignment.id,
            status,
            Some(&summary),
            Some(worktree_path),
            branch_name,
        ) {
        Ok(updated) => {
            insert_event(
                &job.state,
                &job.matter,
                Some(&job.actor_user_id),
                "assignment_execution_completed",
                json!({
                    "assignment_id": updated.id,
                    "role": updated.role,
                    "node_id": updated.node_id,
                    "cli": updated.cli_name,
                    "status": updated.status,
                    "compute_call_id": capture.compute_call_id,
                    "pc_req_id": capture.pc_req_id,
                    "model_used": capture.model_used,
                    "stream_event_count": capture.event_count,
                    "worktree_path": updated.worktree_path,
                    "branch_name": updated.branch_name,
                    "execution_session": session_payload(session.as_ref()),
                    "compute_run": compute_run_payload(compute_run.as_ref()),
                    "result_excerpt": truncate_chars(summary.as_str(), 1600),
                    "manual_merge_required": true
                }),
            );
            write_channel_notice(
                &job.state,
                &job.matter,
                &job.actor_user_id,
                &format!(
                    "Assignment {} 已完成，产物已登记，等待人工审核/合并。",
                    updated.role
                ),
            );
            record_execution_artifact(
                job,
                &updated,
                &summary,
                worktree_path,
                branch_name,
                capture.compute_call_id.as_deref(),
                session.as_ref(),
                compute_run.as_ref(),
            );
            record_review_from_assignment_result(
                &job.state,
                &job.matter,
                &updated,
                &job.actor_user_id,
                &summary,
            );
            mark_review_ready_if_all_finished(job);
        }
        Err(error) => {
            tracing::warn!(
                assignment_id = job.assignment.id,
                "Assignment 完成状态回写失败: {error:#}"
            );
        }
    }
}

fn finish_assignment_failure(job: &AssignmentRunJob, capture: PcAgentWorkspaceCaptureResult) {
    let error_message = capture
        .error_message
        .as_deref()
        .unwrap_or("PC 节点执行失败，未返回错误详情");
    let summary = truncate_chars(error_message, 4000);
    if let Err(error) = job
        .state
        .store
        .update_project_ai_matter_assignment_execution(
            &job.assignment.id,
            "failed",
            Some(&summary),
            Some(&job.workspace_path),
            job.assignment.branch_name.as_deref(),
        )
    {
        tracing::warn!(
            assignment_id = job.assignment.id,
            "Assignment 失败状态回写失败: {error:#}"
        );
    }
    if let Err(error) = job.state.store.update_project_ai_matter_status(
        &job.project_id,
        &job.matter_id,
        MATTER_STATUS_FAILED,
        Some(&job.actor_user_id),
        Some("assignment_execution_failed"),
    ) {
        tracing::warn!(
            matter_id = job.matter_id,
            "Matter 失败状态回写失败: {error:#}"
        );
    }
    insert_event(
        &job.state,
        &job.matter,
        Some(&job.actor_user_id),
        "assignment_execution_failed",
        json!({
            "assignment_id": job.assignment.id,
            "role": job.assignment.role,
            "node_id": job.assignment.node_id,
            "cli": job.assignment.cli_name,
            "compute_call_id": capture.compute_call_id,
            "pc_req_id": capture.pc_req_id,
            "stream_event_count": capture.event_count,
            "progress": capture.progress_messages,
            "error_message": error_message
        }),
    );
    write_channel_notice(
        &job.state,
        &job.matter,
        &job.actor_user_id,
        &format!(
            "Assignment {} 执行失败，Matter 已进入失败状态。",
            job.assignment.role
        ),
    );
}

fn resolve_workspace_path(
    access: &ProjectAccess,
    assignment: &ProjectAiMatterAssignment,
) -> Result<String> {
    [
        assignment.worktree_path.as_deref(),
        access.workspace_path.as_deref(),
        access.storage_worktree_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|value| !value.is_empty())
    .map(ToOwned::to_owned)
    .ok_or_else(|| anyhow!("项目没有可派发到 PC 节点的 workspace_path"))
}

fn runtime_permission_for_assignment(
    state: &AppState,
    access: &ProjectAccess,
    project_id: &str,
    assignment: &ProjectAiMatterAssignment,
) -> Result<String> {
    let authorization = state
        .store
        .list_project_ai_node_authorizations(project_id)?
        .into_iter()
        .find(|auth| auth.node_id == assignment.node_id && auth.enabled)
        .ok_or_else(|| anyhow!("Assignment 节点未授权或已停用"))?;
    if !authorization.allowed_clis.is_empty()
        && !authorization
            .allowed_clis
            .iter()
            .any(|cli| cli == &assignment.cli_name)
    {
        anyhow::bail!("Assignment CLI 不在节点授权范围内");
    }
    Ok(if authorization.permission_level.trim().is_empty() {
        access.runtime_permission.clone()
    } else {
        authorization.permission_level
    })
}

fn execution_session(state: &AppState, pc_req_id: Option<&str>) -> Option<ProjectExecutionSession> {
    pc_req_id.and_then(|request_id| {
        state
            .store
            .get_project_execution_session_by_request_id(request_id)
            .ok()
            .flatten()
    })
}

fn result_summary_from_capture(capture: &PcAgentWorkspaceCaptureResult) -> String {
    if capture.transcript.trim().is_empty() {
        return "节点执行完成，但没有返回可展示的结果摘要。请查看执行事件和工作区 diff。"
            .to_string();
    }
    truncate_chars(capture.transcript.trim(), 4000)
}

fn record_execution_artifact(
    job: &AssignmentRunJob,
    assignment: &ProjectAiMatterAssignment,
    summary: &str,
    worktree_path: &str,
    branch_name: Option<&str>,
    compute_call_id: Option<&str>,
    session: Option<&ProjectExecutionSession>,
    compute_run: Option<&crate::store::NodeComputeRun>,
) {
    let metadata = json!({
        "schema": "project_ai.assignment_artifact.v1",
        "source": "pc_node_capture",
        "compute_call_id": compute_call_id,
        "execution_session_id": session.map(|session| session.id.as_str()),
        "compute_run_status": compute_run.map(|run| run.status.as_str()),
        "manual_merge_required": true
    });
    if let Err(error) =
        job.state
            .store
            .record_project_ai_assignment_artifact(RecordAssignmentArtifactInput {
                project_id: job.project_id.clone(),
                matter_id: job.matter_id.clone(),
                assignment_id: assignment.id.clone(),
                uploader_user_id: Some(job.actor_user_id.clone()),
                artifact_kind: "pc_node_execution_report".to_string(),
                summary: Some(summary.to_string()),
                worktree_path: Some(worktree_path.to_string()),
                branch_name: branch_name.map(ToOwned::to_owned),
                files: Vec::new(),
                diff_stat: Vec::new(),
                test_results: Vec::new(),
                metadata,
            })
    {
        tracing::warn!(
            assignment_id = assignment.id,
            "Assignment artifact 自动登记失败: {error:#}"
        );
    }
}

fn mark_review_ready_if_all_finished(job: &AssignmentRunJob) {
    let Ok(assignments) = job
        .state
        .store
        .list_project_ai_matter_assignments(&job.matter_id)
    else {
        return;
    };
    if assignments.is_empty()
        || assignments
            .iter()
            .any(|assignment| !is_finished_assignment_status(&assignment.status))
    {
        return;
    }
    if let Err(error) = job.state.store.update_project_ai_matter_status(
        &job.project_id,
        &job.matter_id,
        MATTER_STATUS_REVIEW_READY,
        Some(&job.actor_user_id),
        Some("review_ready"),
    ) {
        tracing::warn!(
            matter_id = job.matter_id,
            "Matter review_ready 状态回写失败: {error:#}"
        );
        return;
    }
    insert_event(
        &job.state,
        &job.matter,
        Some(&job.actor_user_id),
        "matter_review_ready",
        json!({
            "assignment_count": assignments.len(),
            "source": "assignment_executor"
        }),
    );
    write_channel_notice(
        &job.state,
        &job.matter,
        &job.actor_user_id,
        "全部 Assignment 已完成，等待人工验收。",
    );
}

fn is_finished_assignment_status(status: &str) -> bool {
    matches!(status, "completed" | "settled" | "settled_no_provider")
}

pub(crate) fn assignment_can_be_dispatched(status: &str) -> bool {
    matches!(status, "planned" | "failed")
}
