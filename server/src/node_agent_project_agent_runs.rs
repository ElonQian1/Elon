// server/src/node_agent_project_agent_runs.rs

use anyhow::{Context, Result};
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_RUNS: usize = 50;
const DEFAULT_RUNS: usize = 20;
const MAX_EVENTS_PER_RUN: usize = 200;
const DEFAULT_EVENTS_PER_RUN: usize = 20;
const MAX_EVENT_LINES_SCANNED: usize = 2_000;

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
    runs: Vec<ProjectAgentRunSummary>,
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

pub(crate) async fn list_handler(Json(req): Json<ProjectAgentRunsReq>) -> impl IntoResponse {
    match list_project_agent_runs(&req) {
        Ok(response) => (StatusCode::OK, Json(json!(response))).into_response(),
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
        runs,
    })
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
    for line in reader.lines().take(MAX_EVENT_LINES_SCANNED) {
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
        Ok(ProjectAgentRunSummary {
            run_id: self.run_id,
            file_name: self.file_name,
            status: self.status,
            mode: self.mode,
            started_at: self.started_at,
            updated_at: self.updated_at,
            event_count: self.event_count,
            scanned_event_count: self.event_count.min(MAX_EVENT_LINES_SCANNED),
            truncated: self.event_count >= MAX_EVENT_LINES_SCANNED,
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

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-{label}-{}-{nanos}", std::process::id()))
    }
}
