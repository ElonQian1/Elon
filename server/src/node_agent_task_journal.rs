// server/src/node_agent_task_journal.rs

#[path = "node_agent_task_journal_cancel.rs"]
mod cancel;
#[path = "node_agent_cancel_tombstones.rs"]
mod cancel_tombstones;
#[path = "node_agent_task_journal_cursor.rs"]
mod cursor;
#[path = "node_agent_task_journal_dispatch.rs"]
mod dispatch;
#[path = "node_agent_task_journal_recovery.rs"]
mod recovery;
#[path = "node_agent_task_journal_terminal.rs"]
mod terminal;

use anyhow::{Context, Result};
use homecli_proto::CancelRequestAudit;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, ErrorKind, Write},
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    node_agent_task_approval_snapshot::{TaskApprovalJournalSnapshot, TaskApprovalJournalTracker},
    node_agent_task_journal_events::{
        cli_chunk_event, is_completed_terminal_status, is_terminal_status, normalize_finish_error,
        normalize_finish_status,
    },
    node_agent_task_journal_lock::with_task_journal_io_lock,
    node_agent_workspace_match::{canonical_or_original, record_cwd_matches_workspace},
};

pub(crate) use dispatch::TaskDispatchProgress;
use dispatch::{advance_dispatch_record, default_dispatch_schema};

#[derive(Clone, Debug)]
pub(crate) struct TaskJournal {
    dir: PathBuf,
    instance_epoch: Arc<str>,
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
    pub process_identity: Option<String>,
    #[serde(default)]
    pub codex_session_id: Option<String>,
    #[serde(default)]
    pub codex_session_scope_key: Option<String>,
    #[serde(default)]
    pub codex_session_updated_at_ms: Option<u128>,
    pub status: String,
    #[serde(default = "default_runtime_phase")]
    pub phase: String,
    #[serde(default)]
    pub current_command: Option<String>,
    #[serde(default)]
    pub last_progress_ms: Option<u128>,
    #[serde(default)]
    pub heartbeat_at_ms: Option<u128>,
    #[serde(default)]
    pub timeout_policy: Option<crate::node_agent_cli_runtime_policy::CliRuntimePolicy>,
    #[serde(default)]
    pub dispatch: Option<TaskDispatchProgress>,
    pub started_at_ms: u128,
    pub updated_at_ms: u128,
    pub cancel_requested_at_ms: Option<u128>,
    #[serde(default)]
    pub cancel_intent: Option<CancelIntentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CancelIntentRecord {
    pub action_id: String,
    pub task_id: String,
    pub task_started_at_ms: u128,
    #[serde(default)]
    pub run_handle_id: Option<String>,
    #[serde(default)]
    pub active_started_at_ms: Option<u128>,
    #[serde(default)]
    pub sidecar_session_id: Option<String>,
    pub audit: CancelRequestAudit,
    pub created_at_ms: u128,
    #[serde(default)]
    pub side_effect: Option<CancelSideEffectCommit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CancelSideEffectCommit {
    pub target_kind: String,
    pub target_id: String,
    pub committed_at_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CancelIntentTarget {
    pub run_handle_id: Option<String>,
    pub active_started_at_ms: Option<u128>,
    pub sidecar_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistCancelIntentOutcome {
    Pending(CancelIntentRecord),
    Committed(CancelIntentRecord),
    Terminal(String),
    Missing,
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
    pub approvals: TaskApprovalJournalSnapshot,
    pub events: Vec<TaskJournalEventView>,
    pub last_event_seq: usize,
    pub has_more: bool,
    pub cursor_epoch: String,
    pub requested_cursor_epoch: Option<String>,
    pub previous_cursor_epoch: Option<String>,
    pub cursor_reset: bool,
    pub requested_cursor: usize,
    pub old_cursor: usize,
    pub new_cursor: usize,
    pub resume_cursor: usize,
    pub sidecar_update_epoch: String,
}

impl TaskJournal {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            instance_epoch: Arc::from(uuid::Uuid::new_v4().simple().to_string()),
        }
    }

    pub(crate) fn default() -> Self {
        Self::new(super::state_path().with_file_name("task-journal"))
    }

    pub(crate) fn record_started(&self, start: TaskJournalStart<'_>) -> Result<()> {
        with_task_journal_io_lock(|| {
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
                process_identity: None,
                codex_session_id: None,
                codex_session_scope_key: None,
                codex_session_updated_at_ms: None,
                status: "running".to_string(),
                phase: "dispatch".to_string(),
                current_command: None,
                last_progress_ms: Some(now),
                heartbeat_at_ms: Some(now),
                timeout_policy: None,
                dispatch: Some(TaskDispatchProgress {
                    schema: default_dispatch_schema(),
                    stage: "persisted".to_string(),
                    stage_started_at_ms: now,
                    stages: Vec::new(),
                    failure: None,
                }),
                started_at_ms: now,
                updated_at_ms: now,
                cancel_requested_at_ms: None,
                cancel_intent: None,
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
        })
    }

    pub(crate) fn load_codex_session(&self, scope_key: &str) -> Result<Option<String>> {
        with_task_journal_io_lock(|| {
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
        })
    }

    pub(crate) fn record_codex_session(
        &self,
        req_id: &str,
        scope_key: &str,
        session_id: &str,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
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
        })
    }

    pub(crate) fn clear_codex_session(&self, req_id: &str, scope_key: &str) -> Result<()> {
        with_task_journal_io_lock(|| {
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
        })
    }

    pub(crate) fn record_process_started(&self, req_id: &str, pid: u32) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let process_identity = crate::node_agent_cli_worker::process_identity(pid);
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                advance_dispatch_record(record, "sidecar_heartbeat", now);
                record.os_pid = Some(pid);
                record.process_started_at_ms = Some(now);
                record.process_identity.clone_from(&process_identity);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)?;
            self.append_event(json!({
                "type": "process_started",
                "req_id": req_id,
                "pid": pid,
                "process_identity": process_identity,
                "at_ms": now
            }))
        })
    }

    pub(crate) fn configure_runtime_policy(
        &self,
        req_id: &str,
        policy: &crate::node_agent_cli_runtime_policy::CliRuntimePolicy,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                record.timeout_policy = Some(policy.clone());
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)
        })
    }

    pub(crate) fn record_runtime_progress(
        &self,
        req_id: &str,
        phase: &str,
        current_command: Option<&str>,
    ) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                if is_terminal_status(&record.status) {
                    return Ok(());
                }
                advance_dispatch_record(record, "active", now);
                record.phase = normalize_runtime_phase(phase).to_string();
                record.current_command = current_command
                    .map(crate::node_agent_cli_output_aggregate::sanitize_command)
                    .filter(|command| !command.is_empty());
                record.last_progress_ms = Some(now);
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)
        })
    }

    pub(crate) fn record_runtime_heartbeat(&self, req_id: &str) -> Result<()> {
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                if is_terminal_status(&record.status) {
                    return Ok(());
                }
                if record
                    .dispatch
                    .as_ref()
                    .is_some_and(|dispatch| dispatch.stage == "sidecar_heartbeat")
                {
                    advance_dispatch_record(record, "active", now);
                }
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)
        })
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
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let requested_status = normalize_finish_status(status, error);
            let mut effective_status = requested_status.to_string();
            let mut registry = self.load_registry()?;
            if let Some(record) = registry.get_mut(req_id) {
                if is_completed_terminal_status(&record.status) {
                    return Ok(());
                }
                record.status = requested_status.to_string();
                effective_status = record.status.clone();
                record.phase = terminal_runtime_phase(&effective_status).to_string();
                record.current_command = None;
                record.heartbeat_at_ms = Some(now);
                record.updated_at_ms = now;
            }
            self.save_registry(&registry)?;

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
        })
    }

    pub(crate) fn record_cli_chunk(&self, req_id: &str, stream: &str, text: &str) -> Result<()> {
        let text = crate::node_agent_cli_redaction::redact_text(text);
        with_task_journal_io_lock(|| match cli_chunk_event(req_id, stream, &text, now_ms()) {
            Some(event) => self.append_event(event),
            None => Ok(()),
        })
    }

    /// Reconstruct a bounded public transcript for the durable completion envelope.
    /// This reads only the current task's already-redacted journal events and never
    /// persists the prompt or environment secrets.
    pub(crate) fn completion_output(&self, req_id: &str, max_chars: usize) -> Result<String> {
        with_task_journal_io_lock(|| {
            let path = self.events_path();
            if !path.exists() || max_chars == 0 {
                return Ok(String::new());
            }
            let file = File::open(&path).with_context(|| format!("读取 {:?}", path))?;
            let mut output = String::new();
            let mut remaining = max_chars;
            for line in BufReader::new(file).lines() {
                let line = line.with_context(|| format!("读取 {:?}", path))?;
                let Ok(event) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if event.get("req_id").and_then(Value::as_str) != Some(req_id) {
                    continue;
                }
                let Some(text) = event.get("text").and_then(Value::as_str) else {
                    continue;
                };
                if remaining == 0 {
                    break;
                }
                let chunk: String = text.chars().take(remaining).collect();
                remaining = remaining.saturating_sub(chunk.chars().count());
                output.push_str(&chunk);
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            Ok(output)
        })
    }

    pub(crate) fn latest_records(&self, limit: usize) -> Result<Vec<TaskJournalRecord>> {
        self.latest_records_matching(limit, |_| true)
    }

    pub(crate) fn latest_records_for_workspace(
        &self,
        workspace: &Path,
        limit: usize,
    ) -> Result<Vec<TaskJournalRecord>> {
        let workspace = canonical_or_original(workspace);
        self.latest_records_matching(limit, |record| {
            record_cwd_matches_workspace(record.cwd.as_deref(), &workspace)
        })
    }

    fn latest_records_matching(
        &self,
        limit: usize,
        mut matches_record: impl FnMut(&TaskJournalRecord) -> bool,
    ) -> Result<Vec<TaskJournalRecord>> {
        with_task_journal_io_lock(|| {
            let mut records: Vec<_> = self
                .load_registry()?
                .into_values()
                .filter(|record| matches_record(record))
                .collect();
            records.sort_by(|left, right| {
                right
                    .updated_at_ms
                    .cmp(&left.updated_at_ms)
                    .then_with(|| right.started_at_ms.cmp(&left.started_at_ms))
            });
            records.truncate(limit.min(100));
            Ok(records)
        })
    }

    fn load_registry(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        let path = self.registry_path();
        if !path.exists() {
            let backup_path = self.registry_backup_path();
            if backup_path.exists() {
                return load_registry_file(&backup_path);
            }
            return Ok(BTreeMap::new());
        }
        match load_registry_file(&path) {
            Ok(registry) => Ok(registry),
            Err(primary_error) => {
                let backup_path = self.registry_backup_path();
                if !backup_path.exists() {
                    return Err(primary_error);
                }
                match load_registry_file(&backup_path) {
                    Ok(registry) => {
                        tracing::warn!(
                            path = %path.display(),
                            backup_path = %backup_path.display(),
                            error = %primary_error,
                            "task journal registry is unreadable; using last valid backup"
                        );
                        Ok(registry)
                    }
                    Err(backup_error) => Err(primary_error)
                        .with_context(|| format!("备用任务 registry 也无法读取: {backup_error}")),
                }
            }
        }
    }

    fn save_registry(&self, registry: &BTreeMap<String, TaskJournalRecord>) -> Result<()> {
        self.ensure_dir()?;
        let path = self.registry_path();
        let backup_path = self.registry_backup_path();
        let previous_path = self.registry_previous_path();
        let temp_path = self.registry_temp_path();
        let text = serde_json::to_string_pretty(registry)?;
        write_registry_temp_file(&temp_path, text.as_bytes())?;
        remove_file_if_exists(&previous_path)?;
        if path.exists() {
            let replacement_path = if load_registry_file(&path).is_ok() {
                previous_path.clone()
            } else {
                self.registry_corrupt_path()
            };
            remove_file_if_exists(&replacement_path)?;
            fs::rename(&path, &replacement_path).with_context(|| {
                format!("移动旧任务 registry {:?} -> {:?}", path, replacement_path)
            })?;
        }
        fs::rename(&temp_path, &path)
            .with_context(|| format!("替换任务 registry {:?} -> {:?}", temp_path, path))?;
        if let Err(error) = fs::copy(&path, &backup_path) {
            tracing::warn!(
                path = %path.display(),
                backup_path = %backup_path.display(),
                error = %error,
                "task journal registry backup update failed"
            );
        }
        remove_file_if_exists(&previous_path)?;
        Ok(())
    }

    pub(crate) fn append_event(&self, event: serde_json::Value) -> Result<()> {
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

    fn registry_backup_path(&self) -> PathBuf {
        self.dir.join("registry.json.bak")
    }

    fn registry_previous_path(&self) -> PathBuf {
        self.dir.join("registry.json.previous")
    }

    fn registry_temp_path(&self) -> PathBuf {
        self.dir
            .join(format!("registry.json.tmp-{}", process::id()))
    }

    fn registry_corrupt_path(&self) -> PathBuf {
        self.dir.join(format!(
            "registry-corrupt-{}-{}.json",
            now_ms(),
            process::id()
        ))
    }

    pub(crate) fn events_path(&self) -> PathBuf {
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
            scan.scanned_last_seq = scan.scanned_last_seq.max(seq);
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            if line.trim().is_empty() {
                continue;
            }
            let event = match serde_json::from_str(&line) {
                Ok(event) => event,
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        seq,
                        error = %error,
                        "skipping corrupt task journal event line"
                    );
                    continue;
                }
            };
            if !event_belongs_to_task(&event, task_id) {
                continue;
            }
            scan.approval_tracker.observe_event(seq, &event);
            if seq <= since {
                continue;
            }
            if scan.events.len() >= event_limit {
                scan.has_more = true;
                continue;
            }
            scan.last_event_seq = seq;
            scan.events.push(TaskJournalEventView { seq, event });
        }
        if scan.last_event_seq == 0 && scan.scanned_last_seq > since {
            scan.last_event_seq = scan.scanned_last_seq;
        }
        Ok(scan.finish())
    }

    #[cfg(test)]
    fn read_registry_for_test(&self) -> Result<BTreeMap<String, TaskJournalRecord>> {
        with_task_journal_io_lock(|| self.load_registry())
    }
}

pub(crate) use crate::node_agent_task_runtime_status::payload as runtime_status_payload;

fn normalize_runtime_phase(phase: &str) -> &'static str {
    match phase.trim().to_ascii_lowercase().as_str() {
        "dispatch" => "dispatch",
        "reasoning" => "reasoning",
        "command" => "command",
        "editing" => "editing",
        "verification" => "verification",
        "approval" => "approval",
        "finalizing" => "finalizing",
        "done" => "done",
        "failed" => "failed",
        "canceled" => "canceled",
        _ => "reasoning",
    }
}

fn terminal_runtime_phase(status: &str) -> &'static str {
    match status {
        "done" | "finished" => "done",
        "canceled" => "canceled",
        _ => "failed",
    }
}

fn default_runtime_phase() -> String {
    "reasoning".to_string()
}

#[derive(Default)]
struct TaskJournalEventScan {
    events: Vec<TaskJournalEventView>,
    last_event_seq: usize,
    scanned_last_seq: usize,
    has_more: bool,
    approval_tracker: TaskApprovalJournalTracker,
    approvals: TaskApprovalJournalSnapshot,
}

impl TaskJournalEventScan {
    fn finish(mut self) -> Self {
        self.approvals = std::mem::take(&mut self.approval_tracker).finish();
        self
    }
}

fn event_belongs_to_task(event: &Value, task_id: &str) -> bool {
    event
        .get("req_id")
        .and_then(|value| value.as_str())
        .is_some_and(|req_id| req_id == task_id)
}

fn load_registry_file(path: &Path) -> Result<BTreeMap<String, TaskJournalRecord>> {
    let text = fs::read_to_string(path).with_context(|| format!("读取 {:?}", path))?;
    serde_json::from_str(&text).with_context(|| format!("解析 {:?}", path))
}

fn write_registry_temp_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("创建 {:?}", path))?;
    file.write_all(bytes)
        .with_context(|| format!("写入 {:?}", path))?;
    file.sync_all().with_context(|| format!("同步 {:?}", path))
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("删除 {:?}", path)),
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "node_agent_task_journal_tests.rs"]
mod tests;
