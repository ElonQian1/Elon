// server/src/node_agent_client_diagnostics.rs

use axum::{http::StatusCode, Json};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::process::{Command, Stdio};

const MAX_LATEST_TASKS: usize = 20;

pub(crate) async fn export_handler() -> (StatusCode, Json<Value>) {
    match export_diagnostics() {
        Ok(export) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "path": path_to_string(&export.path),
                "opened": export.opened,
                "message": "已生成客户端诊断信息。"
            })),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": error })),
        ),
    }
}

#[cfg(windows)]
pub(crate) fn export_diagnostics_file() -> Result<(PathBuf, bool), String> {
    export_diagnostics().map(|export| (export.path, export.opened))
}

fn export_diagnostics() -> Result<ClientDiagnosticExport, String> {
    let paths = DiagnosticPaths::default();
    fs::create_dir_all(&paths.diagnostics_dir).map_err(|error| {
        format!(
            "无法创建诊断目录 {}: {error}",
            paths.diagnostics_dir.display()
        )
    })?;
    let export_path = paths
        .diagnostics_dir
        .join(format!("elon-node-diagnostic-{}.json", now_ms()));
    let payload = diagnostic_payload(&paths);
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("无法生成诊断 JSON: {error}"))?;
    fs::write(&export_path, text)
        .map_err(|error| format!("无法写入诊断文件 {}: {error}", export_path.display()))?;
    let opened = open_export_path(&export_path).unwrap_or(false);
    Ok(ClientDiagnosticExport {
        path: export_path,
        opened,
    })
}

fn diagnostic_payload(paths: &DiagnosticPaths) -> Value {
    json!({
        "schema": "elon_pc_node_client_diagnostics.v1",
        "generated_at_ms": now_ms(),
        "privacy": {
            "raw_prompt_exported": false,
            "raw_cli_output_exported": false,
            "maintenance_log_contents_exported": false,
            "maintenance_log_details_exported": false,
            "token_values_exported": false,
            "api_key_values_exported": false
        },
        "runtime": runtime_summary(),
        "paths": {
            "state_file": path_to_string(&paths.state_file),
            "config_dir": path_to_string(&paths.config_dir),
            "task_journal_dir": path_to_string(&paths.task_journal_dir),
            "logs_dir": path_to_string(&paths.logs_dir),
            "maintenance_log_file": path_to_string(&paths.maintenance_log_file),
            "launcher_logs_dir": optional_path_to_value(paths.launcher_logs_dir.as_deref()),
            "launcher_log_file": optional_path_to_value(paths.launcher_log_file.as_deref()),
            "diagnostics_dir": path_to_string(&paths.diagnostics_dir)
        },
        "files": diagnostic_files(paths),
        "logs": crate::node_agent_client_diagnostic_logs::diagnostic_log_summary(
            &paths.maintenance_log_file,
            paths.launcher_log_file.as_deref(),
        ),
        "env": redacted_env_summary(),
        "install": install_summary(),
        "tasks": task_journal_summary(paths),
    })
}

fn runtime_summary() -> Value {
    json!({
        "platform": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
        "current_exe": std::env::current_exe().ok().map(|path| path_to_string(&path)),
        "current_dir": std::env::current_dir().ok().map(|path| path_to_string(&path)),
    })
}

fn diagnostic_files(paths: &DiagnosticPaths) -> Value {
    json!({
        "state_file": file_meta(&paths.state_file),
        "config_dir": file_meta(&paths.config_dir),
        "logs_dir": file_meta(&paths.logs_dir),
        "maintenance_log": file_meta(&paths.maintenance_log_file),
        "launcher_logs_dir": optional_file_meta(paths.launcher_logs_dir.as_deref()),
        "launcher_log": optional_file_meta(paths.launcher_log_file.as_deref()),
        "task_journal_dir": file_meta(&paths.task_journal_dir),
        "task_registry": file_meta(&paths.task_journal_dir.join("registry.json")),
        "task_events": file_meta(&paths.task_journal_dir.join("events.jsonl")),
        "codex_sessions": file_meta(&paths.task_journal_dir.join("codex-sessions.json")),
    })
}

fn redacted_env_summary() -> Value {
    let vars = [
        "ELON_SERVER_URL",
        "ELON_SERVER_TOKEN",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
    ];
    let mut object = Map::new();
    for name in vars {
        object.insert(name.to_string(), json!(env_var_configured(name)));
    }
    Value::Object(object)
}

fn env_var_configured(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn install_summary() -> Value {
    crate::node_agent_client_install_status::status_payload()
}

fn task_journal_summary(paths: &DiagnosticPaths) -> Value {
    let registry_path = paths.task_journal_dir.join("registry.json");
    let events_path = paths.task_journal_dir.join("events.jsonl");
    json!({
        "latest_records": latest_safe_task_records(&registry_path),
        "events": events_summary(&events_path),
    })
}

fn latest_safe_task_records(registry_path: &Path) -> Value {
    let Some(registry) = safe_json_file(registry_path) else {
        return json!([]);
    };
    let Some(object) = registry.as_object() else {
        return json!([]);
    };
    let mut records: Vec<Value> = object.values().map(safe_task_record).collect();
    records.sort_by(|left, right| {
        value_u128(right.get("updated_at_ms")).cmp(&value_u128(left.get("updated_at_ms")))
    });
    records.truncate(MAX_LATEST_TASKS);
    Value::Array(records)
}

fn safe_task_record(value: &Value) -> Value {
    json!({
        "req_id": value.get("req_id").and_then(Value::as_str),
        "cli_name": value.get("cli_name").and_then(Value::as_str),
        "route": value.get("route").and_then(Value::as_str),
        "run_handle_id": value.get("run_handle_id").and_then(Value::as_str),
        "cwd": value.get("cwd").and_then(Value::as_str),
        "runtime_permission": value.get("runtime_permission").and_then(Value::as_str),
        "status": value.get("status").and_then(Value::as_str),
        "os_pid": value.get("os_pid").and_then(Value::as_u64),
        "started_at_ms": value_u128(value.get("started_at_ms")),
        "updated_at_ms": value_u128(value.get("updated_at_ms")),
        "cancel_requested_at_ms": value_u128(value.get("cancel_requested_at_ms")),
        "codex_session_present": value
            .get("codex_session_id")
            .and_then(Value::as_str)
            .map(|session_id| !session_id.trim().is_empty())
            .unwrap_or(false),
    })
}

fn events_summary(events_path: &Path) -> Value {
    let text = match fs::read_to_string(events_path) {
        Ok(text) => text,
        Err(_) => {
            return json!({
                "line_count": 0,
                "parse_errors": 0,
                "types": {},
            });
        }
    };
    let mut line_count = 0u64;
    let mut parse_errors = 0u64;
    let mut types = BTreeMap::<String, u64>::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        line_count += 1;
        match serde_json::from_str::<Value>(line) {
            Ok(value) => {
                let event_type = value
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                *types.entry(event_type.to_string()).or_default() += 1;
            }
            Err(_) => parse_errors += 1,
        }
    }
    json!({
        "line_count": line_count,
        "parse_errors": parse_errors,
        "types": types,
    })
}

fn safe_json_file(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn file_meta(path: &Path) -> Value {
    match fs::metadata(path) {
        Ok(meta) => json!({
            "path": path_to_string(path),
            "exists": true,
            "is_dir": meta.is_dir(),
            "len": meta.len(),
            "modified_ms": meta.modified().ok().and_then(system_time_ms),
        }),
        Err(_) => json!({
            "path": path_to_string(path),
            "exists": false,
        }),
    }
}

fn optional_file_meta(path: Option<&Path>) -> Value {
    path.map(file_meta).unwrap_or_else(|| {
        json!({
            "exists": false,
            "reason": "launcher_log_path_unavailable",
        })
    })
}

fn value_u128(value: Option<&Value>) -> u128 {
    value
        .and_then(Value::as_u64)
        .map(u128::from)
        .unwrap_or_default()
}

fn now_ms() -> u128 {
    system_time_ms(SystemTime::now()).unwrap_or_default()
}

fn system_time_ms(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn optional_path_to_value(path: Option<&Path>) -> Value {
    path.map(path_to_string)
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn open_export_path(path: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    {
        let mut command = Command::new("explorer.exe");
        command.arg(format!("/select,{}", path.display()));
        apply_hidden_window(&mut command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法打开诊断文件位置 {}: {error}", path.display()))?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(false)
    }
}

#[cfg(windows)]
fn apply_hidden_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

struct ClientDiagnosticExport {
    path: PathBuf,
    opened: bool,
}

struct DiagnosticPaths {
    state_file: PathBuf,
    config_dir: PathBuf,
    task_journal_dir: PathBuf,
    logs_dir: PathBuf,
    maintenance_log_file: PathBuf,
    launcher_logs_dir: Option<PathBuf>,
    launcher_log_file: Option<PathBuf>,
    diagnostics_dir: PathBuf,
}

impl DiagnosticPaths {
    fn default() -> Self {
        let state_file = crate::state_path();
        Self::from_state_file(state_file)
    }

    fn from_state_file(state_file: PathBuf) -> Self {
        let config_dir = state_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let task_journal_dir = state_file.with_file_name("task-journal");
        let logs_dir = config_dir.join("logs");
        let diagnostics_dir = config_dir.join("diagnostics");
        let launcher_logs_dir = launcher_logs_dir();
        let launcher_log_file = launcher_logs_dir
            .as_ref()
            .map(|dir| dir.join("client-launcher.jsonl"));
        Self {
            state_file,
            config_dir,
            task_journal_dir,
            launcher_logs_dir,
            launcher_log_file,
            maintenance_log_file: logs_dir.join("client-maintenance.jsonl"),
            logs_dir,
            diagnostics_dir,
        }
    }
}

fn launcher_logs_dir() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            PathBuf::from(value)
                .join("ElonNode")
                .join("_internal")
                .join("logs")
        })
}

#[cfg(test)]
mod tests {
    use super::{diagnostic_payload, DiagnosticPaths};
    use serde_json::json;
    use std::{fs, path::PathBuf};

    #[test]
    fn diagnostic_payload_redacts_secret_values_and_raw_output() {
        let dir = unique_test_dir("redaction");
        let state_file = dir.join("node.json");
        let journal_dir = dir.join("task-journal");
        fs::create_dir_all(&journal_dir).expect("journal dir");
        fs::write(
            journal_dir.join("registry.json"),
            serde_json::to_string_pretty(&json!({
                "req-1": {
                    "req_id": "req-1",
                    "cli_name": "server-runtime",
                    "route": "route_c_server_runtime",
                    "run_handle_id": "req-1",
                    "cwd": "D:/demo",
                    "runtime_permission": "project_write",
                    "status": "running",
                    "os_pid": 42,
                    "started_at_ms": 1,
                    "updated_at_ms": 2,
                    "codex_session_id": "secret-session-id"
                }
            }))
            .unwrap(),
        )
        .expect("registry");
        fs::write(
            journal_dir.join("events.jsonl"),
            "{\"type\":\"started\",\"req_id\":\"req-1\"}\n{\"type\":\"cli_chunk\",\"text\":\"secret output\"}\n",
        )
        .expect("events");
        let logs_dir = dir.join("logs");
        fs::create_dir_all(&logs_dir).expect("logs dir");
        fs::write(
            logs_dir.join("client-maintenance.jsonl"),
            "{\"at_ms\":3,\"action\":\"open_target\",\"ok\":false,\"detail\":\"secret maintenance detail\"}\n",
        )
        .expect("maintenance log");
        let paths = DiagnosticPaths::from_state_file(state_file);
        let payload = diagnostic_payload(&paths);
        let text = serde_json::to_string(&payload).expect("payload json");

        assert!(payload["privacy"]["raw_cli_output_exported"] == false);
        assert!(payload["privacy"]["maintenance_log_contents_exported"] == false);
        assert!(payload["privacy"]["maintenance_log_details_exported"] == false);
        assert!(payload["privacy"]["api_key_values_exported"] == false);
        assert!(payload["paths"]["logs_dir"].as_str().is_some());
        assert!(payload
            .get("paths")
            .unwrap()
            .get("launcher_logs_dir")
            .is_some());
        assert!(payload
            .get("paths")
            .unwrap()
            .get("launcher_log_file")
            .is_some());
        assert!(payload["files"]["maintenance_log"]["path"]
            .as_str()
            .unwrap()
            .contains("client-maintenance.jsonl"));
        assert!(payload["files"].get("launcher_log").is_some());
        assert!(text.contains("\"codex_session_present\":true"));
        assert!(!text.contains("secret-session-id"));
        assert!(!text.contains("secret output"));
        assert!(!text.contains("secret maintenance detail"));
        assert_eq!(payload["logs"]["maintenance"]["line_count"], 1);
        assert_eq!(
            payload["logs"]["maintenance"]["actions"]["open_target"]["failed"],
            1
        );
        assert_eq!(payload["tasks"]["events"]["line_count"], 2);
        assert_eq!(payload["tasks"]["events"]["types"]["started"], 1);
        assert_eq!(payload["tasks"]["events"]["types"]["cli_chunk"], 1);

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-diagnostics-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
