// server/src/node_agent_task_journal.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub(crate) struct TaskJournal {
    dir: PathBuf,
}

#[derive(Debug)]
pub(crate) struct TaskJournalStart<'a> {
    pub req_id: &'a str,
    pub cli_name: &'a str,
    pub cwd: Option<&'a str>,
    pub runtime_permission: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskJournalRecord {
    pub req_id: String,
    pub cli_name: String,
    pub cwd: Option<String>,
    pub runtime_permission: Option<String>,
    pub status: String,
    pub started_at_ms: u128,
    pub updated_at_ms: u128,
    pub cancel_requested_at_ms: Option<u128>,
}

impl TaskJournal {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub(crate) fn default() -> Self {
        Self::new(super::state_path().with_file_name("task-journal"))
    }

    pub(crate) fn record_started(&self, start: TaskJournalStart<'_>) -> Result<()> {
        let now = now_ms();
        let mut registry = self.load_registry()?;
        let record = TaskJournalRecord {
            req_id: start.req_id.to_string(),
            cli_name: start.cli_name.to_string(),
            cwd: start.cwd.map(str::to_string),
            runtime_permission: start.runtime_permission.map(str::to_string),
            status: "running".to_string(),
            started_at_ms: now,
            updated_at_ms: now,
            cancel_requested_at_ms: None,
        };
        registry.insert(start.req_id.to_string(), record);
        self.save_registry(&registry)?;
        self.append_event(json!({
            "type": "started",
            "req_id": start.req_id,
            "cli": start.cli_name,
            "cwd": start.cwd,
            "runtime_permission": start.runtime_permission,
            "at_ms": now
        }))
    }

    pub(crate) fn record_cancel_requested(&self, req_id: &str) -> Result<()> {
        let now = now_ms();
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            record.status = "cancel_requested".to_string();
            record.updated_at_ms = now;
            record.cancel_requested_at_ms = Some(now);
        }
        self.save_registry(&registry)?;
        self.append_event(json!({
            "type": "cancel_requested",
            "req_id": req_id,
            "at_ms": now
        }))
    }

    pub(crate) fn record_finished(&self, req_id: &str) -> Result<()> {
        let now = now_ms();
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            record.status = "finished".to_string();
            record.updated_at_ms = now;
        }
        self.save_registry(&registry)?;
        self.append_event(json!({
            "type": "finished",
            "req_id": req_id,
            "at_ms": now
        }))
    }

    fn load_registry(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        let path = self.registry_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let text = fs::read_to_string(&path).with_context(|| format!("读取 {:?}", path))?;
        serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))
    }

    fn save_registry(&self, registry: &BTreeMap<String, TaskJournalRecord>) -> Result<()> {
        self.ensure_dir()?;
        let path = self.registry_path();
        let text = serde_json::to_string_pretty(registry)?;
        fs::write(&path, text).with_context(|| format!("写入 {:?}", path))
    }

    fn append_event(&self, event: serde_json::Value) -> Result<()> {
        self.ensure_dir()?;
        let path = self.events_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("打开 {:?}", path))?;
        writeln!(file, "{}", serde_json::to_string(&event)?)
            .with_context(|| format!("写入 {:?}", path))
    }

    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir).with_context(|| format!("创建 {:?}", self.dir))
    }

    fn registry_path(&self) -> PathBuf {
        self.dir.join("registry.json")
    }

    fn events_path(&self) -> PathBuf {
        self.dir.join("events.jsonl")
    }

    #[cfg(test)]
    fn read_registry_for_test(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        self.load_registry()
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{TaskJournal, TaskJournalStart};
    use std::{fs, path::PathBuf};

    #[test]
    fn records_started_cancel_and_finished_events() {
        let dir = unique_test_dir();
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "codex",
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("started event should persist");
        journal
            .record_cancel_requested("req-1")
            .expect("cancel event should persist");
        journal
            .record_finished("req-1")
            .expect("finished event should persist");

        let registry = journal
            .read_registry_for_test()
            .expect("registry should read");
        let record = registry.get("req-1").expect("record should exist");
        assert_eq!(record.status, "finished");
        assert_eq!(record.cli_name, "codex");
        assert_eq!(record.cwd.as_deref(), Some("D:/demo"));
        assert!(record.cancel_requested_at_ms.is_some());

        let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
        assert!(events.contains(r#""type":"started""#));
        assert!(events.contains(r#""type":"cancel_requested""#));
        assert!(events.contains(r#""type":"finished""#));
        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("elon-task-journal-test-{}", std::process::id()))
    }
}
