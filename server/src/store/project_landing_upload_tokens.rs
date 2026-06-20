use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{hash_token, new_id, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct ProjectLandingUploadTokenRecord {
    pub id: String,
    pub project_id: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl Store {
    pub(crate) fn rotate_project_landing_upload_token(
        &self,
        project_id: &str,
        created_by: &str,
        token: &str,
    ) -> Result<ProjectLandingUploadTokenRecord> {
        let id = new_id("pltok");
        let now = now();
        let token_hash = hash_token(token);
        let created_by_value = if created_by == "local-owner" {
            None
        } else {
            Some(created_by)
        };
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND status != 'deleted')",
            params![project_id],
            |row| row.get(0),
        )?;
        if !exists {
            anyhow::bail!("项目不存在，无法生成首页上传凭证");
        }
        tx.execute(
            "INSERT INTO project_landing_upload_tokens
               (id, project_id, token_hash, created_by, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)
             ON CONFLICT(project_id) DO UPDATE SET
               id = excluded.id,
               token_hash = excluded.token_hash,
               created_by = excluded.created_by,
               created_at = excluded.created_at,
               last_used_at = NULL",
            params![id, project_id, token_hash, created_by_value, now],
        )?;
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_landing_upload_token_rotated', ?4, ?5)",
            params![
                new_id("evt"),
                project_id,
                created_by_value,
                serde_json::json!({
                    "token_id": id,
                })
                .to_string(),
                now,
            ],
        )?;
        tx.commit()?;
        Ok(ProjectLandingUploadTokenRecord {
            id,
            project_id: project_id.to_string(),
            created_at: now,
            last_used_at: None,
        })
    }

    pub(crate) fn authenticate_project_landing_upload_token(
        &self,
        project_id: &str,
        token: &str,
    ) -> Result<Option<ProjectLandingUploadTokenRecord>> {
        let token_hash = hash_token(token);
        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let record = tx
            .query_row(
                "SELECT id, project_id, created_at, last_used_at
                 FROM project_landing_upload_tokens
                 WHERE project_id = ?1
                   AND token_hash = ?2",
                params![project_id, token_hash],
                |row| {
                    Ok(ProjectLandingUploadTokenRecord {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        created_at: row.get(2)?,
                        last_used_at: row.get(3)?,
                    })
                },
            )
            .optional()?;
        if record.is_some() {
            tx.execute(
                "UPDATE project_landing_upload_tokens
                 SET last_used_at = ?1
                 WHERE project_id = ?2
                   AND token_hash = ?3",
                params![now, project_id, token_hash],
            )?;
        }
        tx.commit()?;
        Ok(record)
    }
}
