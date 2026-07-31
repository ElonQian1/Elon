use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::ai_resource_control::model::{default_policy, AiResourcePolicy, UpdateAiResourcePolicy};

use super::{now, Store};

impl Store {
    pub(crate) fn project_ai_resource_policy(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<AiResourcePolicy> {
        let stored = self
            .conn()?
            .query_row(
                "SELECT project_id, enabled_classes_json, priority_json, allow_fallback,
                        privacy_mode, max_estimated_unit_cost_micros,
                        updated_by_user_id, created_at, updated_at
                   FROM project_ai_resource_policies WHERE project_id = ?1",
                params![project_id.trim()],
                |row| {
                    Ok(AiResourcePolicy {
                        project_id: row.get(0)?,
                        enabled_classes: parse_json_array(row.get::<_, String>(1)?, 1)?,
                        priority: parse_json_array(row.get::<_, String>(2)?, 2)?,
                        allow_fallback: row.get::<_, i64>(3)? != 0,
                        privacy_mode: row.get(4)?,
                        max_estimated_unit_cost_micros: row.get(5)?,
                        updated_by_user_id: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(stored.unwrap_or_else(|| default_policy(project_id.trim(), user_id.trim())))
    }

    pub(crate) fn upsert_project_ai_resource_policy(
        &self,
        project_id: &str,
        user_id: &str,
        request: &UpdateAiResourcePolicy,
    ) -> Result<AiResourcePolicy> {
        let enabled_json = serde_json::to_string(&request.enabled_classes)?;
        let priority_json = serde_json::to_string(&request.priority)?;
        let timestamp = now();
        self.conn()?.execute(
            "INSERT INTO project_ai_resource_policies (
               project_id, enabled_classes_json, priority_json, allow_fallback,
               privacy_mode, max_estimated_unit_cost_micros,
               updated_by_user_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(project_id) DO UPDATE SET
               enabled_classes_json = excluded.enabled_classes_json,
               priority_json = excluded.priority_json,
               allow_fallback = excluded.allow_fallback,
               privacy_mode = excluded.privacy_mode,
               max_estimated_unit_cost_micros =
                 excluded.max_estimated_unit_cost_micros,
               updated_by_user_id = excluded.updated_by_user_id,
               updated_at = excluded.updated_at",
            params![
                project_id.trim(),
                enabled_json,
                priority_json,
                request.allow_fallback as i64,
                request.privacy_mode,
                request.max_estimated_unit_cost_micros,
                user_id.trim(),
                timestamp
            ],
        )?;
        self.project_ai_resource_policy(project_id, user_id)
    }
}

fn parse_json_array(value: String, index: usize) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
