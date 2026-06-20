use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use super::{new_id, now, Store};

const MAX_LANDING_JSON_BYTES: usize = 256 * 1024;

impl Store {
    pub fn project_landing_snapshot(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<Option<Value>> {
        let conn = self.conn()?;
        let landing_json: Option<String> = conn
            .query_row(
                "SELECT p.landing_json
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1
                   AND pm.user_id = ?2
                   AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
        landing_json.as_deref().map(parse_snapshot).transpose()
    }

    pub fn update_project_landing_snapshot(
        &self,
        user_id: &str,
        project_id: &str,
        landing: &Value,
    ) -> Result<Option<Value>> {
        let Some(snapshot) = crate::project_landing::normalize_landing_snapshot(landing) else {
            return Ok(None);
        };
        let landing_json = serde_json::to_string(&snapshot)?;
        if landing_json.len() > MAX_LANDING_JSON_BYTES {
            anyhow::bail!(
                "项目首页 manifest 超过 {} KB",
                MAX_LANDING_JSON_BYTES / 1024
            );
        }

        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE projects
             SET landing_json = ?1,
                 updated_at = ?2
             WHERE id = ?3
               AND status != 'deleted'
               AND EXISTS (
                 SELECT 1 FROM project_members pm
                 WHERE pm.project_id = projects.id
                   AND pm.user_id = ?4
                   AND pm.role IN ('owner', 'admin', 'editor')
               )",
            params![landing_json, now, project_id, user_id],
        )?;
        if changed == 0 {
            anyhow::bail!("项目不存在，或当前用户无权更新项目首页");
        }
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_landing_snapshot_updated', ?4, ?5)",
            params![
                new_id("evt"),
                project_id,
                user_id,
                serde_json::json!({
                    "source_mode": snapshot
                        .get("source")
                        .and_then(|source| source.get("mode"))
                        .and_then(Value::as_str),
                    "download_count": snapshot
                        .get("downloads")
                        .and_then(Value::as_array)
                        .map(|items| items.len())
                        .unwrap_or(0),
                })
                .to_string(),
                now,
            ],
        )?;
        tx.commit()?;
        Ok(Some(snapshot))
    }

    pub fn update_project_landing_snapshot_with_upload_token(
        &self,
        project_id: &str,
        token_id: &str,
        landing: &Value,
    ) -> Result<Option<Value>> {
        let Some(snapshot) = crate::project_landing::normalize_landing_snapshot(landing) else {
            return Ok(None);
        };
        let landing_json = serde_json::to_string(&snapshot)?;
        if landing_json.len() > MAX_LANDING_JSON_BYTES {
            anyhow::bail!(
                "项目首页 manifest 超过 {} KB",
                MAX_LANDING_JSON_BYTES / 1024
            );
        }

        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE projects
             SET landing_json = ?1,
                 updated_at = ?2
             WHERE id = ?3
               AND status != 'deleted'",
            params![landing_json, now, project_id],
        )?;
        if changed == 0 {
            anyhow::bail!("项目不存在，无法更新项目首页");
        }
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, NULL, 'project_landing_snapshot_updated', ?3, ?4)",
            params![
                new_id("evt"),
                project_id,
                serde_json::json!({
                    "actor": "project_landing_upload_token",
                    "token_id": token_id,
                    "source_mode": snapshot
                        .get("source")
                        .and_then(|source| source.get("mode"))
                        .and_then(Value::as_str),
                    "download_count": snapshot
                        .get("downloads")
                        .and_then(Value::as_array)
                        .map(|items| items.len())
                        .unwrap_or(0),
                })
                .to_string(),
                now,
            ],
        )?;
        tx.commit()?;
        Ok(Some(snapshot))
    }
}

fn parse_snapshot(value: &str) -> Result<Value> {
    let parsed = serde_json::from_str::<Value>(value)?;
    crate::project_landing::normalize_landing_snapshot(&parsed)
        .ok_or_else(|| anyhow!("项目首页快照为空或格式无效"))
}
