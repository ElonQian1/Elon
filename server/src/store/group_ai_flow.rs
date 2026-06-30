use anyhow::{anyhow, Result};
use rusqlite::{params, Row};
use serde_json::Value;

use crate::group_ai::types::{
    CreateMatterAssignmentRecord, ProjectAiEvent, ProjectAiMatterAssignment,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn update_project_ai_matter_status(
        &self,
        project_id: &str,
        matter_id: &str,
        status: &str,
        decision_user_id: Option<&str>,
        final_decision: Option<&str>,
    ) -> Result<()> {
        let project_id = clean_required(project_id, "project_id")?;
        let matter_id = clean_required(matter_id, "matter_id")?;
        let status = clean_required(status, "status")?;
        let ts = now();
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE project_ai_matters
                SET status = ?3,
                    decision_user_id = COALESCE(?4, decision_user_id),
                    final_decision = COALESCE(?5, final_decision),
                    updated_at = ?6
              WHERE project_id = ?1 AND id = ?2",
            params![
                project_id,
                matter_id,
                status,
                clean_optional(decision_user_id),
                clean_optional(final_decision),
                ts,
            ],
        )?;
        if updated == 0 {
            anyhow::bail!("Matter 不存在");
        }
        Ok(())
    }

    pub(crate) fn create_project_ai_matter_assignment(
        &self,
        record: CreateMatterAssignmentRecord,
    ) -> Result<ProjectAiMatterAssignment> {
        let matter_id = clean_required(&record.matter_id, "matter_id")?;
        let bot_id = clean_required(&record.bot_id, "bot_id")?;
        let provider_user_id = clean_required(&record.provider_user_id, "provider_user_id")?;
        let node_id = clean_required(&record.node_id, "node_id")?;
        let role = clean_required(&record.role, "role")?;
        let runtime_route = clean_required(&record.runtime_route, "runtime_route")?;
        let cli_name = clean_required(&record.cli_name, "cli_name")?;
        let status = clean_required(&record.status, "status")?;
        let id = new_id("paia");
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_matter_assignments
               (id, matter_id, bot_id, assignee_user_id, provider_user_id, node_id,
                role, runtime_route, cli_name, worktree_path, branch_name, status,
                result_summary, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?13)",
            params![
                id,
                matter_id,
                bot_id,
                clean_optional(record.assignee_user_id.as_deref()),
                provider_user_id,
                node_id,
                role,
                runtime_route,
                cli_name,
                clean_optional(record.worktree_path.as_deref()),
                clean_optional(record.branch_name.as_deref()),
                status,
                ts,
            ],
        )?;
        drop(conn);
        self.get_project_ai_matter_assignment(&id)?
            .ok_or_else(|| anyhow!("Matter assignment 保存失败"))
    }

    pub(crate) fn get_project_ai_matter_assignment(
        &self,
        assignment_id: &str,
    ) -> Result<Option<ProjectAiMatterAssignment>> {
        let assignment_id = clean_required(assignment_id, "assignment_id")?;
        let conn = self.conn()?;
        let assignment = conn.query_row(
            assignment_select_sql("WHERE id = ?1").as_str(),
            params![assignment_id],
            project_ai_assignment_from_row,
        );
        match assignment {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn update_project_ai_matter_assignment_status(
        &self,
        assignment_id: &str,
        status: &str,
        result_summary: Option<&str>,
    ) -> Result<ProjectAiMatterAssignment> {
        let assignment_id = clean_required(assignment_id, "assignment_id")?;
        let status = clean_required(status, "status")?;
        let ts = now();
        let conn = self.conn()?;
        let updated = conn.execute(
            "UPDATE project_ai_matter_assignments
                SET status = ?2,
                    result_summary = COALESCE(?3, result_summary),
                    updated_at = ?4
              WHERE id = ?1",
            params![assignment_id, status, clean_optional(result_summary), ts],
        )?;
        if updated == 0 {
            anyhow::bail!("Matter assignment 不存在");
        }
        drop(conn);
        self.get_project_ai_matter_assignment(&assignment_id)?
            .ok_or_else(|| anyhow!("Matter assignment 更新后读取失败"))
    }

    pub(crate) fn list_project_ai_matter_assignments(
        &self,
        matter_id: &str,
    ) -> Result<Vec<ProjectAiMatterAssignment>> {
        let matter_id = clean_required(matter_id, "matter_id")?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            assignment_select_sql("WHERE matter_id = ?1 ORDER BY created_at ASC").as_str(),
        )?;
        let rows = stmt.query_map(params![matter_id], project_ai_assignment_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn insert_project_ai_event(
        &self,
        project_id: &str,
        matter_id: &str,
        actor_user_id: Option<&str>,
        event_type: &str,
        payload: Value,
    ) -> Result<ProjectAiEvent> {
        let project_id = clean_required(project_id, "project_id")?;
        let matter_id = clean_required(matter_id, "matter_id")?;
        let event_type = clean_required(event_type, "event_type")?;
        let payload_json = serde_json::to_string(&payload)?;
        let id = new_id("paie");
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_events
               (id, matter_id, project_id, actor_user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                matter_id,
                project_id,
                clean_optional(actor_user_id),
                event_type,
                payload_json,
                ts,
            ],
        )?;
        Ok(ProjectAiEvent {
            id,
            matter_id,
            project_id,
            actor_user_id: clean_optional(actor_user_id).map(ToOwned::to_owned),
            event_type,
            payload,
            created_at: ts,
        })
    }

    pub(crate) fn list_project_ai_matter_events(
        &self,
        project_id: &str,
        matter_id: &str,
    ) -> Result<Vec<ProjectAiEvent>> {
        let project_id = clean_required(project_id, "project_id")?;
        let matter_id = clean_required(matter_id, "matter_id")?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, matter_id, project_id, actor_user_id, event_type, payload_json, created_at
               FROM project_ai_events
              WHERE project_id = ?1 AND matter_id = ?2
              ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![project_id, matter_id], project_ai_event_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn assignment_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT id, matter_id, bot_id, assignee_user_id, provider_user_id, node_id,
                role, runtime_route, cli_name, worktree_path, branch_name, status,
                result_summary, created_at, updated_at
           FROM project_ai_matter_assignments
          {where_clause}"
    )
}

fn project_ai_assignment_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectAiMatterAssignment> {
    Ok(ProjectAiMatterAssignment {
        id: row.get(0)?,
        matter_id: row.get(1)?,
        bot_id: row.get(2)?,
        assignee_user_id: row.get(3)?,
        provider_user_id: row.get(4)?,
        node_id: row.get(5)?,
        role: row.get(6)?,
        runtime_route: row.get(7)?,
        cli_name: row.get(8)?,
        worktree_path: row.get(9)?,
        branch_name: row.get(10)?,
        status: row.get(11)?,
        result_summary: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn project_ai_event_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectAiEvent> {
    let payload_json: String = row.get(5)?;
    Ok(ProjectAiEvent {
        id: row.get(0)?,
        matter_id: row.get(1)?,
        project_id: row.get(2)?,
        actor_user_id: row.get(3)?,
        event_type: row.get(4)?,
        payload: serde_json::from_str(&payload_json).unwrap_or(Value::Null),
        created_at: row.get(6)?,
    })
}

fn clean_required(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{field} 不能为空");
    }
    Ok(value.to_string())
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_ai::types::CreateMatterRecord;
    use serde_json::json;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-group-ai-flow-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn assignment_status_and_events_are_durable() {
        let store = temp_store();
        let user = store
            .create_user("group-ai-flow@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Group AI Flow", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai development channel should exist");
        let matter = store
            .create_project_ai_matter(CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: user.id.clone(),
                source_message_id: None,
                title: "多 AI 协作".to_string(),
                brief: "验证 Matter assignment 状态流".to_string(),
                collaboration_mode: "critic".to_string(),
                participant_user_ids: vec![user.id.clone()],
                node_policy_json: json!({ "mode": "project_write" }),
                acceptance_criteria: vec!["事件可追踪".to_string()],
                plan_json: json!({ "roles": [] }),
            })
            .expect("matter should be created");
        let assignment = store
            .create_project_ai_matter_assignment(CreateMatterAssignmentRecord {
                matter_id: matter.id.clone(),
                bot_id: "bot:codex".to_string(),
                assignee_user_id: Some(user.id.clone()),
                provider_user_id: user.id.clone(),
                node_id: "node-a".to_string(),
                role: "critic".to_string(),
                runtime_route: "pc_node_cli".to_string(),
                cli_name: "codex".to_string(),
                worktree_path: None,
                branch_name: Some("group-ai/test".to_string()),
                status: "planned".to_string(),
            })
            .expect("assignment should be created");

        let updated = store
            .update_project_ai_matter_assignment_status(
                &assignment.id,
                "failed",
                Some("节点离线，等待重试"),
            )
            .expect("assignment should update");
        assert_eq!(updated.status, "failed");
        assert_eq!(
            updated.result_summary.as_deref(),
            Some("节点离线，等待重试")
        );

        store
            .insert_project_ai_event(
                &project.id,
                &matter.id,
                Some(&user.id),
                "assignment_failed",
                json!({ "assignment_id": assignment.id }),
            )
            .expect("event should insert");
        let events = store
            .list_project_ai_matter_events(&project.id, &matter.id)
            .expect("events should list");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "assignment_failed");
        assert_eq!(events[0].payload["assignment_id"], assignment.id);
    }
}
