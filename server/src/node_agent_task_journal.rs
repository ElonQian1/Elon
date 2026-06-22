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
const MAX_ERROR_CHARS: usize = 2_000;

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
    #[serde(default)]
    pub codex_session_id: Option<String>,
    #[serde(default)]
    pub codex_session_scope_key: Option<String>,
    #[serde(default)]
    pub codex_session_updated_at_ms: Option<u128>,
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
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
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

    pub(crate) fn load_codex_session(&self, scope_key: &str) -> Result<Option<String>> {
        let path = self.codex_sessions_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).with_context(|| format!("读取 {:?}", path))?;
        let map: BTreeMap<String, String> =
            serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))?;
        Ok(map
            .get(scope_key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()))
    }

    pub(crate) fn record_codex_session(
        &self,
        req_id: &str,
        scope_key: &str,
        session_id: &str,
    ) -> Result<()> {
        let scope_key = scope_key.trim();
        let session_id = session_id.trim();
        if scope_key.is_empty() || session_id.is_empty() {
            return Ok(());
        }
        let now = now_ms();
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            record.codex_session_id = Some(session_id.to_string());
            record.codex_session_scope_key = Some(scope_key.to_string());
            record.codex_session_updated_at_ms = Some(now);
            record.updated_at_ms = now;
        }
        self.save_registry(&registry)?;
        self.save_codex_session(scope_key, session_id)?;
        self.append_event(json!({
            "type": "codex_session",
            "req_id": req_id,
            "scope_key": scope_key,
            "session_id": session_id,
            "at_ms": now
        }))
    }

    pub(crate) fn clear_codex_session(&self, req_id: &str, scope_key: &str) -> Result<()> {
        let scope_key = scope_key.trim();
        if scope_key.is_empty() {
            return Ok(());
        }
        let now = now_ms();
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            if record.codex_session_scope_key.as_deref() == Some(scope_key) {
                record.codex_session_id = None;
                record.codex_session_scope_key = None;
                record.codex_session_updated_at_ms = None;
                record.updated_at_ms = now;
            }
        }
        self.save_registry(&registry)?;
        self.remove_codex_session(scope_key)?;
        self.append_event(json!({
            "type": "codex_session_cleared",
            "req_id": req_id,
            "scope_key": scope_key,
            "reason": "stale_resume",
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
        self.record_finished_with_outcome(req_id, "finished", None)
    }

    pub(crate) fn record_finished_with_outcome(
        &self,
        req_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now_ms();
        let requested_status = normalize_finish_status(status, error);
        let mut effective_status = requested_status.to_string();
        let mut already_terminal = false;
        let mut registry = self.load_registry()?;
        if let Some(record) = registry.get_mut(req_id) {
            if requested_status == "finished" && is_terminal_status(&record.status) {
                effective_status = record.status.clone();
                already_terminal = true;
            } else {
                record.status = requested_status.to_string();
                effective_status = record.status.clone();
            }
            record.updated_at_ms = now;
        }
        self.save_registry(&registry)?;
        if already_terminal {
            return Ok(());
        }

        let mut event = json!({
            "type": "finished",
            "req_id": req_id,
            "status": effective_status,
            "at_ms": now
        });
        if let Some(error) = normalize_finish_error(error) {
            event["error"] = Value::String(error);
        }
        self.append_event(event)
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

        // Journal 只保存本机进程状态，不写入 prompt/API key；读取时仍按 req_id 过滤，避免
        // 前端把其他任务的本机路径混进当前任务卡片。压力场景下按行流式扫描，避免把整个
        // events.jsonl 收集到内存后再过滤。
        let event_scan = self.scan_task_events(task_id, since, event_limit)?;

        Ok(TaskJournalSnapshot {
            task_id: task_id.to_string(),
            record,
            events: event_scan.events,
            last_event_seq: event_scan.last_event_seq,
            has_more: event_scan.has_more,
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

    fn codex_sessions_path(&self) -> PathBuf {
        self.dir.join("codex-sessions.json")
    }

    fn save_codex_session(&self, scope_key: &str, session_id: &str) -> Result<()> {
        self.ensure_dir()?;
        let path = self.codex_sessions_path();
        let mut map: BTreeMap<String, String> = if path.exists() {
            let text = fs::read_to_string(&path).with_context(|| format!("读取 {:?}", path))?;
            serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))?
        } else {
            BTreeMap::new()
        };
        map.insert(scope_key.to_string(), session_id.to_string());
        fs::write(&path, serde_json::to_string_pretty(&map)?)
            .with_context(|| format!("写入 {:?}", path))
    }

    fn remove_codex_session(&self, scope_key: &str) -> Result<()> {
        self.ensure_dir()?;
        let path = self.codex_sessions_path();
        if !path.exists() {
            return Ok(());
        }
        let text = fs::read_to_string(&path).with_context(|| format!("读取 {:?}", path))?;
        let mut map: BTreeMap<String, String> =
            serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))?;
        map.remove(scope_key);
        fs::write(&path, serde_json::to_string_pretty(&map)?)
            .with_context(|| format!("写入 {:?}", path))
    }

    fn scan_task_events(
        &self,
        task_id: &str,
        since: usize,
        event_limit: usize,
    ) -> Result<TaskJournalEventScan> {
        let path = self.events_path();
        if !path.exists() {
            return Ok(TaskJournalEventScan::default());
        }
        let file = File::open(&path).with_context(|| format!("打开 {:?}", path))?;
        let reader = BufReader::new(file);
        let mut scan = TaskJournalEventScan::default();
        for (index, line) in reader.lines().enumerate() {
            let seq = index + 1;
            scan.last_event_seq = scan.last_event_seq.max(seq);
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            if line.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str(&line)
                .with_context(|| format!("解析 {:?} 第 {} 行", path, index + 1))?;
            if seq <= since || !event_belongs_to_task(&event, task_id) {
                continue;
            }
            if scan.events.len() >= event_limit {
                scan.has_more = true;
                continue;
            }
            scan.events.push(TaskJournalEventView { seq, event });
        }
        Ok(scan)
    }

    #[cfg(test)]
    fn read_registry_for_test(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        self.load_registry()
    }
}

#[derive(Default)]
struct TaskJournalEventScan {
    events: Vec<TaskJournalEventView>,
    last_event_seq: usize,
    has_more: bool,
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

fn normalize_finish_status(status: &str, error: Option<&str>) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "done" | "ok" | "success" | "succeeded" => "done",
        "failed" | "failure" | "error" | "errored" => "failed",
        "canceled" | "cancelled" | "cancel" | "stopped" => "canceled",
        "interrupted" => "interrupted",
        "finished" if looks_canceled(error) => "canceled",
        "finished" if has_error(error) => "failed",
        "finished" => "finished",
        _ if looks_canceled(error) => "canceled",
        _ if has_error(error) => "failed",
        _ => "finished",
    }
}

fn normalize_finish_error(error: Option<&str>) -> Option<String> {
    error
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_chars(value, MAX_ERROR_CHARS))
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "finished" | "done" | "failed" | "canceled" | "cancelled" | "interrupted"
    )
}

fn has_error(error: Option<&str>) -> bool {
    error.map(str::trim).is_some_and(|value| !value.is_empty())
}

fn looks_canceled(error: Option<&str>) -> bool {
    let Some(error) = error.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let lower = error.to_ascii_lowercase();
    lower.contains("cancel")
        || lower.contains("cancelled")
        || lower.contains("canceled")
        || lower.contains("stopped")
        || error.contains("取消")
        || error.contains("停止")
        || error.contains("终止")
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
    fn preserves_explicit_terminal_outcome_from_generic_cleanup() {
        let dir = unique_test_dir("terminal-outcome");
        let journal = TaskJournal::new(&dir);
        journal
            .record_started(TaskJournalStart {
                req_id: "req-1",
                cli_name: "server-runtime",
                route: Some("route_b_api_runtime"),
                run_handle_id: Some("req-1"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("started event should persist");
        journal
            .record_finished_with_outcome("req-1", "canceled", Some("用户已停止 PC CLI 任务"))
            .expect("terminal outcome should persist");
        journal
            .record_finished("req-1")
            .expect("generic cleanup should not overwrite terminal status");

        let registry = journal
            .read_registry_for_test()
            .expect("registry should read");
        let record = registry.get("req-1").expect("record should exist");
        assert_eq!(record.status, "canceled");

        let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
        assert_eq!(events.matches(r#""type":"finished""#).count(), 1);
        assert!(events.contains(r#""status":"canceled""#));
        assert!(events.contains(r#""error":"用户已停止 PC CLI 任务""#));
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
    fn stress_snapshot_handles_many_interleaved_task_events() {
        let dir = unique_test_dir("stress-snapshot");
        let journal = TaskJournal::new(&dir);
        for task_index in 0..120 {
            let req_id = format!("req-{task_index:03}");
            journal
                .record_started(TaskJournalStart {
                    req_id: &req_id,
                    cli_name: if task_index % 2 == 0 {
                        "server-runtime"
                    } else {
                        "codex"
                    },
                    route: Some(if task_index % 2 == 0 {
                        "route_c_server_runtime"
                    } else {
                        "route_a_external_cli"
                    }),
                    run_handle_id: Some(&req_id),
                    cwd: Some("D:/demo"),
                    runtime_permission: Some("project_write"),
                })
                .expect("start event should persist");
            for chunk_index in 0..4 {
                journal
                    .record_cli_chunk(
                        &req_id,
                        "stdout",
                        &format!("task {task_index} chunk {chunk_index}\n"),
                    )
                    .expect("chunk should persist");
            }
            if task_index % 3 == 0 {
                journal
                    .record_finished(&req_id)
                    .expect("finish event should persist");
            }
        }

        let snapshot = journal
            .snapshot("req-000", 0, 3)
            .expect("snapshot should read under pressure");
        assert_eq!(snapshot.events.len(), 3);
        assert!(snapshot.has_more);
        assert!(snapshot.events.iter().all(|event| {
            event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-000")
        }));

        let latest = journal
            .latest_records(500)
            .expect("latest records should read under pressure");
        assert_eq!(latest.len(), 100, "latest_records clamps public output");
        assert!(latest
            .iter()
            .all(|record| record.req_id.starts_with("req-")));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stress_snapshot_streams_large_interleaved_journal() {
        let dir = unique_test_dir("stress-streaming-snapshot");
        let journal = TaskJournal::new(&dir);
        for task_index in 0..400 {
            let req_id = format!("req-{task_index:03}");
            journal
                .record_started(TaskJournalStart {
                    req_id: &req_id,
                    cli_name: "server-runtime",
                    route: Some("route_c_server_runtime"),
                    run_handle_id: Some(&req_id),
                    cwd: Some("D:/demo"),
                    runtime_permission: Some("project_write"),
                })
                .expect("start event should persist");
            for chunk_index in 0..8 {
                journal
                    .record_cli_chunk(
                        &req_id,
                        "runtime",
                        &format!("task {task_index} chunk {chunk_index}\n"),
                    )
                    .expect("chunk should persist");
            }
            if task_index % 5 == 0 {
                journal
                    .record_finished(&req_id)
                    .expect("finish event should persist");
            }
        }

        let snapshot = journal
            .snapshot("req-399", 0, 4)
            .expect("snapshot should stream large journal");
        assert_eq!(snapshot.events.len(), 4);
        assert!(snapshot.has_more);
        assert!(snapshot.last_event_seq > 3_000);
        assert!(snapshot.events.iter().all(|event| {
            event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-399")
        }));

        let cursor = snapshot.events.last().map(|event| event.seq).unwrap_or(0);
        let next = journal
            .snapshot("req-399", cursor, 20)
            .expect("cursor snapshot should continue target task only");
        assert!(next.events.iter().all(|event| {
            event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-399")
        }));
        assert!(next.events.len() >= 5);

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

    #[test]
    fn records_codex_session_for_task_resume() {
        let dir = unique_test_dir("codex-session");
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
            .record_codex_session("req-1", "scope-a", "session-uuid")
            .expect("codex session should persist");

        let snapshot = journal
            .snapshot("req-1", 0, 20)
            .expect("snapshot should read");
        let record = snapshot.record.expect("record should exist");
        assert_eq!(record.codex_session_id.as_deref(), Some("session-uuid"));
        assert_eq!(record.codex_session_scope_key.as_deref(), Some("scope-a"));
        assert!(record.codex_session_updated_at_ms.is_some());
        assert_eq!(
            journal
                .load_codex_session("scope-a")
                .expect("codex session should load")
                .as_deref(),
            Some("session-uuid")
        );
        assert!(snapshot.events.iter().any(|event| {
            event.event.get("type").and_then(|value| value.as_str()) == Some("codex_session")
                && event
                    .event
                    .get("session_id")
                    .and_then(|value| value.as_str())
                    == Some("session-uuid")
        }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn clears_stale_codex_session_for_fresh_retry() {
        let dir = unique_test_dir("codex-session-clear");
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
            .record_codex_session("req-1", "scope-a", "session-uuid")
            .expect("codex session should persist");
        journal
            .clear_codex_session("req-1", "scope-a")
            .expect("stale session should clear");

        let snapshot = journal
            .snapshot("req-1", 0, 20)
            .expect("snapshot should read");
        let record = snapshot.record.expect("record should exist");
        assert!(record.codex_session_id.is_none());
        assert!(record.codex_session_scope_key.is_none());
        assert_eq!(
            journal
                .load_codex_session("scope-a")
                .expect("codex session file should load"),
            None
        );
        assert!(snapshot.events.iter().any(|event| {
            event.event.get("type").and_then(|value| value.as_str())
                == Some("codex_session_cleared")
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
