// server/src/node_agent_project_agent_runs.rs

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use crate::{
    node_agent_active_task::ActiveCliPromptView,
    node_agent_task_approval_snapshot::TaskApprovalJournalSnapshot,
    node_agent_task_journal::TaskJournalRecord,
    node_agent_task_resume::{
        task_attach_state, task_resume_contract, task_resume_contract_with_journal_approvals,
        TaskAttachState, TaskResumeContract,
    },
    NodeRuntime,
};

const MAX_RUNS: usize = 50;
const DEFAULT_RUNS: usize = 20;
const MAX_EVENTS_PER_RUN: usize = 200;
const DEFAULT_EVENTS_PER_RUN: usize = 20;
const RECENT_TASK_CANDIDATE_LIMIT: usize = 100;
const RECENT_TASKS_LIMIT: usize = 6;

#[derive(Debug, Deserialize)]
pub(crate) struct ProjectAgentRunsReq {
    workspace_path: String,
    limit: Option<usize>,
    event_limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunsResponse {
    ok: bool,
    workspace_path: String,
    log_dir: String,
    recovery_entry: Option<ProjectAgentRunRecoveryEntry>,
    active_controls: Vec<ProjectAgentRunControl>,
    recent_tasks: Vec<ProjectAgentRunTaskResume>,
    runs: Vec<ProjectAgentRunSummary>,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunRecoveryEntry {
    kind: String,
    task_id: String,
    cli_name: String,
    route: Option<String>,
    cwd: Option<String>,
    runtime_permission: Option<String>,
    status: String,
    recommended_action: String,
    reason: String,
    can_cancel: bool,
    can_continue: bool,
    updated_at_ms: Option<u128>,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunControl {
    task_id: String,
    run_handle_id: String,
    cli_name: String,
    route: String,
    cwd: Option<String>,
    runtime_permission: Option<String>,
    started_at_ms: u128,
    last_heartbeat_ms: u128,
    control_lease_expires_at_ms: u128,
    os_pid: Option<u32>,
    can_cancel: bool,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunTaskResume {
    task_id: String,
    cli_name: String,
    route: Option<String>,
    cwd: Option<String>,
    runtime_permission: Option<String>,
    status: String,
    started_at_ms: u128,
    updated_at_ms: u128,
    cancel_requested_at_ms: Option<u128>,
    attach: TaskAttachState,
    resume: TaskResumeContract,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunSummary {
    run_id: String,
    file_name: String,
    status: String,
    mode: Option<String>,
    started_at: Option<String>,
    updated_at: Option<String>,
    event_count: usize,
    scanned_event_count: usize,
    truncated: bool,
    turn_count: usize,
    tool_count: usize,
    tool_names: Vec<String>,
    last_event_type: Option<String>,
    last_error: Option<String>,
    events: Vec<ProjectAgentRunEventView>,
}

#[derive(Debug, Serialize)]
struct ProjectAgentRunEventView {
    seq: usize,
    event_type: String,
    ts: Option<String>,
    data: Value,
}

pub(crate) async fn list_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<ProjectAgentRunsReq>,
) -> impl IntoResponse {
    match list_project_agent_runs(&req) {
        Ok(mut response) => {
            let workspace = PathBuf::from(&response.workspace_path);
            let active_views = runtime
                .active_cli_prompt_views_for_workspace(&workspace)
                .await;
            let active_task_ids: BTreeSet<String> = active_views
                .iter()
                .map(|view| view.req_id.clone())
                .collect();
            response.active_controls = active_views
                .into_iter()
                .map(ProjectAgentRunControl::from)
                .collect();
            let journal_records = runtime
                .task_journal_records_for_workspace(&workspace, RECENT_TASK_CANDIDATE_LIMIT)
                .unwrap_or_default()
                .into_iter()
                .take(RECENT_TASK_CANDIDATE_LIMIT)
                .collect();
            response.recent_tasks = recent_task_resume_views_with_journal_approvals(
                journal_records,
                &active_task_ids,
                RECENT_TASKS_LIMIT,
                |task_id| {
                    runtime
                        .task_journal_snapshot(task_id, 0, 1)
                        .ok()
                        .map(|snapshot| snapshot.approvals)
                },
            );
            response.recovery_entry =
                recovery_entry_from(&response.active_controls, &response.recent_tasks);
            (StatusCode::OK, Json(json!(response))).into_response()
        }
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": error.to_string(),
            })),
        )
            .into_response(),
    }
}

fn list_project_agent_runs(req: &ProjectAgentRunsReq) -> Result<ProjectAgentRunsResponse> {
    let workspace = validate_workspace(&req.workspace_path)?;
    let log_dir = workspace.join(".elon").join("agent-runs");
    let limit = req.limit.unwrap_or(DEFAULT_RUNS).clamp(1, MAX_RUNS);
    let event_limit = req
        .event_limit
        .unwrap_or(DEFAULT_EVENTS_PER_RUN)
        .clamp(0, MAX_EVENTS_PER_RUN);

    let mut files = agent_run_files(&log_dir)?;
    files.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| right.path.cmp(&left.path))
    });

    let runs = files
        .into_iter()
        .take(limit)
        .map(|entry| parse_agent_run_file(&entry.path, event_limit))
        .collect::<Result<Vec<_>>>()?;

    Ok(ProjectAgentRunsResponse {
        ok: true,
        workspace_path: workspace.to_string_lossy().to_string(),
        log_dir: log_dir.to_string_lossy().to_string(),
        recovery_entry: None,
        active_controls: Vec::new(),
        recent_tasks: Vec::new(),
        runs,
    })
}

fn recovery_entry_from(
    active_controls: &[ProjectAgentRunControl],
    recent_tasks: &[ProjectAgentRunTaskResume],
) -> Option<ProjectAgentRunRecoveryEntry> {
    active_controls
        .iter()
        .find(|control| !control.task_id.trim().is_empty())
        .map(ProjectAgentRunRecoveryEntry::from_active_control)
        .or_else(|| {
            recent_tasks
                .iter()
                .find(|task| !task.task_id.trim().is_empty())
                .map(ProjectAgentRunRecoveryEntry::from_recent_task)
        })
}

impl ProjectAgentRunRecoveryEntry {
    fn from_active_control(control: &ProjectAgentRunControl) -> Self {
        Self {
            kind: "active_control".to_string(),
            task_id: control.task_id.clone(),
            cli_name: control.cli_name.clone(),
            route: Some(control.route.clone()),
            cwd: control.cwd.clone(),
            runtime_permission: control.runtime_permission.clone(),
            status: "running".to_string(),
            recommended_action: "wait_or_cancel".to_string(),
            reason: "当前本机节点仍持有运行控制句柄，PC 端应优先展示继续观察或停止入口。"
                .to_string(),
            can_cancel: control.can_cancel,
            can_continue: false,
            updated_at_ms: Some(control.last_heartbeat_ms),
        }
    }

    fn from_recent_task(task: &ProjectAgentRunTaskResume) -> Self {
        let recommended_action = task.resume.next_action().to_string();
        Self {
            kind: "snapshot_resume".to_string(),
            task_id: task.task_id.clone(),
            cli_name: task.cli_name.clone(),
            route: task.route.clone(),
            cwd: task.cwd.clone(),
            runtime_permission: task.runtime_permission.clone(),
            status: task.resume.status().to_string(),
            can_cancel: false,
            can_continue: recommended_action == "continue_from_snapshot",
            recommended_action,
            reason: task.resume.reason().to_string(),
            updated_at_ms: Some(task.updated_at_ms),
        }
    }
}

fn recent_task_resume_views(
    records: Vec<TaskJournalRecord>,
    active_task_ids: &BTreeSet<String>,
    limit: usize,
) -> Vec<ProjectAgentRunTaskResume> {
    recent_task_resume_views_with_journal_approvals(records, active_task_ids, limit, |_| None)
}

fn recent_task_resume_views_with_journal_approvals(
    records: Vec<TaskJournalRecord>,
    active_task_ids: &BTreeSet<String>,
    limit: usize,
    mut approvals_for_task: impl FnMut(&str) -> Option<TaskApprovalJournalSnapshot>,
) -> Vec<ProjectAgentRunTaskResume> {
    let mut seen = BTreeSet::new();
    records
        .into_iter()
        .filter(|record| !active_task_ids.contains(&record.req_id))
        .filter(|record| seen.insert(record.req_id.clone()))
        .take(limit)
        .map(|record| {
            let approvals = approvals_for_task(&record.req_id);
            project_task_resume_view_with_approvals(record, approvals.as_ref())
        })
        .collect()
}

impl From<ActiveCliPromptView> for ProjectAgentRunControl {
    fn from(view: ActiveCliPromptView) -> Self {
        Self {
            task_id: view.req_id,
            run_handle_id: view.run_handle_id,
            cli_name: view.cli_name,
            route: view.route,
            cwd: view.cwd,
            runtime_permission: view.runtime_permission,
            started_at_ms: view.started_at_ms,
            last_heartbeat_ms: view.last_heartbeat_ms,
            control_lease_expires_at_ms: view.control_lease_expires_at_ms,
            os_pid: view.os_pid,
            can_cancel: view.control_handle_live,
        }
    }
}

fn project_task_resume_view(record: TaskJournalRecord) -> ProjectAgentRunTaskResume {
    project_task_resume_view_with_approvals(record, None)
}

fn project_task_resume_view_with_approvals(
    record: TaskJournalRecord,
    approvals: Option<&TaskApprovalJournalSnapshot>,
) -> ProjectAgentRunTaskResume {
    let attach = task_attach_state(Some(&record), None);
    let resume = approvals
        .map(|approvals| task_resume_contract_with_journal_approvals(&attach, approvals))
        .unwrap_or_else(|| task_resume_contract(&attach));
    ProjectAgentRunTaskResume {
        task_id: record.req_id,
        cli_name: record.cli_name,
        route: record.route,
        cwd: record.cwd,
        runtime_permission: record.runtime_permission,
        status: record.status,
        started_at_ms: record.started_at_ms,
        updated_at_ms: record.updated_at_ms,
        cancel_requested_at_ms: record.cancel_requested_at_ms,
        attach,
        resume,
    }
}

fn validate_workspace(raw: &str) -> Result<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        anyhow::bail!("workspace_path 不能为空");
    }
    let path = PathBuf::from(raw);
    if !path.exists() {
        anyhow::bail!("PC 本地路径不存在: {raw}");
    }
    if !path.is_dir() {
        anyhow::bail!("workspace_path 必须指向一个目录");
    }
    fs::canonicalize(&path).with_context(|| format!("解析项目目录失败: {raw}"))
}

#[derive(Debug)]
struct AgentRunFile {
    path: PathBuf,
    modified: SystemTime,
}

fn agent_run_files(log_dir: &Path) -> Result<Vec<AgentRunFile>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    if !log_dir.is_dir() {
        anyhow::bail!(".elon/agent-runs 不是目录");
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(log_dir).with_context(|| format!("读取 {:?}", log_dir))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        files.push(AgentRunFile {
            path,
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }
    Ok(files)
}

fn parse_agent_run_file(path: &Path, event_limit: usize) -> Result<ProjectAgentRunSummary> {
    let file = File::open(path).with_context(|| format!("读取 {:?}", path))?;
    let reader = BufReader::new(file);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("agent-run.jsonl")
        .to_string();
    let fallback_run_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("agent-run")
        .to_string();

    let mut summary = AgentRunSummaryBuilder::new(fallback_run_id, file_name, event_limit);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        summary.observe_event(serde_json::from_str::<Value>(&line).with_context(|| {
            format!(
                "解析 agent run 日志失败: {}",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("agent-run.jsonl")
            )
        })?);
    }
    summary.finish()
}

struct AgentRunSummaryBuilder {
    run_id: String,
    file_name: String,
    status: String,
    mode: Option<String>,
    started_at: Option<String>,
    updated_at: Option<String>,
    event_count: usize,
    event_limit: usize,
    turn_numbers: BTreeSet<u64>,
    tool_names: BTreeSet<String>,
    tool_count: usize,
    last_event_type: Option<String>,
    last_error: Option<String>,
    events: Vec<ProjectAgentRunEventView>,
}

impl AgentRunSummaryBuilder {
    fn new(run_id: String, file_name: String, event_limit: usize) -> Self {
        Self {
            run_id,
            file_name,
            status: "unknown".to_string(),
            mode: None,
            started_at: None,
            updated_at: None,
            event_count: 0,
            event_limit,
            turn_numbers: BTreeSet::new(),
            tool_names: BTreeSet::new(),
            tool_count: 0,
            last_event_type: None,
            last_error: None,
            events: Vec::new(),
        }
    }

    fn observe_event(&mut self, event: Value) {
        self.event_count += 1;
        if let Some(run_id) = json_string(&event, "run_id") {
            self.run_id = run_id;
        }
        let event_type = json_string(&event, "type").unwrap_or_else(|| "unknown".to_string());
        let ts = json_string(&event, "ts");
        if self.started_at.is_none() {
            self.started_at = ts.clone();
        }
        if ts.is_some() {
            self.updated_at = ts.clone();
        }
        self.last_event_type = Some(event_type.clone());

        let data = event.get("data").cloned().unwrap_or(Value::Null);
        self.observe_data(&event_type, &data);
        if self.event_limit == 0 {
            return;
        }
        if self.events.len() == self.event_limit {
            self.events.remove(0);
        }
        self.events.push(ProjectAgentRunEventView {
            seq: self.event_count,
            event_type,
            ts,
            data,
        });
    }

    fn observe_data(&mut self, event_type: &str, data: &Value) {
        if event_type == "run_started" {
            self.status = "running".to_string();
            self.mode = json_string(data, "mode").or_else(|| self.mode.clone());
        } else if event_type == "run_finished" {
            self.status = "completed".to_string();
        } else if event_type == "run_failed" {
            self.status = "failed".to_string();
            self.last_error = json_string(data, "error").or_else(|| {
                data.get("details")
                    .and_then(|details| json_string(details, "error"))
            });
        }

        if let Some(turn) = data.get("turn").and_then(Value::as_u64) {
            self.turn_numbers.insert(turn);
        }
        if event_type == "tool_started" {
            self.tool_count += 1;
        }
        if let Some(tool) = json_string(data, "tool") {
            self.tool_names.insert(tool);
        }
    }

    fn finish(self) -> Result<ProjectAgentRunSummary> {
        let visible_event_count = self.events.len();
        Ok(ProjectAgentRunSummary {
            run_id: self.run_id,
            file_name: self.file_name,
            status: self.status,
            mode: self.mode,
            started_at: self.started_at,
            updated_at: self.updated_at,
            event_count: self.event_count,
            scanned_event_count: self.event_count,
            truncated: self.event_count > visible_event_count,
            turn_count: self.turn_numbers.len(),
            tool_count: self.tool_count,
            tool_names: self.tool_names.into_iter().collect(),
            last_event_type: self.last_event_type,
            last_error: self.last_error,
            events: self.events,
        })
    }
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lists_project_agent_runs_without_file_contents() {
        let root = temp_dir("agent-runs-list");
        let log_dir = root.join(".elon").join("agent-runs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::write(
            log_dir.join("run-1.jsonl"),
            [
                r#"{"ts":"2026-06-23T01:00:00Z","run_id":"run-1","type":"run_started","data":{"mode":"api-runtime","prompt_chars":12}}"#,
                r#"{"ts":"2026-06-23T01:00:01Z","run_id":"run-1","type":"turn_started","data":{"turn":1}}"#,
                r#"{"ts":"2026-06-23T01:00:02Z","run_id":"run-1","type":"tool_started","data":{"turn":1,"tool":"read_file","target":"README.md"}}"#,
                r#"{"ts":"2026-06-23T01:00:03Z","run_id":"run-1","type":"tool_finished","data":{"turn":1,"tool":"read_file","target":"README.md","result_chars":120}}"#,
                r#"{"ts":"2026-06-23T01:00:04Z","run_id":"run-1","type":"run_finished","data":{"status":"completed","run_commands_used":0}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let response = list_project_agent_runs(&ProjectAgentRunsReq {
            workspace_path: root.to_string_lossy().to_string(),
            limit: Some(10),
            event_limit: Some(3),
        })
        .unwrap();
        let _ = fs::remove_dir_all(root);

        assert!(response.ok);
        assert!(response.active_controls.is_empty());
        assert!(response.recent_tasks.is_empty());
        assert!(response.recovery_entry.is_none());
        assert_eq!(response.runs.len(), 1);
        let run = &response.runs[0];
        assert_eq!(run.run_id, "run-1");
        assert_eq!(run.status, "completed");
        assert_eq!(run.mode.as_deref(), Some("api-runtime"));
        assert_eq!(run.event_count, 5);
        assert_eq!(run.turn_count, 1);
        assert_eq!(run.tool_count, 1);
        assert_eq!(run.tool_names, vec!["read_file"]);
        assert_eq!(run.events.len(), 3);
        let serialized = serde_json::to_string(run).unwrap();
        assert!(!serialized.contains("file content"));
        assert!(!serialized.contains("prompt"));
    }

    #[test]
    fn missing_agent_runs_directory_returns_empty_list() {
        let root = temp_dir("agent-runs-empty");
        fs::create_dir_all(&root).unwrap();
        let response = list_project_agent_runs(&ProjectAgentRunsReq {
            workspace_path: root.to_string_lossy().to_string(),
            limit: None,
            event_limit: None,
        })
        .unwrap();
        let _ = fs::remove_dir_all(root);

        assert!(response.ok);
        assert!(response.runs.is_empty());
    }

    #[test]
    fn stress_agent_run_summary_reads_long_run_to_terminal_status() {
        let root = temp_dir("agent-runs-long-terminal");
        let log_dir = root.join(".elon").join("agent-runs");
        fs::create_dir_all(&log_dir).unwrap();
        let mut lines = Vec::new();
        lines.push(
            r#"{"ts":"2026-06-23T01:00:00Z","run_id":"run-long","type":"run_started","data":{"mode":"server-runtime","prompt_chars":22,"max_context_chars":60000}}"#
                .to_string(),
        );
        for turn in 1..=2_100 {
            lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:01Z","run_id":"run-long","type":"turn_started","data":{{"turn":{turn}}}}}"#
            ));
            lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:02Z","run_id":"run-long","type":"tool_started","data":{{"turn":{turn},"tool":"read_file","target":"src/lib.rs"}}}}"#
            ));
            lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:03Z","run_id":"run-long","type":"tool_finished","data":{{"turn":{turn},"tool":"read_file","target":"src/lib.rs","result_chars":64}}}}"#
            ));
        }
        lines.push(
            r#"{"ts":"2026-06-23T01:59:58Z","run_id":"run-long","type":"context_compacted","data":{"turn":2100,"before_chars":90000,"after_chars":42000,"omitted_messages":300,"omitted_chars":48000,"max_context_chars":60000,"compaction_count":7}}"#
                .to_string(),
        );
        lines.push(
            r#"{"ts":"2026-06-23T01:59:59Z","run_id":"run-long","type":"run_finished","data":{"status":"completed","run_commands_used":8,"context_compactions":7}}"#
                .to_string(),
        );
        fs::write(log_dir.join("run-long.jsonl"), lines.join("\n")).unwrap();

        let response = list_project_agent_runs(&ProjectAgentRunsReq {
            workspace_path: root.to_string_lossy().to_string(),
            limit: Some(1),
            event_limit: Some(5),
        })
        .unwrap();
        let _ = fs::remove_dir_all(root);

        let run = response.runs.first().expect("long run should be listed");
        assert_eq!(run.run_id, "run-long");
        assert_eq!(run.status, "completed");
        assert_eq!(run.mode.as_deref(), Some("server-runtime"));
        assert_eq!(run.event_count, 6_303);
        assert_eq!(run.scanned_event_count, run.event_count);
        assert!(run.truncated);
        assert_eq!(run.turn_count, 2_100);
        assert_eq!(run.tool_count, 2_100);
        assert_eq!(run.tool_names, vec!["read_file"]);
        assert_eq!(run.events.len(), 5);
        assert_eq!(
            run.events.last().map(|event| event.event_type.as_str()),
            Some("run_finished")
        );
        assert!(serde_json::to_string(run)
            .unwrap()
            .contains("context_compacted"));
    }

    #[test]
    fn rejects_missing_workspace() {
        let missing = temp_dir("agent-runs-missing");
        let error = list_project_agent_runs(&ProjectAgentRunsReq {
            workspace_path: missing.to_string_lossy().to_string(),
            limit: None,
            event_limit: None,
        })
        .unwrap_err();
        assert!(error.to_string().contains("PC 本地路径不存在"));
    }

    #[test]
    fn task_resume_view_uses_snapshot_continue_contract() {
        let view = project_task_resume_view(TaskJournalRecord {
            req_id: "req-detached".to_string(),
            cli_name: "server-runtime".to_string(),
            route: Some("route_c_server_runtime".to_string()),
            run_handle_id: Some("req-detached".to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: Some(42),
            process_started_at_ms: Some(100),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "cancel_requested".to_string(),
            started_at_ms: 100,
            updated_at_ms: 200,
            cancel_requested_at_ms: Some(180),
        });

        assert_eq!(view.task_id, "req-detached");
        assert_eq!(view.status, "cancel_requested");
        let serialized = serde_json::to_string(&view).unwrap();
        assert!(serialized.contains("continue_from_snapshot"));
        assert!(serialized.contains("tool_approval_recovery"));
        assert!(serialized.contains("lost_after_restart"));
        assert!(serialized.contains("本机 journal"));
        assert!(!serialized.contains("secret prompt"));
        assert!(!serialized.contains("sk-live-secret"));
    }

    #[test]
    fn task_resume_view_includes_journal_pending_approvals_without_enabling_clicks() {
        let mut approval_tracker =
            crate::node_agent_task_approval_snapshot::TaskApprovalJournalTracker::default();
        approval_tracker.observe_event(
            1,
            &json!({
                "type": "tool_approval_required",
                "approval_id": "tap_restart_pending",
                "tool": "write_file"
            }),
        );
        let approvals = approval_tracker.finish();
        let view = project_task_resume_view_with_approvals(
            TaskJournalRecord {
                req_id: "req-detached-approval".to_string(),
                cli_name: "server-runtime".to_string(),
                route: Some("route_c_server_runtime".to_string()),
                run_handle_id: Some("req-detached-approval".to_string()),
                cwd: Some("D:/demo".to_string()),
                runtime_permission: Some("project_write".to_string()),
                os_pid: Some(42),
                process_started_at_ms: Some(100),
                codex_session_id: None,
                codex_session_scope_key: None,
                codex_session_updated_at_ms: None,
                status: "running".to_string(),
                started_at_ms: 100,
                updated_at_ms: 200,
                cancel_requested_at_ms: None,
            },
            Some(&approvals),
        );
        let resume = serde_json::to_value(&view.resume).unwrap();

        assert_eq!(resume["status"], "detached");
        assert_eq!(resume["can_approve_tools"], false);
        assert_eq!(resume["active_approval_ids"], json!([]));
        assert_eq!(
            resume["tool_approval_recovery"]["journal_pending_approval_ids"],
            json!(["tap_restart_pending"])
        );
        assert_eq!(
            resume["tool_approval_recovery"]["journal_pending_count"],
            json!(1)
        );
        assert_eq!(
            resume["tool_approval_recovery"]["pending_after_restart_action"],
            "continue_from_snapshot"
        );
    }

    #[test]
    fn recent_task_resume_views_filter_active_ids_dedupe_and_cap() {
        let records = vec![
            task_record("req-live", 900),
            task_record("req-9", 899),
            task_record("req-8", 898),
            task_record("req-8", 897),
            task_record("req-7", 896),
            task_record("req-6", 895),
            task_record("req-5", 894),
            task_record("req-4", 893),
            task_record("req-3", 892),
        ];
        let active = BTreeSet::from(["req-live".to_string()]);

        let views = recent_task_resume_views(records, &active, 6);

        assert_eq!(views.len(), 6);
        assert_eq!(
            views
                .iter()
                .map(|view| view.task_id.as_str())
                .collect::<Vec<_>>(),
            vec!["req-9", "req-8", "req-7", "req-6", "req-5", "req-4"]
        );
        assert!(views.iter().all(|view| {
            let resume = serde_json::to_value(&view.resume).expect("resume should serialize");
            resume["next_action"] == "continue_from_snapshot" && resume["can_cancel"] == false
        }));
    }

    #[test]
    fn recovery_entry_prefers_live_control_handle() {
        let control = ProjectAgentRunControl {
            task_id: "req-live".to_string(),
            run_handle_id: "req-live".to_string(),
            cli_name: "server-runtime".to_string(),
            route: "route_c_server_runtime".to_string(),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            started_at_ms: 100,
            last_heartbeat_ms: 200,
            control_lease_expires_at_ms: 47_000,
            os_pid: Some(1234),
            can_cancel: true,
        };
        let recent = project_task_resume_view(task_record("req-detached", 190));

        let entry = recovery_entry_from(&[control], &[recent]).expect("entry should exist");

        assert_eq!(entry.kind, "active_control");
        assert_eq!(entry.task_id, "req-live");
        assert_eq!(entry.status, "running");
        assert_eq!(entry.recommended_action, "wait_or_cancel");
        assert!(entry.can_cancel);
        assert!(!entry.can_continue);
    }

    #[test]
    fn recovery_entry_points_to_snapshot_continue_without_secrets() {
        let mut record = task_record("req-detached", 900);
        record.codex_session_id = Some("session-secret-uuid".to_string());
        record.codex_session_scope_key = Some("scope-secret".to_string());
        let recent = project_task_resume_view(record);

        let entry = recovery_entry_from(&[], &[recent]).expect("entry should exist");
        let serialized = serde_json::to_string(&entry).unwrap();

        assert_eq!(entry.kind, "snapshot_resume");
        assert_eq!(entry.task_id, "req-detached");
        assert_eq!(entry.status, "terminal");
        assert_eq!(entry.recommended_action, "continue_from_snapshot");
        assert!(!entry.can_cancel);
        assert!(entry.can_continue);
        assert!(serialized.contains("本机进程已经结束"));
        assert!(!serialized.contains("session-secret-uuid"));
        assert!(!serialized.contains("scope-secret"));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-{label}-{}-{nanos}", std::process::id()))
    }

    fn task_record(req_id: &str, updated_at_ms: u128) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: req_id.to_string(),
            cli_name: "server-runtime".to_string(),
            route: Some("route_c_server_runtime".to_string()),
            run_handle_id: Some(req_id.to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: None,
            process_started_at_ms: None,
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "canceled".to_string(),
            started_at_ms: updated_at_ms.saturating_sub(10),
            updated_at_ms,
            cancel_requested_at_ms: Some(updated_at_ms.saturating_sub(1)),
        }
    }
}
