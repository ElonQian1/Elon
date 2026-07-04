use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::{clean_optional, new_id, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRelease {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub uploaded_by: Option<String>,
    pub version_name: Option<String>,
    pub channel: String,
    pub status: String,
    pub apk_url: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub changelog: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectReleaseWrite<'a> {
    pub id: Option<&'a str>,
    pub project_id: &'a str,
    pub task_id: Option<&'a str>,
    pub uploaded_by: Option<&'a str>,
    pub version_name: Option<&'a str>,
    pub channel: Option<&'a str>,
    pub status: Option<&'a str>,
    pub apk_url: &'a str,
    pub file_name: &'a str,
    pub file_path: Option<&'a str>,
    pub sha256: Option<&'a str>,
    pub size_bytes: Option<i64>,
    pub changelog: Option<&'a str>,
}

impl Store {
    pub fn create_project_release(&self, write: ProjectReleaseWrite<'_>) -> Result<ProjectRelease> {
        let id = clean_optional(write.id)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| new_id("rel"));
        let channel = clean_optional(write.channel).unwrap_or("internal");
        let status = clean_optional(write.status).unwrap_or("published");
        if clean_optional(Some(write.apk_url)).is_none() {
            return Err(anyhow!("APK download URL cannot be empty"));
        }
        let now = now();
        self.conn()?.execute(
            "INSERT INTO project_releases (
               id, project_id, task_id, uploaded_by, version_name, channel, status,
               apk_url, file_name, file_path, sha256, size_bytes, changelog, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                id,
                write.project_id,
                clean_optional(write.task_id),
                clean_optional(write.uploaded_by),
                clean_optional(write.version_name),
                channel,
                status,
                write.apk_url.trim(),
                release_file_name(write.file_name),
                clean_optional(write.file_path),
                clean_optional(write.sha256),
                write.size_bytes,
                clean_optional(write.changelog),
                now,
            ],
        )?;
        self.project_release(&id)
    }

    pub fn project_release(&self, release_id: &str) -> Result<ProjectRelease> {
        let sql = format!("{PROJECT_RELEASE_SELECT} WHERE id = ?1");
        self.conn()?
            .query_row(&sql, params![release_id], project_release_from_row)
            .map_err(Into::into)
    }

    pub fn latest_project_release(&self, project_id: &str) -> Result<Option<ProjectRelease>> {
        let sql = format!(
            "{PROJECT_RELEASE_SELECT} WHERE project_id = ?1 AND status = 'published'
             ORDER BY updated_at DESC, created_at DESC LIMIT 1"
        );
        self.conn()?
            .query_row(&sql, params![project_id], project_release_from_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn list_project_releases(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<ProjectRelease>> {
        let limit = limit.clamp(1, 100) as i64;
        let conn = self.conn()?;
        let sql = format!(
            "{PROJECT_RELEASE_SELECT} WHERE project_id = ?1
             ORDER BY updated_at DESC, created_at DESC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let releases = stmt
            .query_map(params![project_id, limit], project_release_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(releases)
    }

    pub fn project_release_for_download(
        &self,
        project_id: &str,
        filename: &str,
    ) -> Result<Option<ProjectRelease>> {
        if filename == crate::tools::STABLE_APK_FILENAME {
            return self.latest_project_release(project_id);
        }
        let sql = format!(
            "{PROJECT_RELEASE_SELECT} WHERE project_id = ?1 AND file_name = ?2
             AND status = 'published' ORDER BY updated_at DESC, created_at DESC LIMIT 1"
        );
        self.conn()?
            .query_row(
                &sql,
                params![project_id, filename],
                project_release_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

pub(super) fn insert_task_apk_release_locked(
    conn: &Connection,
    task_id: &str,
    status: &str,
    apk_url: Option<&str>,
) -> Result<()> {
    let Some(apk_url) = clean_optional(apk_url).filter(|_| status == "done") else {
        return Ok(());
    };
    let Some((project_id, user_id, created_at)) = conn
        .query_row(
            "SELECT project_id, user_id, created_at FROM tasks WHERE id = ?1",
            params![task_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(());
    };
    let file_name = release_file_name_from_url(apk_url);
    let now = now();
    if let Some(release_id) = conn
        .query_row(
            "SELECT id FROM project_releases
             WHERE project_id = ?1 AND task_id IS NULL AND status = 'published'
               AND TRIM(apk_url) = ?2
             ORDER BY updated_at DESC, created_at DESC LIMIT 1",
            params![project_id, apk_url.trim()],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        conn.execute(
            "UPDATE project_releases
                SET task_id = ?1,
                    uploaded_by = COALESCE(uploaded_by, ?2),
                    version_name = COALESCE(version_name, ?3),
                    updated_at = ?4
              WHERE id = ?5",
            params![
                task_id,
                user_id,
                format!("AI task {}", created_at),
                now,
                release_id,
            ],
        )?;
        return Ok(());
    }
    if let Some((
        existing_file_name,
        existing_file_path,
        existing_sha256,
        existing_size,
        channel,
        changelog,
    )) = conn
        .query_row(
            "SELECT file_name, file_path, sha256, size_bytes, channel, changelog
             FROM project_releases
             WHERE project_id = ?1 AND status = 'published'
               AND TRIM(apk_url) = ?2
             ORDER BY
               CASE WHEN file_path IS NULL OR TRIM(file_path) = '' THEN 1 ELSE 0 END,
               updated_at DESC, created_at DESC
             LIMIT 1",
            params![project_id, apk_url.trim()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()?
    {
        conn.execute(
            "INSERT OR IGNORE INTO project_releases (
               id, project_id, task_id, uploaded_by, version_name, channel, status,
               apk_url, file_name, file_path, sha256, size_bytes, changelog,
               created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'published',
                     ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                new_id("rel"),
                project_id,
                task_id,
                user_id,
                format!("AI task {}", created_at),
                channel,
                apk_url.trim(),
                release_file_name(&existing_file_name),
                existing_file_path,
                existing_sha256,
                existing_size,
                changelog,
                created_at,
                now,
            ],
        )?;
        return Ok(());
    }
    conn.execute(
        "INSERT OR IGNORE INTO project_releases (
           id, project_id, task_id, uploaded_by, version_name, channel, status,
           apk_url, file_name, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, 'internal', 'published', ?6, ?7, ?8, ?9)",
        params![
            new_id("rel"),
            project_id,
            task_id,
            user_id,
            format!("AI task {}", created_at),
            apk_url.trim(),
            file_name,
            created_at,
            now,
        ],
    )?;
    Ok(())
}

const PROJECT_RELEASE_SELECT: &str = "SELECT id, project_id, task_id, uploaded_by, version_name,
    channel, status, apk_url, file_name, file_path, sha256, size_bytes, changelog, created_at,
    updated_at FROM project_releases";

fn project_release_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRelease> {
    Ok(ProjectRelease {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        uploaded_by: row.get(3)?,
        version_name: row.get(4)?,
        channel: row.get(5)?,
        status: row.get(6)?,
        apk_url: row.get(7)?,
        file_name: row.get(8)?,
        file_path: row.get(9)?,
        sha256: row.get(10)?,
        size_bytes: row.get(11)?,
        changelog: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn release_file_name_from_url(apk_url: &str) -> String {
    apk_url
        .split(['?', '#'])
        .next()
        .and_then(|path| path.rsplit('/').next())
        .map(release_file_name)
        .unwrap_or_else(|| crate::tools::STABLE_APK_FILENAME.to_string())
}

fn release_file_name(raw: &str) -> String {
    let safe = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>();
    if safe.to_ascii_lowercase().ends_with(".apk") && !safe.is_empty() {
        safe
    } else {
        crate::tools::STABLE_APK_FILENAME.to_string()
    }
}
