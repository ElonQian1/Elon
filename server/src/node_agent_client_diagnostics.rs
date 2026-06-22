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
            "token_values_exported": false,
            "api_key_values_exported": false
        },
        "runtime": runtime_summary(),
        "paths": {
            "state_file": path_to_string(&paths.state_file),
            "config_dir": path_to_string(&paths.config_dir),
            "task_journal_dir": path_to_string(&paths.task_journal_dir),
            "diagnostics_dir": path_to_string(&paths.diagnostics_dir)
        },
        "files": diagnostic_files(paths),
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

#[cfg(windows)]
fn install_summary() -> Value {
    let install_dir = std::env::var("LOCALAPPDATA")
        .ok()
        .map(|value| PathBuf::from(value).join("ElonNode"));
    match install_dir {
        Some(install_dir) => json!({
            "install_dir": path_to_string(&install_dir),
            "client_exe": file_meta(&install_dir.join(crate::node_client_launcher::CLIENT_EXE_NAME)),
            "uninstall_exe": file_meta(&install_dir.join(crate::node_client_launcher::UNINSTALL_EXE_NAME)),
            "internal_dir": file_meta(&install_dir.join(crate::node_client_launcher::INTERNAL_DIR_NAME)),
            "version_manifest": safe_json_file(&install_dir.join(crate::node_client_launcher::INTERNAL_DIR_NAME).join("node-agent-version.json")),
            "running_from_install_dir": std::env::current_exe()
                .map(|path| path.starts_with(&install_dir))
                .unwrap_or(false),
        }),
        None => json!({
            "error": "LOCALAPPDATA is not configured",
        }),
    }
}

#[cfg(not(windows))]
fn install_summary() -> Value {
    json!({
        "supported": false,
        "reason": "Windows client install status is only available on Windows."
    })
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
        let diagnostics_dir = config_dir.join("diagnostics");
        Self {
            state_file,
            config_dir,
            task_journal_dir,
            diagnostics_dir,
        }
    }
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
        let paths = DiagnosticPaths::from_state_file(state_file);
        let payload = diagnostic_payload(&paths);
        let text = serde_json::to_string(&payload).expect("payload json");

        assert!(payload["privacy"]["raw_cli_output_exported"] == false);
        assert!(payload["privacy"]["api_key_values_exported"] == false);
        assert!(text.contains("\"codex_session_present\":true"));
        assert!(!text.contains("secret-session-id"));
        assert!(!text.contains("secret output"));
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
