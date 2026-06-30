use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::json;
use std::{collections::HashSet, sync::Arc};

use crate::{
    group_ai::{
        actions::{matter_detail, start_matter, MatterDetail},
        execution_recording::insert_event,
        executor::{assignment_can_be_dispatched, schedule_assignment_run},
        governance::budget_dispatch_blocker,
        types::{
            CreateMatterAssignmentRecord, ProjectAiBot, ProjectAiMatterAssignment,
            MATTER_STATUS_CANCELED, MATTER_STATUS_DONE, MATTER_STATUS_PLAN_READY,
        },
    },
    store::ProjectAccess,
    types::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct AutomationRunResult {
    pub detail: MatterDetail,
    pub scheduled_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<AssignmentDispatchError>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AssignmentDispatchError {
    pub assignment_id: String,
    pub role: String,
    pub reason: String,
}

pub(crate) fn schedule_matter_assignments(
    state: Arc<AppState>,
    access: ProjectAccess,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<String>,
) -> Result<AutomationRunResult> {
    let mut detail = ensure_assignments_ready(
        &state,
        project_id,
        matter_id,
        actor_user_id,
        comment.as_deref(),
    )?;
    ensure_budget_allows_dispatch(&state, &detail, actor_user_id)?;
    let candidates = detail.assignments.clone();
    let mut scheduled_count = 0;
    let mut skipped_count = 0;
    let mut errors = Vec::new();

    for assignment in candidates {
        if is_review_role(&assignment.role) {
            skipped_count += 1;
            continue;
        }
        if !assignment_can_be_dispatched(&assignment.status) {
            skipped_count += 1;
            continue;
        }
        match schedule_assignment_run(
            state.clone(),
            access.clone(),
            detail.matter.clone(),
            assignment.clone(),
            actor_user_id.to_string(),
            comment.clone(),
        ) {
            Ok(next) => {
                detail = next;
                scheduled_count += 1;
            }
            Err(error) => errors.push(dispatch_error(&assignment, error.to_string())),
        }
    }

    insert_event(
        &state,
        &detail.matter,
        Some(actor_user_id),
        "matter_dispatch_batch_finished",
        json!({
            "scheduled_count": scheduled_count,
            "skipped_count": skipped_count,
            "error_count": errors.len(),
            "errors": &errors,
        }),
    );
    detail =
        matter_detail(&state, project_id, matter_id)?.ok_or_else(|| anyhow!("Matter 不存在"))?;
    Ok(AutomationRunResult {
        detail,
        scheduled_count,
        skipped_count,
        errors,
    })
}

pub(crate) fn schedule_review_assignment(
    state: Arc<AppState>,
    access: ProjectAccess,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<String>,
    bots: &[ProjectAiBot],
) -> Result<AutomationRunResult> {
    let mut detail = ensure_assignments_ready(
        &state,
        project_id,
        matter_id,
        actor_user_id,
        comment.as_deref(),
    )?;
    ensure_budget_allows_dispatch(&state, &detail, actor_user_id)?;
    let assignment = match detail
        .assignments
        .iter()
        .find(|assignment| is_review_role(&assignment.role))
        .cloned()
    {
        Some(assignment) => assignment,
        None => {
            let created = create_review_assignment(&state, &detail.assignments, bots)?;
            insert_event(
                &state,
                &detail.matter,
                Some(actor_user_id),
                "review_assignment_created",
                json!({
                    "assignment_id": created.id,
                    "bot_id": created.bot_id,
                    "node_id": created.node_id,
                    "cli": created.cli_name,
                    "source": "review_request"
                }),
            );
            detail = matter_detail(&state, project_id, matter_id)?
                .ok_or_else(|| anyhow!("Matter 不存在"))?;
            created
        }
    };

    if !assignment_can_be_dispatched(&assignment.status) {
        insert_event(
            &state,
            &detail.matter,
            Some(actor_user_id),
            "review_dispatch_skipped",
            json!({
                "assignment_id": assignment.id,
                "role": assignment.role,
                "status": assignment.status,
                "reason": "review_assignment_not_dispatchable"
            }),
        );
        return Ok(AutomationRunResult {
            detail,
            scheduled_count: 0,
            skipped_count: 1,
            errors: Vec::new(),
        });
    }

    let dispatch_comment = comment.or_else(|| {
        Some("Review Bot 自动审核：检查实现产物、验证证据、风险和人工合并建议。".to_string())
    });
    let detail = schedule_assignment_run(
        state,
        access,
        detail.matter,
        assignment,
        actor_user_id.to_string(),
        dispatch_comment,
    )?;
    Ok(AutomationRunResult {
        detail,
        scheduled_count: 1,
        skipped_count: 0,
        errors: Vec::new(),
    })
}

fn ensure_budget_allows_dispatch(
    state: &AppState,
    detail: &MatterDetail,
    actor_user_id: &str,
) -> Result<()> {
    let Some(reason) = budget_dispatch_blocker(state, &detail.matter)? else {
        return Ok(());
    };
    insert_event(
        state,
        &detail.matter,
        Some(actor_user_id),
        "dispatch_blocked_by_budget",
        json!({ "reason": reason }),
    );
    anyhow::bail!("{reason}");
}

fn ensure_assignments_ready(
    state: &Arc<AppState>,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let mut detail =
        matter_detail(state, project_id, matter_id)?.ok_or_else(|| anyhow!("Matter 不存在"))?;
    if detail.matter.status == MATTER_STATUS_DONE || detail.matter.status == MATTER_STATUS_CANCELED
    {
        anyhow::bail!("Matter 已结束，不能继续派发");
    }
    if detail.matter.status == MATTER_STATUS_PLAN_READY || detail.assignments.is_empty() {
        detail = start_matter(state, project_id, matter_id, actor_user_id, comment)?;
    }
    Ok(detail)
}

fn create_review_assignment(
    state: &AppState,
    assignments: &[ProjectAiMatterAssignment],
    bots: &[ProjectAiBot],
) -> Result<ProjectAiMatterAssignment> {
    let template = assignments
        .first()
        .ok_or_else(|| anyhow!("没有可参考的 Assignment，无法创建 Review Bot"))?;
    let used_bot_ids: HashSet<&str> = assignments
        .iter()
        .map(|assignment| assignment.bot_id.as_str())
        .collect();
    let bot = bots
        .iter()
        .find(|bot| bot.online && !used_bot_ids.contains(bot.bot_id.as_str()))
        .or_else(|| bots.iter().find(|bot| bot.online));
    let record = if let Some(bot) = bot {
        CreateMatterAssignmentRecord {
            matter_id: template.matter_id.clone(),
            bot_id: bot.bot_id.clone(),
            assignee_user_id: Some(bot.provider_user_id.clone()),
            provider_user_id: bot.provider_user_id.clone(),
            node_id: bot.node_id.clone(),
            role: "reviewer".to_string(),
            runtime_route: bot.runtime_route.clone(),
            cli_name: bot.cli_name.clone(),
            worktree_path: None,
            branch_name: Some(format!("group-ai/{}-review", template.matter_id)),
            status: "planned".to_string(),
        }
    } else {
        CreateMatterAssignmentRecord {
            matter_id: template.matter_id.clone(),
            bot_id: format!("{}:review", template.bot_id),
            assignee_user_id: Some(template.provider_user_id.clone()),
            provider_user_id: template.provider_user_id.clone(),
            node_id: template.node_id.clone(),
            role: "reviewer".to_string(),
            runtime_route: template.runtime_route.clone(),
            cli_name: template.cli_name.clone(),
            worktree_path: None,
            branch_name: Some(format!("group-ai/{}-review", template.matter_id)),
            status: "planned".to_string(),
        }
    };
    state.store.create_project_ai_matter_assignment(record)
}

fn dispatch_error(
    assignment: &ProjectAiMatterAssignment,
    reason: String,
) -> AssignmentDispatchError {
    AssignmentDispatchError {
        assignment_id: assignment.id.clone(),
        role: assignment.role.clone(),
        reason,
    }
}

fn is_review_role(role: &str) -> bool {
    let role = role.trim().to_ascii_lowercase();
    role.contains("review") || role.contains("critic")
}
