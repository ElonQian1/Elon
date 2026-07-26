//! Persistent derived index and durable change queue for project documents.

use anyhow::{Context, Result};
use homecli_proto::ProjectDocumentEntry;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct DocumentMaintenanceState {
    pub index_version: u8,
    pub durable_queue: bool,
    pub poll_interval_seconds: u64,
    pub changed_documents: usize,
    pub pending_events: usize,
    pub processed_events: usize,
    pub last_indexed_at_ms: u64,
}

pub(crate) struct ProjectDocumentIndex {
    pub(crate) conn: Connection,
    run_id: String,
    changed_documents: usize,
    classifier_version_current: bool,
}

impl ProjectDocumentIndex {
    pub(crate) fn open(workspace: &Path) -> Result<Self> {
        let database = index_path(workspace)?;
        if let Some(parent) = database.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建文档索引目录失败：{}", parent.display()))?;
        }
        let conn = Connection::open(&database)
            .with_context(|| format!("打开文档索引失败：{}", database.display()))?;
        initialize_schema(&conn)?;
        crate::project_document_issue_workflow::initialize_schema(&conn)?;
        let classifier_version_current = metadata_value(&conn, "classifier_version")?.as_deref()
            == Some(crate::project_document_policy::CLASSIFIER_VERSION);
        Ok(Self {
            conn,
            run_id: format!("{}-{}", now_millis(), uuid::Uuid::new_v4().simple()),
            changed_documents: 0,
            classifier_version_current,
        })
    }

    pub(crate) fn cached_document(
        &self,
        path: &str,
        size_bytes: u64,
        modified_at_ms: u64,
    ) -> Result<Option<ProjectDocumentEntry>> {
        if !self.classifier_version_current {
            return Ok(None);
        }
        let cached = self
            .conn
            .query_row(
                "SELECT entry_json FROM documents WHERE path=?1 AND size_bytes=?2 AND modified_at_ms=?3",
                params![path, to_i64(size_bytes), to_i64(modified_at_ms)],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        cached
            .map(|json| serde_json::from_str(&json).context("解析缓存文档目录失败"))
            .transpose()
    }

    pub(crate) fn observe_document(
        &mut self,
        entry: &ProjectDocumentEntry,
        modified_at_ms: u64,
    ) -> Result<()> {
        let path = normalize_path(&entry.path);
        let old_hash = self
            .conn
            .query_row(
                "SELECT content_hash FROM documents WHERE path=?1",
                params![path],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let new_hash = entry.metadata.content_hash.as_str();
        if old_hash.as_deref() != Some(new_hash) {
            self.insert_event(
                &path,
                if old_hash.is_some() {
                    "modified"
                } else {
                    "created"
                },
                old_hash.as_deref(),
                Some(new_hash),
            )?;
            self.changed_documents += 1;
        }
        self.conn.execute(
            "INSERT INTO documents(path,size_bytes,modified_at_ms,content_hash,entry_json,last_seen_run)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(path) DO UPDATE SET size_bytes=excluded.size_bytes,
             modified_at_ms=excluded.modified_at_ms,content_hash=excluded.content_hash,
             entry_json=excluded.entry_json,last_seen_run=excluded.last_seen_run",
            params![
                path,
                to_i64(entry.byte_len),
                to_i64(modified_at_ms),
                new_hash,
                serde_json::to_string(entry)?,
                self.run_id
            ],
        )?;
        Ok(())
    }

    pub(crate) fn finish_scan(&mut self, seen_paths: &HashSet<String>) -> Result<()> {
        let mut statement = self
            .conn
            .prepare("SELECT path,content_hash FROM documents WHERE last_seen_run<>?1")?;
        let stale = statement
            .query_map(params![self.run_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        for (path, hash) in stale {
            if seen_paths.contains(&path) {
                continue;
            }
            self.insert_event(&path, "deleted", Some(&hash), None)?;
            self.conn
                .execute("DELETE FROM documents WHERE path=?1", params![path])?;
            self.conn
                .execute("DELETE FROM quality_cache WHERE path=?1", params![path])?;
            self.changed_documents += 1;
        }
        self.conn.execute(
            "INSERT INTO metadata(key,value) VALUES('classifier_version',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![crate::project_document_policy::CLASSIFIER_VERSION],
        )?;
        self.classifier_version_current = true;
        Ok(())
    }

    pub(crate) fn cached_quality_facts(&self, path: &str, hash: &str) -> Result<Option<Value>> {
        let value = self
            .conn
            .query_row(
                "SELECT facts_json FROM quality_cache WHERE path=?1 AND content_hash=?2",
                params![normalize_path(path), hash],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        value
            .map(|json| serde_json::from_str(&json).context("解析文档质量缓存失败"))
            .transpose()
    }

    pub(crate) fn store_quality_facts(&self, path: &str, hash: &str, facts: &Value) -> Result<()> {
        self.conn.execute(
            "INSERT INTO quality_cache(path,content_hash,facts_json) VALUES(?1,?2,?3)
             ON CONFLICT(path) DO UPDATE SET content_hash=excluded.content_hash,facts_json=excluded.facts_json",
            params![normalize_path(path), hash, serde_json::to_string(facts)?],
        )?;
        Ok(())
    }

    pub(crate) fn replace_issues(&self, issues: &[Value]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM issues", [])?;
        for issue in issues {
            let fingerprint = issue
                .get("fingerprint")
                .and_then(Value::as_str)
                .unwrap_or_default();
            tx.execute(
                "INSERT INTO issues(fingerprint,issue_json,updated_at_ms) VALUES(?1,?2,?3)",
                params![
                    fingerprint,
                    serde_json::to_string(issue)?,
                    to_i64(now_millis())
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn list_issues(
        &self,
        issue_types: &[String],
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Value>> {
        let mut statement = self.conn.prepare(
            "SELECT issue_json FROM issues ORDER BY
             CASE json_extract(issue_json,'$.severity') WHEN 'error' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
             json_extract(issue_json,'$.path'), fingerprint LIMIT 100000",
        )?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|json| serde_json::from_str::<Value>(&json).ok())
            .filter(|issue| {
                issue_types.is_empty()
                    || issue
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| issue_types.iter().any(|value| value == kind))
            })
            .skip(offset)
            .take(limit)
            .collect();
        Ok(values)
    }

    pub(crate) fn external_link_status(
        &self,
        url: &str,
    ) -> Result<Option<(Option<u16>, Option<String>)>> {
        self.conn
            .query_row(
                "SELECT http_status,error FROM external_links WHERE url=?1",
                params![url],
                |row| {
                    let status = row.get::<_, Option<i64>>(0)?.map(|value| value as u16);
                    Ok((status, row.get::<_, Option<String>>(1)?))
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn checked_external_links(&self) -> Result<usize> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM external_links WHERE checked_at_ms>0",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    pub(crate) fn external_links_due(&self, limit: usize) -> Result<Vec<String>> {
        let cutoff = now_millis().saturating_sub(24 * 60 * 60 * 1_000);
        let mut statement = self.conn.prepare(
            "SELECT DISTINCT json_each.value
             FROM quality_cache,json_each(quality_cache.facts_json,'$.external_links')
             LEFT JOIN external_links ON external_links.url=json_each.value
             WHERE external_links.checked_at_ms IS NULL OR external_links.checked_at_ms<?1
             LIMIT ?2",
        )?;
        let urls = statement
            .query_map(params![to_i64(cutoff), to_i64(limit as u64)], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(urls)
    }

    pub(crate) fn store_external_link_result(
        &self,
        url: &str,
        status: Option<u16>,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO external_links(url,http_status,error,checked_at_ms) VALUES(?1,?2,?3,?4)
             ON CONFLICT(url) DO UPDATE SET http_status=excluded.http_status,
             error=excluded.error,checked_at_ms=excluded.checked_at_ms",
            params![url, status.map(i64::from), error, to_i64(now_millis())],
        )?;
        Ok(())
    }

    pub(crate) fn complete_analysis(&self) -> Result<DocumentMaintenanceState> {
        let pending_before = self.pending_events()?;
        let retention_cutoff = now_millis().saturating_sub(30 * 24 * 60 * 60 * 1_000);
        self.conn.execute(
            "DELETE FROM events WHERE processed_at_ms IS NOT NULL AND created_at_ms<?1",
            params![to_i64(retention_cutoff)],
        )?;
        self.conn.execute(
            "UPDATE events SET processed_at_ms=?1 WHERE processed_at_ms IS NULL",
            params![to_i64(now_millis())],
        )?;
        self.conn.execute(
            "INSERT INTO metadata(key,value) VALUES('last_indexed_at_ms',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![now_millis().to_string()],
        )?;
        Ok(DocumentMaintenanceState {
            index_version: 2,
            durable_queue: true,
            poll_interval_seconds: 60,
            changed_documents: self.changed_documents,
            pending_events: 0,
            processed_events: pending_before,
            last_indexed_at_ms: now_millis(),
        })
    }

    pub(crate) fn state(&self) -> Result<DocumentMaintenanceState> {
        let last = self
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key='last_indexed_at_ms'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .and_then(|value| value.parse().ok())
            .unwrap_or_default();
        Ok(DocumentMaintenanceState {
            index_version: 2,
            durable_queue: true,
            poll_interval_seconds: 60,
            changed_documents: self.changed_documents,
            pending_events: self.pending_events()?,
            processed_events: 0,
            last_indexed_at_ms: last,
        })
    }

    fn pending_events(&self) -> Result<usize> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM events WHERE processed_at_ms IS NULL",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    fn insert_event(
        &self,
        path: &str,
        change_kind: &str,
        before_hash: Option<&str>,
        after_hash: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events(id,path,change_kind,before_hash,after_hash,source,created_at_ms)
             VALUES(?1,?2,?3,?4,?5,'workspace_poll',?6)",
            params![
                uuid::Uuid::new_v4().simple().to_string(),
                path,
                change_kind,
                before_hash,
                after_hash,
                to_i64(now_millis())
            ],
        )?;
        Ok(())
    }
}

pub(crate) fn file_modified_millis(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS documents(
           path TEXT PRIMARY KEY,size_bytes INTEGER NOT NULL,modified_at_ms INTEGER NOT NULL,
           content_hash TEXT NOT NULL,entry_json TEXT NOT NULL,last_seen_run TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS events(
           id TEXT PRIMARY KEY,path TEXT NOT NULL,change_kind TEXT NOT NULL,before_hash TEXT,
           after_hash TEXT,source TEXT NOT NULL,created_at_ms INTEGER NOT NULL,processed_at_ms INTEGER);
         CREATE INDEX IF NOT EXISTS events_pending ON events(processed_at_ms,created_at_ms);
         CREATE TABLE IF NOT EXISTS quality_cache(
           path TEXT PRIMARY KEY,content_hash TEXT NOT NULL,facts_json TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS issues(
           fingerprint TEXT PRIMARY KEY,issue_json TEXT NOT NULL,updated_at_ms INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS external_links(
           url TEXT PRIMARY KEY,http_status INTEGER,error TEXT,checked_at_ms INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);",
    )?;
    Ok(())
}

fn metadata_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key=?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn index_path(workspace: &Path) -> Result<PathBuf> {
    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().replace('\\', "/").as_bytes());
    let key = format!("{:x}", hasher.finalize());
    Ok(state_root().join("indexes").join(format!("{key}.sqlite3")))
}

pub(crate) fn state_root() -> PathBuf {
    if let Some(path) = std::env::var_os("ELON_PROJECT_DOCS_STATE_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("Elon").join("project-docs");
    }
    if let Some(path) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(path).join("elon").join("project-docs");
    }
    std::env::temp_dir().join("elon-project-docs-state")
}

fn normalize_path(path: &str) -> String {
    path.trim().replace('\\', "/")
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use homecli_proto::{ProjectDocumentEntry, ProjectDocumentMetadata};

    #[test]
    fn persistent_index_emits_only_real_changes() {
        let root = std::env::temp_dir().join(format!(
            "elon_document_index_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let entry = ProjectDocumentEntry {
            path: "README.md".to_string(),
            title: "Home".to_string(),
            content: String::new(),
            truncated: false,
            byte_len: 10,
            source: "workspace".to_string(),
            metadata: ProjectDocumentMetadata {
                content_hash: "v1".to_string(),
                ..ProjectDocumentMetadata::default()
            },
        };
        let mut first = ProjectDocumentIndex::open(&root).unwrap();
        first.observe_document(&entry, 1).unwrap();
        first
            .finish_scan(&HashSet::from(["README.md".to_string()]))
            .unwrap();
        assert_eq!(first.state().unwrap().changed_documents, 1);
        drop(first);

        let mut second = ProjectDocumentIndex::open(&root).unwrap();
        assert!(second
            .cached_document("README.md", entry.byte_len, 1)
            .unwrap()
            .is_some());
        second.observe_document(&entry, 1).unwrap();
        assert_eq!(second.state().unwrap().changed_documents, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn classifier_version_invalidates_unchanged_catalog_metadata() {
        let root = std::env::temp_dir().join(format!(
            "elon_document_classifier_version_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let entry = ProjectDocumentEntry {
            path: "default-project-docs/README.md".to_string(),
            title: "Template".to_string(),
            content: String::new(),
            truncated: false,
            byte_len: 10,
            source: "workspace".to_string(),
            metadata: ProjectDocumentMetadata {
                role: "project_template".to_string(),
                content_hash: "same-content".to_string(),
                ..ProjectDocumentMetadata::default()
            },
        };
        let mut first = ProjectDocumentIndex::open(&root).unwrap();
        first.observe_document(&entry, 7).unwrap();
        first
            .finish_scan(&HashSet::from([entry.path.clone()]))
            .unwrap();
        first
            .conn
            .execute(
                "UPDATE metadata SET value='legacy' WHERE key='classifier_version'",
                [],
            )
            .unwrap();
        drop(first);

        let stale = ProjectDocumentIndex::open(&root).unwrap();
        assert!(stale
            .cached_document(&entry.path, entry.byte_len, 7)
            .unwrap()
            .is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
