use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};

mod assignment_actions;
pub(crate) use assignment_actions::{
    complete_assignment, fail_assignment, record_assignment_settlement, retry_assignment,
    AssignmentActionInput,
};

use crate::{
    group_ai::{
        learning::record_matter_decision_learning,
        review_gate::{ensure_matter_acceptance_ready, review_gate_summary},
        types::{
            CreateMatterAssignmentRecord, ProjectAiEvent, ProjectAiMatter,
            ProjectAiMatterAssignment, MATTER_STATUS_CANCELED, MATTER_STATUS_DONE,
            MATTER_STATUS_PLAN_READY, MATTER_STATUS_RUNNING,
        },
    },
    types::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct MatterDetail {
    pub matter: ProjectAiMatter,
    pub assignments: Vec<ProjectAiMatterAssignment>,
    pub events: Vec<ProjectAiEvent>,
}

pub(crate) fn matter_detail(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<Option<MatterDetail>> {
    let Some(matter) = state.store.get_project_ai_matter(project_id, matter_id)? else {
        return Ok(None);
    };
    let assignments = state.store.list_project_ai_matter_assignments(matter_id)?;
    let events = state
        .store
        .list_project_ai_matter_events(project_id, matter_id)?;
    Ok(Some(MatterDetail {
        matter,
        assignments,
        events,
    }))
}

pub(crate) fn approve_matter(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        &matter.status,
        Some(actor_user_id),
        Some("approved"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "matter_approved",
        json!({ "comment": clean_comment(comment) }),
    )?;
    write_channel_notice(
        state,
        &matter,
        actor_user_id,
        "群体 AI Matter 已批准，可以开始分配执行。",
    )?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn start_matter(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    if matter.status != MATTER_STATUS_PLAN_READY && matter.status != MATTER_STATUS_RUNNING {
        anyhow::bail!("当前 Matter 状态不能启动");
    }

    let mut assignments = state.store.list_project_ai_matter_assignments(matter_id)?;
    if assignments.is_empty() {
        let records = assignment_records_from_plan(&matter)?;
        if records.is_empty() {
            anyhow::bail!("Matter 计划中没有可分配的 Bot");
        }
        for record in records {
            assignments.push(state.store.create_project_ai_matter_assignment(record)?);
        }
        write_event(
            state,
            project_id,
            matter_id,
            actor_user_id,
            "assignments_created",
            json!({ "count": assignments.len() }),
        )?;
    }

    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_RUNNING,
        Some(actor_user_id),
        Some("started"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "matter_started",
        json!({
            "comment": clean_comment(comment),
            "assignment_count": assignments.len(),
            "dispatch_state": "assignments_ready"
        }),
    )?;
    write_channel_notice(
        state,
        &matter,
        actor_user_id,
        &format!(
            "群体 AI Matter 已启动，已生成 {} 个 Bot assignment。",
            assignments.len()
        ),
    )?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn request_changes(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    ensure_not_terminal(&matter)?;
    let gate = review_gate_summary(state, project_id, matter_id).ok();
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_PLAN_READY,
        Some(actor_user_id),
        Some("changes_requested"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "changes_requested",
        json!({ "comment": clean_comment(comment) }),
    )?;
    record_matter_decision_learning(
        state,
        &matter,
        actor_user_id,
        "changes_requested",
        comment,
        gate.as_ref(),
    );
    write_channel_notice(
        state,
        &matter,
        actor_user_id,
        "群体 AI Matter 已打回，需要调整计划或继续补充要求。",
    )?;
    require_detail(state, project_id, matter_id)
}

pub(crate) fn accept_matter(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    if matter.status == MATTER_STATUS_CANCELED {
        anyhow::bail!("已取消的 Matter 不能验收");
    }
    let gate = ensure_matter_acceptance_ready(state, project_id, matter_id)?;
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_DONE,
        Some(actor_user_id),
        Some("accepted"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "matter_accepted",
        json!({ "comment": clean_comment(comment), "review_gate": &gate }),
    )?;
    record_matter_decision_learning(
        state,
        &matter,
        actor_user_id,
        "accepted",
        comment,
        Some(&gate),
    );
    write_channel_notice(state, &matter, actor_user_id, "群体 AI Matter 已验收完成。")?;
    if let Err(error) =
        crate::task_settlement::post_accepted_matter(&state.store, project_id, matter_id)
    {
        tracing::warn!(
            project_id,
            matter_id,
            error = %error,
            "failed to post optional task shadow settlement"
        );
    }
    require_detail(state, project_id, matter_id)
}

pub(crate) fn cancel_matter(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    comment: Option<&str>,
) -> Result<MatterDetail> {
    let matter = require_matter(state, project_id, matter_id)?;
    if matter.status == MATTER_STATUS_DONE {
        anyhow::bail!("已完成的 Matter 不能取消");
    }
    state.store.update_project_ai_matter_status(
        project_id,
        matter_id,
        MATTER_STATUS_CANCELED,
        Some(actor_user_id),
        Some("canceled"),
    )?;
    write_event(
        state,
        project_id,
        matter_id,
        actor_user_id,
        "matter_canceled",
        json!({ "comment": clean_comment(comment) }),
    )?;
    write_channel_notice(state, &matter, actor_user_id, "群体 AI Matter 已取消。")?;
    if let Err(error) =
        crate::task_settlement::void_canceled_matter(&state.store, project_id, matter_id)
    {
        tracing::warn!(
            project_id,
            matter_id,
            error = %error,
            "failed to void optional task shadow settlement"
        );
    }
    require_detail(state, project_id, matter_id)
}

pub(super) fn require_matter(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<ProjectAiMatter> {
    state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))
}

pub(super) fn require_detail(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<MatterDetail> {
    matter_detail(state, project_id, matter_id)?.ok_or_else(|| anyhow!("Matter 不存在"))
}

pub(super) fn ensure_not_terminal(matter: &ProjectAiMatter) -> Result<()> {
    if matter.status == MATTER_STATUS_DONE || matter.status == MATTER_STATUS_CANCELED {
        anyhow::bail!("Matter 已结束，不能继续操作");
    }
    Ok(())
}

fn assignment_records_from_plan(
    matter: &ProjectAiMatter,
) -> Result<Vec<CreateMatterAssignmentRecord>> {
    let roles = matter
        .plan
        .get("roles")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Matter 计划缺少 roles"))?;
    let mut records = Vec::new();
    for (index, role) in roles.iter().enumerate() {
        let role_name = string_field(role, "role").unwrap_or("parallel_worker");
        let bot_id = string_field(role, "bot_id").ok_or_else(|| anyhow!("计划角色缺少 bot_id"))?;
        let provider_user_id = string_field(role, "provider_user_id")
            .ok_or_else(|| anyhow!("计划角色缺少 provider_user_id"))?;
        let node_id =
            string_field(role, "node_id").ok_or_else(|| anyhow!("计划角色缺少 node_id"))?;
        let runtime_route = string_field(role, "runtime_route")
            .ok_or_else(|| anyhow!("计划角色缺少 runtime_route"))?;
        let cli_name =
            string_field(role, "cli_name").ok_or_else(|| anyhow!("计划角色缺少 cli_name"))?;
        records.push(CreateMatterAssignmentRecord {
            matter_id: matter.id.clone(),
            bot_id: bot_id.to_string(),
            assignee_user_id: Some(provider_user_id.to_string()),
            provider_user_id: provider_user_id.to_string(),
            node_id: node_id.to_string(),
            role: role_name.to_string(),
            runtime_route: runtime_route.to_string(),
            cli_name: cli_name.to_string(),
            worktree_path: None,
            branch_name: Some(
                string_field(role, "branch_name")
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| {
                        format!(
                            "group-ai/{}-{}-{}",
                            matter.id,
                            index + 1,
                            branch_slug(role_name)
                        )
                    }),
            ),
            status: "planned".to_string(),
        });
    }
    Ok(records)
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn write_event(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    actor_user_id: &str,
    event_type: &str,
    payload: Value,
) -> Result<ProjectAiEvent> {
    let event = state.store.insert_project_ai_event(
        project_id,
        matter_id,
        Some(actor_user_id),
        event_type,
        payload,
    )?;
    crate::project_events::publish_group_ai_matter_event(
        state,
        project_id,
        matter_id,
        Some(actor_user_id),
        event_type,
        "群体 AI Matter 状态已更新。",
    );
    Ok(event)
}

pub(super) fn write_channel_notice(
    state: &AppState,
    matter: &ProjectAiMatter,
    actor_user_id: &str,
    message: &str,
) -> Result<()> {
    let content = format!("{}：{}", matter.title, message);
    state.store.insert_project_channel_message(
        &matter.project_id,
        &matter.channel_id,
        Some(actor_user_id),
        "ai_progress",
        &content,
        None,
        None,
    )?;
    Ok(())
}

pub(super) fn clean_comment(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn branch_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(32)
        .collect::<String>();
    if slug.is_empty() {
        "worker".to_string()
    } else {
        slug
    }
}
