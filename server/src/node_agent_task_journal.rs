// server/src/node_agent_task_journal.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_CHUNK_CHARS: usize = 12_000;

#[derive(Clone, Debug)]
pub(crate) struct TaskJournal {
    dir: PathBuf,
}

#[derive(Debug)]
pub(crate) struct TaskJournalStart<'a> {
    pub req_id: &'a str,
    pub cli_name: &'a str,
    pub route: Option<&'a str>,
    pub run_handle_id: Option<&'a str>,
    pub cwd: Option<&'a str>,
    pub runtime_permission: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskJournalRecord {
    pub req_id: String,
    pub cli_name: String,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub run_handle_id: Option<String>,
    pub cwd: Option<String>,
    pub runtime_permission: Option<String>,
    #[serde(default)]
    pub os_pid: Option<u32>,
    #[serde(default)]
    pub process_started_at_ms: Option<u128>,
    pub status: String,
    pub started_at_ms: u128,
    pub updated_at_ms: u128,
    pub cancel_requested_at_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskJournalEventView {
    pub seq: usize,
    pub event: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskJournalSnapshot {
    pub task_id: String,
    pub record: Option<TaskJournalRecord>,
    pub events: Vec<TaskJournalEventView>,
    pub last_event_seq: usize,
    pub has_more: bool,
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
            route: start.route.map(str::to_string),
            run_handle_id: start.run_handle_id.map(str::to_string),
            cwd: start.cwd.map(str::to_string),
            runtime_permission: start.runtime_permission.map(str::to_string),
            os_pid: None,
            process_started_at_ms: None,
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
            "route": start.route,
            "run_handle_id": start.run_handle_id,
            "cwd": start.cwd,
            "runtime_permission": start.runtime_permission,
            "at_ms": now
        }))
    }

    pub(crate) fn record_process_started(&self, req_id: &str, pid: u32) -> Result<()> {
        let now = now_ms();
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            record.os_pid = Some(pid);
            record.process_started_at_ms = Some(now);
            record.updated_at_ms = now;
        }
        self.save_registry(&registry)?;
        self.append_event(json!({
            "type": "process_started",
            "req_id": req_id,
            "pid": pid,
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

    pub(crate) fn record_cli_chunk(&self, req_id: &str, stream: &str, text: &str) -> Result<()> {
        let text = truncate_chars(text, MAX_CHUNK_CHARS);
        if text.trim().is_empty() {
            return Ok(());
        }
        let now = now_ms();
        if let Some(event) = parse_runtime_event(req_id, stream, &text, now) {
            return self.append_event(event);
        }
        self.append_event(json!({
            "type": "cli_chunk",
            "req_id": req_id,
            "stream": normalize_stream(stream),
            "text": text,
            "at_ms": now
        }))
    }

    pub(crate) fn latest_records(&self, limit: usize) -> Result<Vec<TaskJournalRecord>> {
        let mut records: Vec<_> = self.load_registry()?.into_values().collect();
        records.sort_by(|left, right| {
            right
                .updated_at_ms
                .cmp(&left.updated_at_ms)
                .then_with(|| right.started_at_ms.cmp(&left.started_at_ms))
        });
        records.truncate(limit.min(100));
        Ok(records)
    }

    pub(crate) fn snapshot(
        &self,
        task_id: &str,
        since: usize,
        limit: usize,
    ) -> Result<TaskJournalSnapshot> {
        let registry = self.load_registry()?;
        let record = registry.get(task_id).cloned();
        let event_limit = limit.clamp(1, 200);
        let mut events = Vec::new();
        let mut last_event_seq = 0usize;
        let mut has_more = false;

        // Journal 只保存本机进程状态，不写入 prompt/API key；读取时仍按 req_id 过滤，避免
        // 前端把其他任务的本机路径混进当前任务卡片。
        for (seq, event) in self.read_events()?.into_iter() {
            last_event_seq = last_event_seq.max(seq);
            if seq <= since || !event_belongs_to_task(&event, task_id) {
                continue;
            }
            if events.len() >= event_limit {
                has_more = true;
                continue;
            }
            events.push(TaskJournalEventView { seq, event });
        }

        Ok(TaskJournalSnapshot {
            task_id: task_id.to_string(),
            record,
            events,
            last_event_seq,
            has_more,
        })
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

    fn read_events(&self) -> Result<Vec<(usize, serde_json::Value)>> {
        let path = self.events_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path).with_context(|| format!("打开 {:?}", path))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str(&line)
                .with_context(|| format!("解析 {:?} 第 {} 行", path, index + 1))?;
            events.push((index + 1, event));
        }
        Ok(events)
    }

    #[cfg(test)]
    fn read_registry_for_test(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        self.load_registry()
    }
}

fn event_belongs_to_task(event: &Value, task_id: &str) -> bool {
    event
        .get("req_id")
        .and_then(|value| value.as_str())
        .is_some_and(|req_id| req_id == task_id)
}

fn parse_runtime_event(req_id: &str, stream: &str, text: &str, at_ms: u128) -> Option<Value> {
    let parsed: Value = serde_json::from_str(text.trim()).ok()?;
    let event_type = parsed.get("type").and_then(Value::as_str)?;
    if !matches!(
        event_type,
        "tool_call" | "tool_result" | "tool_approval_required" | "tool_approval_decision"
    ) {
        return None;
    }
    Some(json!({
        "type": "tool_event",
        "req_id": req_id,
        "stream": normalize_stream(stream),
        "event": parsed,
        "text": text,
        "at_ms": at_ms
    }))
}

fn normalize_stream(stream: &str) -> &'static str {
    match stream.trim().to_ascii_lowercase().as_str() {
        "stderr" => "stderr",
        "runtime" => "runtime",
        _ => "stdout",
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push_str("\n...（本机 journal 输出已截断）");
    out
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
        let dir = unique_test_dir("lifecycle");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("req-1"),
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
        assert_eq!(record.route.as_deref(), Some("route_a_external_cli"));
        assert_eq!(record.run_handle_id.as_deref(), Some("req-1"));
        assert_eq!(record.cwd.as_deref(), Some("D:/demo"));
        assert!(record.cancel_requested_at_ms.is_some());

        let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
        assert!(events.contains(r#""type":"started""#));
        assert!(events.contains(r#""type":"cancel_requested""#));
        assert!(events.contains(r#""type":"finished""#));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn records_cli_chunks_without_prompt_or_secret_fields() {
        let dir = unique_test_dir("chunks");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("req-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("start event should persist");
        journal
            .record_cli_chunk("req-1", "stdout", "hello from cli\n")
            .expect("chunk should persist");

        let snapshot = journal
            .snapshot("req-1", 0, 20)
            .expect("snapshot should read");
        let chunk = snapshot
            .events
            .iter()
            .find(|event| {
                event.event.get("type").and_then(|value| value.as_str()) == Some("cli_chunk")
            })
            .expect("chunk event should be present");
        assert_eq!(chunk.event["text"], "hello from cli\n");
        assert!(chunk.event.get("prompt").is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn records_structured_tool_events_for_replay() {
        let dir = unique_test_dir("tool-event");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "server-runtime",
                route: Some("route_c_server_runtime"),
                run_handle_id: Some("req-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("start event should persist");
        journal
            .record_cli_chunk(
                "req-1",
                "runtime",
                r#"{"type":"tool_call","tool":"run_command","args":{"program":"git"}}"#,
            )
            .expect("tool event should persist");

        let snapshot = journal
            .snapshot("req-1", 0, 20)
            .expect("snapshot should read");
        let tool_event = snapshot
            .events
            .iter()
            .find(|event| {
                event.event.get("type").and_then(|value| value.as_str()) == Some("tool_event")
            })
            .expect("tool event should be present");
        assert_eq!(tool_event.event["event"]["type"], "tool_call");
        assert_eq!(tool_event.event["event"]["tool"], "run_command");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn snapshot_filters_events_by_task_and_cursor() {
        let dir = unique_test_dir("snapshot");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("req-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("first task should persist");
        journal
            .record_started(TaskJournalStart {
                req_id: "req-2",
                cli_name: "claude",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("req-2"),
                cwd: Some("D:/other"),
                runtime_permission: Some("read_only"),
            })
            .expect("second task should persist");
        journal
            .record_finished("req-1")
            .expect("finish event should persist");

        let snapshot = journal
            .snapshot("req-1", 1, 20)
            .expect("snapshot should read");
        assert_eq!(snapshot.task_id, "req-1");
        assert_eq!(snapshot.record.as_ref().unwrap().status, "finished");
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(
            snapshot.events[0]
                .event
                .get("type")
                .and_then(|value| value.as_str()),
            Some("finished")
        );
        assert!(snapshot.last_event_seq >= 3);
        assert!(!snapshot.has_more);

        let latest = journal
            .latest_records(10)
            .expect("latest records should read");
        assert_eq!(latest.len(), 2);
        assert_eq!(latest[0].req_id, "req-1");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn records_process_pid_for_active_route_a_handle() {
        let dir = unique_test_dir("pid");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("req-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("start event should persist");
        journal
            .record_process_started("req-1", 4242)
            .expect("pid event should persist");

        let snapshot = journal
            .snapshot("req-1", 0, 20)
            .expect("snapshot should read");
        let record = snapshot.record.expect("record should exist");
        assert_eq!(record.os_pid, Some(4242));
        assert!(record.process_started_at_ms.is_some());
        assert!(snapshot.events.iter().any(|event| {
            event.event.get("type").and_then(|value| value.as_str()) == Some("process_started")
                && event.event.get("pid").and_then(|value| value.as_u64()) == Some(4242)
        }));
        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-task-journal-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
