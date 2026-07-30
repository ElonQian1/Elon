use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{
    clean_comment, ensure_not_terminal, require_detail, require_matter, write_channel_notice,
    write_event, MatterDetail,
};
use crate::{
    group_ai::types::{
        ProjectAiMatter, ProjectAiMatterAssignment, MATTER_STATUS_FAILED,
        MATTER_STATUS_REVIEW_READY, MATTER_STATUS_RUNNING,
    },
    types::AppState,
};

#[derive(Debug, Default)]
pub(crate) struct AssignmentActionInput<'a> {
    pub comment: Option<&'a str>,
    pub result_summary: Option<&'a str>,
    pub compute_call_id: Option<&'a str>,
    pub status: Option<&'a str>,
    pub accounting_status: Option<&'a str>,
    pub billed_cost_rmb_fen: Option<i64>,
    pub provider_earned_fen: Option<i64>,
}

pub(crate) fn complete_assignment(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    actor_user_id: &str,
    input: AssignmentActionInput<'_>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    let assignment = require_assignment(state, matter_id, assignment_id)?;
    let updated = state.store.update_project_ai_matter_assignment_status(
        assignment_id,
        "completed",
        input.result_summary.or(input.comment),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "assignment_completed",
        json!({
            "assignment_id": updated.id,
            "bot_id": updated.bot_id,
            "role": updated.role,
            "node_id": updated.node_id,
            "previous_status": assignment.status,
            "result_summary": clean_comment(input.result_summary),
            "compute_call_id": clean_comment(input.compute_call_id),
            "comment": clean_comment(input.comment)
        }),
    )?;
    maybe_record_compute_evidence(
        state,
        project_id,
        matter_id,
        actor_user_id,
        &updated,
        &input,
        "assignment_compute_evidence_recorded",
    )?;
    mark_review_ready_if_all_completed(state, project_id, matter_id, actor_user_id, &matter)?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn fail_assignment(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    actor_user_id: &str,
    input: AssignmentActionInput<'_>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    let assignment = require_assignment(state, matter_id, assignment_id)?;
    let updated = state.store.update_project_ai_matter_assignment_status(
        assignment_id,
        "failed",
        input.result_summary.or(input.comment),
    )?;
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_FAILED,
        Some(actor_user_id),
        Some("execution_failed"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "assignment_failed",
        json!({
            "assignment_id": updated.id,
            "bot_id": updated.bot_id,
            "role": updated.role,
            "node_id": updated.node_id,
            "previous_status": assignment.status,
            "comment": clean_comment(input.comment),
            "result_summary": clean_comment(input.result_summary)
        }),
    )?;
    maybe_record_compute_evidence(
        state,
        project_id,
        matter_id,
        actor_user_id,
        &updated,
        &input,
        "assignment_failure_compute_evidence_recorded",
    )?;
    write_channel_notice(
        state,
        &matter,
        actor_user_id,
        &format!(
            "Assignment {} 执行失败，Matter 已进入失败待恢复状态。",
            updated.role
        ),
    )?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn retry_assignment(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    actor_user_id: &str,
    input: AssignmentActionInput<'_>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    let assignment = require_assignment(state, matter_id, assignment_id)?;
    let updated = state.store.update_project_ai_matter_assignment_status(
        assignment_id,
        "planned",
        input.result_summary.or(input.comment),
    )?;
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_RUNNING,
        Some(actor_user_id),
        Some("retry_requested"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "assignment_retry_requested",
        json!({
            "assignment_id": updated.id,
            "bot_id": updated.bot_id,
            "role": updated.role,
            "node_id": updated.node_id,
            "previous_status": assignment.status,
            "branch_name": updated.branch_name,
            "comment": clean_comment(input.comment)
        }),
    )?;
    write_channel_notice(
        state,
        &matter,
        actor_user_id,
        &format!("Assignment {} 已请求重试，等待节点重新执行。", updated.role),
    )?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn record_assignment_settlement(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
    actor_user_id: &str,
    input: AssignmentActionInput<'_>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    let assignment = require_assignment(state, matter_id, assignment_id)?;
    let compute_call_id = clean_required_option(input.compute_call_id, "compute_call_id")?;
    let compute_run = state
        .store
        .get_node_compute_run_by_compute_call_id(&compute_call_id)?;
    if let Some(run) = compute_run.as_ref() {
        if run.node_id.as_str() != assignment.node_id.as_str() {
            anyhow::bail!("compute_call_id 与 Assignment 节点不匹配");
        }
    }
    let next_status = input
        .status
        .and_then(clean_status)
        .or_else(|| {
            compute_run
                .as_ref()
                .map(|run| assignment_status_from_run(&run.status))
        })
        .unwrap_or("settled");
    let updated = state.store.update_project_ai_matter_assignment_status(
        assignment_id,
        next_status,
        input.result_summary.or(input.comment),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "assignment_settlement_recorded",
        settlement_payload(&updated, &compute_call_id, compute_run.as_ref(), &input),
    )?;
    if let Err(error) = crate::task_settlement::capture_task_assignment(
        &state.store,
        project_id,
        matter_id,
        assignment_id,
        &compute_call_id,
    ) {
        tracing::warn!(
            project_id,
            matter_id,
            assignment_id,
            compute_call_id,
            error = %error,
            "failed to capture optional task shadow settlement facts"
        );
    }
    mark_review_ready_if_all_completed(state, project_id, matter_id, actor_user_id, &matter)?;
    require_detail(state, project_id, matter_id)
}

fn require_assignment(
    state: &AppState,
    matter_id: &str,
    assignment_id: &str,
) -> Result<ProjectAiMatterAssignment> {
    let assignment = state
        .store
        .get_project_ai_matter_assignment(assignment_id)?
        .ok_or_else(|| anyhow!("Matter assignment 不存在"))?;
    if assignment.matter_id != matter_id {
        anyhow::bail!("Matter assignment 不属于当前 Matter");
    }
    Ok(assignment)
}

fn mark_review_ready_if_all_completed(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    matter: &ProjectAiMatter,
) -> Result<()> {
    let assignments = state.store.list_project_ai_matter_assignments(matter_id)?;
    if assignments.is_empty()
        || assignments
            .iter()
            .any(|assignment| !is_finished_assignment_status(&assignment.status))
    {
        return Ok(());
    }
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_REVIEW_READY,
        Some(actor_user_id),
        Some("review_ready"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "matter_review_ready",
        json!({ "assignment_count": assignments.len() }),
    )?;
    write_channel_notice(
        state,
        matter,
        actor_user_id,
        "全部 Assignment 已完成，等待人工验收。",
    )?;
    Ok(())
}

fn maybe_record_compute_evidence(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    assignment: &ProjectAiMatterAssignment,
    input: &AssignmentActionInput<'_>,
    event_type: &str,
) -> Result<()> {
    let Some(compute_call_id) = input
        .compute_call_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let compute_run = state
        .store
        .get_node_compute_run_by_compute_call_id(compute_call_id)?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        event_type,
        settlement_payload(assignment, compute_call_id, compute_run.as_ref(), input),
    )?;
    Ok(())
}

fn settlement_payload(
    assignment: &ProjectAiMatterAssignment,
    compute_call_id: &str,
    compute_run: Option<&crate::store::NodeComputeRun>,
    input: &AssignmentActionInput<'_>,
) -> Value {
    json!({
        "assignment_id": assignment.id,
        "bot_id": assignment.bot_id,
        "role": assignment.role,
        "node_id": assignment.node_id,
        "status": assignment.status,
        "compute_call_id": compute_call_id,
        "compute_run_found": compute_run.is_some(),
        "compute_run": compute_run.map(|run| json!({
            "status": run.status.as_str(),
            "settlement_status": run.settlement_status.as_deref(),
            "prompt_tokens": run.prompt_tokens,
            "completion_tokens": run.completion_tokens,
            "billed_cost_rmb_fen": run.billed_cost_rmb_fen,
            "provider_earned_fen": run.provider_earned_fen,
            "error_message": run.error_message.as_deref()
        })),
        "accounting_status": clean_comment(input.accounting_status),
        "billed_cost_rmb_fen": input.billed_cost_rmb_fen,
        "provider_earned_fen": input.provider_earned_fen,
        "comment": clean_comment(input.comment)
    })
}

fn is_finished_assignment_status(status: &str) -> bool {
    matches!(status, "completed" | "settled" | "settled_no_provider")
}

fn assignment_status_from_run(status: &str) -> &'static str {
    match status {
        "settled" | "deduplicated" => "settled",
        "settled_no_provider" => "settled_no_provider",
        "failed" | "settlement_failed" => "failed",
        _ => "completed",
    }
}

fn clean_required_option(value: Option<&str>, field: &str) -> Result<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("{field} 不能为空"))
}

fn clean_status(value: &str) -> Option<&'static str> {
    match value.trim() {
        "completed" => Some("completed"),
        "settled" => Some("settled"),
        "settled_no_provider" => Some("settled_no_provider"),
        "failed" => Some("failed"),
        _ => None,
    }
}
