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
    node_agent_cli_sidecar::{now_ms, sidecar_status_view},
    node_agent_project_agent_recovery::{recovery_entry_from, ProjectAgentRunRecoveryEntry},
    node_agent_task_approval_snapshot::TaskApprovalJournalSnapshot,
    node_agent_task_journal::TaskJournalRecord,
    node_agent_task_resume::{
        task_attach_state, task_resume_contract, task_resume_contract_with_journal_approvals,
        TaskAttachState, TaskResumeContract,
    },
    node_agent_workspace_match::record_cwd_matches_workspace,
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
    sidecar_sessions: Vec<Value>,
    active_controls: Vec<ProjectAgentRunControl>,
    recent_tasks: Vec<ProjectAgentRunTaskResume>,
    runs: Vec<ProjectAgentRunSummary>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProjectAgentRunControl {
    pub(crate) task_id: String,
    pub(crate) run_handle_id: String,
    pub(crate) cli_name: String,
    pub(crate) route: String,
    pub(crate) cwd: Option<String>,
    pub(crate) runtime_permission: Option<String>,
    pub(crate) started_at_ms: u128,
    pub(crate) last_heartbeat_ms: u128,
    pub(crate) control_lease_expires_at_ms: u128,
    pub(crate) os_pid: Option<u32>,
    pub(crate) can_cancel: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProjectAgentRunTaskResume {
    pub(crate) task_id: String,
    pub(crate) cli_name: String,
    pub(crate) route: Option<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) runtime_permission: Option<String>,
    pub(crate) status: String,
    pub(crate) started_at_ms: u128,
    pub(crate) updated_at_ms: u128,
    pub(crate) cancel_requested_at_ms: Option<u128>,
    pub(crate) attach: TaskAttachState,
    pub(crate) resume: TaskResumeContract,
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
                        .task_journal_snapshot(task_id, 0, 1, None)
                        .ok()
                        .map(|snapshot| snapshot.approvals)
                },
            );
            response.recovery_entry =
                recovery_entry_from(&response.active_controls, &response.recent_tasks);
            response.sidecar_sessions =
                project_sidecar_sessions(&runtime, &workspace, RECENT_TASKS_LIMIT);
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
        sidecar_sessions: Vec::new(),
        active_controls: Vec::new(),
        recent_tasks: Vec::new(),
        runs,
    })
}

fn project_sidecar_sessions(runtime: &NodeRuntime, workspace: &Path, limit: usize) -> Vec<Value> {
    let now = now_ms();
    let relevant_task_ids: BTreeSet<String> = runtime
        .task_journal_records_for_workspace(workspace, RECENT_TASK_CANDIDATE_LIMIT)
        .unwrap_or_default()
        .into_iter()
        .map(|record| record.req_id)
        .collect();
    runtime
        .cli_sidecars
        .latest_sessions(50)
        .unwrap_or_default()
        .into_iter()
        .filter(|session| session.is_attachable_at(now))
        .filter(|session| {
            record_cwd_matches_workspace(session.cwd.as_deref(), workspace)
                || relevant_task_ids.contains(&session.task_id)
        })
        .take(limit)
        .map(|session| sidecar_status_view(&session))
        .collect()
}

#[cfg(test)]
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

#[cfg(test)]
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
#[path = "node_agent_project_agent_runs_tests.rs"]
mod tests;
