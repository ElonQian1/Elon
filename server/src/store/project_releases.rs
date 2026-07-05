use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Map, Value};

use super::{clean_optional, new_id, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRelease {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub uploaded_by: Option<String>,
    pub release_number: Option<i64>,
    pub version_name: Option<String>,
    pub package_name: Option<String>,
    pub version_code: Option<i64>,
    pub channel: String,
    pub status: String,
    pub apk_url: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub changelog: Option<String>,
    pub build_started_at: Option<String>,
    pub source_git_sha: Option<String>,
    pub source_worktree: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectReleaseWrite<'a> {
    pub id: Option<&'a str>,
    pub project_id: &'a str,
    pub task_id: Option<&'a str>,
    pub uploaded_by: Option<&'a str>,
    pub version_name: Option<&'a str>,
    pub package_name: Option<&'a str>,
    pub version_code: Option<i64>,
    pub channel: Option<&'a str>,
    pub status: Option<&'a str>,
    pub apk_url: &'a str,
    pub file_name: &'a str,
    pub file_path: Option<&'a str>,
    pub sha256: Option<&'a str>,
    pub size_bytes: Option<i64>,
    pub changelog: Option<&'a str>,
    pub build_started_at: Option<&'a str>,
    pub source_git_sha: Option<&'a str>,
    pub source_worktree: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
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
        {
            let conn = self.conn()?;
            let release_number: i64 = conn.query_row(
                "SELECT COALESCE(MAX(release_number), 0) + 1
                 FROM project_releases
                 WHERE project_id = ?1",
                params![write.project_id],
                |row| row.get(0),
            )?;
            conn.execute(
                "INSERT INTO project_releases (
                   id, project_id, task_id, uploaded_by, release_number,
                   version_name, package_name, version_code, channel, status,
                   apk_url, file_name, file_path, sha256, size_bytes, changelog,
                   build_started_at, source_git_sha, source_worktree, metadata_json,
                   created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?21)",
                params![
                    id,
                    write.project_id,
                    clean_optional(write.task_id),
                    clean_optional(write.uploaded_by),
                    release_number,
                    clean_optional(write.version_name),
                    clean_optional(write.package_name),
                    write.version_code,
                    channel,
                    status,
                    write.apk_url.trim(),
                    release_file_name(write.file_name),
                    clean_optional(write.file_path),
                    clean_optional(write.sha256),
                    write.size_bytes,
                    clean_optional(write.changelog),
                    clean_optional(write.build_started_at),
                    clean_optional(write.source_git_sha),
                    clean_optional(write.source_worktree),
                    clean_optional(write.metadata_json),
                    now,
                ],
            )?;
        }
        let release = self.project_release(&id)?;
        if let Err(error) = self.sync_project_landing_download_from_release(&release) {
            tracing::warn!(
                project_id = %release.project_id,
                release_id = %release.id,
                error = %error,
                "failed to sync project release into landing snapshot"
            );
        }
        Ok(release)
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
               AND file_path IS NOT NULL AND TRIM(file_path) != ''
             ORDER BY COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC LIMIT 1"
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
             ORDER BY COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC LIMIT ?2"
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
             AND status = 'published'
             AND file_path IS NOT NULL AND TRIM(file_path) != ''
             ORDER BY COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC LIMIT 1"
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
    let now = now();
    if let Some(release_id) = conn
        .query_row(
            "SELECT id FROM project_releases
             WHERE project_id = ?1 AND task_id IS NULL AND status = 'published'
               AND TRIM(apk_url) = ?2
               AND file_path IS NOT NULL AND TRIM(file_path) != ''
             ORDER BY COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC LIMIT 1",
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
        existing_version_name,
        package_name,
        version_code,
        build_started_at,
        source_git_sha,
        source_worktree,
        metadata_json,
    )) = conn
        .query_row(
            "SELECT file_name, file_path, sha256, size_bytes, channel, changelog,
                    version_name, package_name, version_code, build_started_at,
                    source_git_sha, source_worktree, metadata_json
             FROM project_releases
             WHERE project_id = ?1 AND status = 'published'
               AND TRIM(apk_url) = ?2
               AND file_path IS NOT NULL AND TRIM(file_path) != ''
             ORDER BY
               CASE WHEN file_path IS NULL OR TRIM(file_path) = '' THEN 1 ELSE 0 END,
               COALESCE(release_number, 0) DESC, updated_at DESC, created_at DESC
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
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                ))
            },
        )
        .optional()?
    {
        let release_number: i64 = conn.query_row(
            "SELECT COALESCE(MAX(release_number), 0) + 1
             FROM project_releases
             WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO project_releases (
               id, project_id, task_id, uploaded_by, release_number,
               version_name, package_name, version_code, channel, status,
               apk_url, file_name, file_path, sha256, size_bytes, changelog,
               build_started_at, source_git_sha, source_worktree, metadata_json,
               created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'published',
                     ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                new_id("rel"),
                project_id,
                task_id,
                user_id,
                release_number,
                existing_version_name.unwrap_or_else(|| format!("AI task {}", created_at)),
                package_name,
                version_code,
                channel,
                apk_url.trim(),
                release_file_name(&existing_file_name),
                existing_file_path,
                existing_sha256,
                existing_size,
                changelog,
                build_started_at,
                source_git_sha,
                source_worktree,
                metadata_json,
                created_at,
                now,
            ],
        )?;
        return Ok(());
    }
    // A stable download URL alone is not a published APK. Only create task release
    // records after the APK file has been synced into server-managed artifacts.
    Ok(())
}

impl Store {
    fn sync_project_landing_download_from_release(&self, release: &ProjectRelease) -> Result<()> {
        if release.status != "published" || clean_optional(release.file_path.as_deref()).is_none() {
            return Ok(());
        }

        let conn = self.conn()?;
        let Some((name, display_name, description, landing_json)) = conn
            .query_row(
                "SELECT name, display_name, description, landing_json
                 FROM projects
                 WHERE id = ?1 AND status != 'deleted'",
                params![release.project_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(());
        };

        let mut landing = landing_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        insert_if_absent(
            &mut landing,
            "title",
            clean_optional(display_name.as_deref()).unwrap_or(name.trim()),
        );
        if let Some(description) = clean_optional(description.as_deref()) {
            insert_if_absent(&mut landing, "summary", description);
        }

        let previous_downloads = landing_download_items(landing.remove("downloads"));
        let mut downloads = vec![project_release_android_download(release)];
        for download in previous_downloads {
            if !landing_download_is_android(&download) {
                downloads.push(download);
            }
            if downloads.len() >= 12 {
                break;
            }
        }
        landing.insert("downloads".to_string(), Value::Array(downloads));

        let release_update = project_release_update_text(release);
        let mut recent_updates = landing_text_items(landing.remove("recent_updates"));
        if let Some(update) = release_update {
            recent_updates.retain(|item| item != &update);
            recent_updates.insert(0, update);
        }
        if !recent_updates.is_empty() {
            landing.insert(
                "recent_updates".to_string(),
                Value::Array(
                    recent_updates
                        .into_iter()
                        .take(12)
                        .map(Value::String)
                        .collect(),
                ),
            );
        }

        let Some(snapshot) =
            crate::project_landing::normalize_landing_snapshot(&Value::Object(landing))
        else {
            return Ok(());
        };
        let landing_json = serde_json::to_string(&snapshot)?;
        let now = now();
        conn.execute(
            "UPDATE projects
             SET landing_json = ?1,
                 updated_at = ?2
             WHERE id = ?3 AND status != 'deleted'",
            params![landing_json, now, release.project_id.as_str()],
        )?;
        conn.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_release_landing_synced', ?4, ?5)",
            params![
                new_id("evt"),
                release.project_id.as_str(),
                release.uploaded_by.as_deref(),
                json!({
                    "release_id": release.id.as_str(),
                    "release_number": release.release_number,
                    "apk_url": release.apk_url.as_str(),
                    "package_name": release.package_name.as_deref(),
                    "version_code": release.version_code,
                })
                .to_string(),
                now,
            ],
        )?;
        Ok(())
    }
}

const PROJECT_RELEASE_SELECT: &str = "SELECT id, project_id, task_id, uploaded_by,
    release_number, version_name, package_name, version_code, channel, status, apk_url,
    file_name, file_path, sha256, size_bytes, changelog, build_started_at, source_git_sha,
    source_worktree, metadata_json, created_at, updated_at FROM project_releases";

fn project_release_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRelease> {
    Ok(ProjectRelease {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        uploaded_by: row.get(3)?,
        release_number: row.get(4)?,
        version_name: row.get(5)?,
        package_name: row.get(6)?,
        version_code: row.get(7)?,
        channel: row.get(8)?,
        status: row.get(9)?,
        apk_url: row.get(10)?,
        file_name: row.get(11)?,
        file_path: row.get(12)?,
        sha256: row.get(13)?,
        size_bytes: row.get(14)?,
        changelog: row.get(15)?,
        build_started_at: row.get(16)?,
        source_git_sha: row.get(17)?,
        source_worktree: row.get(18)?,
        metadata_json: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

fn insert_if_absent(landing: &mut Map<String, Value>, key: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    let already_set = landing
        .get(key)
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !already_set {
        landing.insert(key.to_string(), Value::String(value.trim().to_string()));
    }
}

fn landing_download_items(value: Option<Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items,
        Some(Value::Object(items)) => items
            .into_iter()
            .filter_map(|(platform, item)| match item {
                Value::Object(mut object) => {
                    object
                        .entry("platform".to_string())
                        .or_insert_with(|| Value::String(platform));
                    Some(Value::Object(object))
                }
                Value::String(url) => Some(json!({
                    "platform": platform,
                    "url": url,
                })),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn landing_download_is_android(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    ["platform", "os", "type", "kind"]
        .iter()
        .filter_map(|key| object.get(*key).and_then(Value::as_str))
        .any(|value| {
            let compact = value
                .trim()
                .to_ascii_lowercase()
                .chars()
                .filter(|ch| !matches!(ch, '_' | '-' | ' '))
                .collect::<String>();
            matches!(compact.as_str(), "android" | "apk" | "androidapk")
        })
}

fn landing_text_items(value: Option<Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| match item {
                Value::String(text) => clean_text(text),
                Value::Object(object) => object
                    .get("title")
                    .or_else(|| object.get("text"))
                    .and_then(Value::as_str)
                    .and_then(|text| clean_text(text.to_string())),
                _ => None,
            })
            .collect(),
        Some(Value::String(text)) => clean_text(text).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn project_release_android_download(release: &ProjectRelease) -> Value {
    let mut download = Map::new();
    download.insert("platform".to_string(), Value::String("android".to_string()));
    download.insert(
        "label".to_string(),
        Value::String("Android APK".to_string()),
    );
    download.insert("short".to_string(), Value::String("APK".to_string()));
    download.insert("kind".to_string(), Value::String("project_apk".to_string()));
    download.insert("url".to_string(), Value::String(release.apk_url.clone()));
    download.insert("status".to_string(), Value::String("available".to_string()));
    if let Some(version) = project_release_display_version(release) {
        download.insert("version".to_string(), Value::String(version));
    }
    if let Some(size_bytes) = release.size_bytes.filter(|size| *size > 0) {
        download.insert(
            "size_bytes".to_string(),
            Value::String(size_bytes.to_string()),
        );
        if let Some(size_label) = format_size_label(size_bytes) {
            download.insert("size_label".to_string(), Value::String(size_label));
        }
    }
    if let Some(changelog) = clean_optional(release.changelog.as_deref()) {
        download.insert("note".to_string(), Value::String(changelog.to_string()));
    }
    Value::Object(download)
}

fn project_release_display_version(release: &ProjectRelease) -> Option<String> {
    let version_name = clean_optional(release.version_name.as_deref());
    match (version_name, release.release_number) {
        (Some(version_name), Some(release_number)) => {
            Some(format!("{version_name} (build {release_number})"))
        }
        (Some(version_name), None) => Some(version_name.to_string()),
        (None, Some(release_number)) => Some(format!("build {release_number}")),
        (None, None) => None,
    }
}

fn project_release_update_text(release: &ProjectRelease) -> Option<String> {
    if let Some(changelog) = clean_optional(release.changelog.as_deref()) {
        return Some(changelog.to_string());
    }
    project_release_display_version(release)
        .map(|version| format!("Android APK {version} published"))
}

fn format_size_label(size_bytes: i64) -> Option<String> {
    if size_bytes <= 0 {
        return None;
    }
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let size = size_bytes as f64;
    if size >= MB {
        Some(format!("{:.1} MB", size / MB))
    } else if size >= KB {
        Some(format!("{:.1} KB", size / KB))
    } else {
        Some(format!("{size_bytes} B"))
    }
}

fn clean_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(160).collect::<String>())
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
