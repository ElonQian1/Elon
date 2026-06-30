use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};
use serde_json::Value;
use std::collections::BTreeSet;

use crate::group_ai::types::{
    CreateMatterRecord, ProjectAiMatter, ProjectAiNodeAuthorization,
    UpsertNodeAuthorizationRequest, MATTER_STATUS_PLAN_READY,
};

use super::{
    new_id, normalize_project_runtime_permission, now, Store,
    PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE,
};

impl Store {
    pub(crate) fn upsert_project_ai_node_authorization(
        &self,
        project_id: &str,
        provider_user_id: &str,
        created_by_user_id: &str,
        req: UpsertNodeAuthorizationRequest,
    ) -> Result<ProjectAiNodeAuthorization> {
        let project_id = clean_required(project_id, "project_id")?;
        let provider_user_id = clean_required(provider_user_id, "provider_user_id")?;
        let created_by_user_id = clean_required(created_by_user_id, "created_by_user_id")?;
        let node_id = clean_required(&req.node_id, "node_id")?;
        let permission_level = req
            .permission_level
            .as_deref()
            .and_then(normalize_project_runtime_permission)
            .unwrap_or(PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE)
            .to_string();
        let allowed_clis = normalize_clis(&req.allowed_clis);
        let enabled = req.enabled.unwrap_or(true);
        let ts = now();
        let authorization_id = new_id("pana");
        let allowed_clis_json = serde_json::to_string(&allowed_clis)?;

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_node_authorizations
               (id, project_id, provider_user_id, node_id, allowed_clis_json,
                permission_level, enabled, created_by_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(project_id, node_id) DO UPDATE SET
               provider_user_id = excluded.provider_user_id,
               allowed_clis_json = excluded.allowed_clis_json,
               permission_level = excluded.permission_level,
               enabled = excluded.enabled,
               updated_at = excluded.updated_at",
            params![
                authorization_id,
                project_id,
                provider_user_id,
                node_id,
                allowed_clis_json,
                permission_level,
                bool_to_i64(enabled),
                created_by_user_id,
                ts,
            ],
        )?;
        get_project_ai_node_authorization_by_node_locked(&conn, &project_id, &node_id)?
            .ok_or_else(|| anyhow!("节点授权保存失败"))
    }

    pub(crate) fn list_project_ai_node_authorizations(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAiNodeAuthorization>> {
        let project_id = clean_required(project_id, "project_id")?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, provider_user_id, node_id, allowed_clis_json,
                    permission_level, enabled, created_by_user_id, created_at, updated_at
               FROM project_ai_node_authorizations
              WHERE project_id = ?1
              ORDER BY enabled DESC, updated_at DESC",
        )?;
        let rows = stmt.query_map(params![project_id], project_ai_node_authorization_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn create_project_ai_matter(
        &self,
        record: CreateMatterRecord,
    ) -> Result<ProjectAiMatter> {
        let project_id = clean_required(&record.project_id, "project_id")?;
        let channel_id = clean_required(&record.channel_id, "channel_id")?;
        let requester_user_id = clean_required(&record.requester_user_id, "requester_user_id")?;
        let title = clean_required(&record.title, "title")?;
        let brief = clean_required(&record.brief, "brief")?;
        let matter_id = new_id("paim");
        let ts = now();
        let participant_user_ids_json =
            serde_json::to_string(&normalize_clis_like_values(&record.participant_user_ids))?;
        let node_policy_json = serde_json::to_string(&record.node_policy_json)?;
        let acceptance_criteria_json =
            serde_json::to_string(&normalize_texts(&record.acceptance_criteria))?;
        let plan_json = serde_json::to_string(&record.plan_json)?;

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO project_ai_matters
               (id, project_id, channel_id, requester_user_id, decision_user_id,
                source_message_id, title, brief, collaboration_mode, status,
                participant_user_ids_json, node_policy_json, acceptance_criteria_json,
                plan_json, final_summary, final_decision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, NULL, ?14, ?14)",
            params![
                matter_id,
                project_id,
                channel_id,
                requester_user_id,
                record.source_message_id,
                title,
                brief,
                record.collaboration_mode,
                MATTER_STATUS_PLAN_READY,
                participant_user_ids_json,
                node_policy_json,
                acceptance_criteria_json,
                plan_json,
                ts,
            ],
        )?;
        get_project_ai_matter_locked(&conn, &project_id, &matter_id)?
            .ok_or_else(|| anyhow!("Matter 保存失败"))
    }

    pub(crate) fn list_project_ai_matters(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ProjectAiMatter>> {
        let project_id = clean_required(project_id, "project_id")?;
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, channel_id, requester_user_id, decision_user_id,
                    source_message_id, title, brief, collaboration_mode, status,
                    participant_user_ids_json, node_policy_json, acceptance_criteria_json,
                    plan_json, final_summary, final_decision, created_at, updated_at
               FROM project_ai_matters
              WHERE project_id = ?1
              ORDER BY updated_at DESC
              LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit], project_ai_matter_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn get_project_ai_matter(
        &self,
        project_id: &str,
        matter_id: &str,
    ) -> Result<Option<ProjectAiMatter>> {
        let project_id = clean_required(project_id, "project_id")?;
        let matter_id = clean_required(matter_id, "matter_id")?;
        let conn = self.conn()?;
        get_project_ai_matter_locked(&conn, &project_id, &matter_id)
    }
}

fn get_project_ai_node_authorization_by_node_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    node_id: &str,
) -> Result<Option<ProjectAiNodeAuthorization>> {
    conn.query_row(
        "SELECT id, project_id, provider_user_id, node_id, allowed_clis_json,
                permission_level, enabled, created_by_user_id, created_at, updated_at
           FROM project_ai_node_authorizations
          WHERE project_id = ?1 AND node_id = ?2",
        params![project_id, node_id],
        project_ai_node_authorization_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn get_project_ai_matter_locked(
    conn: &rusqlite::Connection,
    project_id: &str,
    matter_id: &str,
) -> Result<Option<ProjectAiMatter>> {
    conn.query_row(
        "SELECT id, project_id, channel_id, requester_user_id, decision_user_id,
                source_message_id, title, brief, collaboration_mode, status,
                participant_user_ids_json, node_policy_json, acceptance_criteria_json,
                plan_json, final_summary, final_decision, created_at, updated_at
           FROM project_ai_matters
          WHERE project_id = ?1 AND id = ?2",
        params![project_id, matter_id],
        project_ai_matter_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn project_ai_node_authorization_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<ProjectAiNodeAuthorization> {
    let allowed_clis_json: String = row.get(4)?;
    let enabled: i64 = row.get(6)?;
    Ok(ProjectAiNodeAuthorization {
        id: row.get(0)?,
        project_id: row.get(1)?,
        provider_user_id: row.get(2)?,
        node_id: row.get(3)?,
        allowed_clis: parse_string_vec(&allowed_clis_json),
        permission_level: row.get(5)?,
        enabled: enabled != 0,
        created_by_user_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn project_ai_matter_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectAiMatter> {
    let participant_user_ids_json: String = row.get(10)?;
    let node_policy_json: String = row.get(11)?;
    let acceptance_criteria_json: String = row.get(12)?;
    let plan_json: String = row.get(13)?;
    Ok(ProjectAiMatter {
        id: row.get(0)?,
        project_id: row.get(1)?,
        channel_id: row.get(2)?,
        requester_user_id: row.get(3)?,
        decision_user_id: row.get(4)?,
        source_message_id: row.get(5)?,
        title: row.get(6)?,
        brief: row.get(7)?,
        collaboration_mode: row.get(8)?,
        status: row.get(9)?,
        participant_user_ids: parse_string_vec(&participant_user_ids_json),
        node_policy: parse_json_value(&node_policy_json),
        acceptance_criteria: parse_string_vec(&acceptance_criteria_json),
        plan: parse_json_value(&plan_json),
        final_summary: row.get(14)?,
        final_decision: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn clean_required(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{field} 不能为空");
    }
    Ok(value.to_string())
}

fn normalize_clis(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_clis_like_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_texts(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn parse_json_value(value: &str) -> Value {
    serde_json::from_str::<Value>(value).unwrap_or(Value::Null)
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
