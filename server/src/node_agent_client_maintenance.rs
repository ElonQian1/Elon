// server/src/node_agent_client_maintenance.rs

use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::process::{Command, Stdio};

#[derive(Deserialize)]
pub(crate) struct OpenTargetRequest {
    target: String,
}

pub(crate) async fn status_handler() -> Json<Value> {
    Json(status_payload())
}

pub(crate) async fn open_target_handler(
    Json(req): Json<OpenTargetRequest>,
) -> (StatusCode, Json<Value>) {
    match open_target(&req.target) {
        Ok(path) => {
            record_maintenance_event("open_target", true, &req.target);
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "opened": path_to_string(&path),
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("open_target", false, &format!("{}: {}", req.target, error));
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn update_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Update) {
        Ok(()) => {
            record_maintenance_event("update", true, "scheduled");
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已开始后台检查更新；如有新版本，客户端会自动替换并重启。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("update", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn uninstall_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Uninstall) {
        Ok(()) => {
            record_maintenance_event("uninstall", true, "scheduled");
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已安排卸载；本机节点会退出并清理安装目录。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("uninstall", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

fn status_payload() -> Value {
    let paths = maintenance_paths();
    let payload = json!({
        "ok": true,
        "platform": std::env::consts::OS,
        "supported": cfg!(windows),
        "version": env!("CARGO_PKG_VERSION"),
        "state_file": path_to_string(&paths.state_file),
        "config_dir": path_to_string(&paths.config_dir),
        "task_journal_dir": path_to_string(&paths.task_journal_dir),
        "logs_dir": path_to_string(&paths.logs_dir),
        "maintenance_log_file": path_to_string(&paths.maintenance_log_file),
        "diagnostics_dir": path_to_string(&paths.diagnostics_dir),
        "maintenance_targets": [
            { "target": "install_dir", "label": "安装目录", "purpose": "确认根目录只保留主程序、卸载程序和 _internal。" },
            { "target": "logs", "label": "运行日志", "purpose": "查看客户端维护、更新、卸载等本机运行日志。" },
            { "target": "task_journal", "label": "任务日志", "purpose": "查看本机任务生命周期 journal，不包含 prompt 或 API key。" },
            { "target": "diagnostics_dir", "label": "诊断目录", "purpose": "保存可发给客服或开发者的脱敏诊断文件。" },
            { "target": "config_dir", "label": "配置目录", "purpose": "查看本机节点凭证和运行配置所在目录。" }
        ],
        "client_care_summary": "普通用户日常只需要运行一龙PC节点.exe；需要移除时运行卸载一龙PC节点.exe。运行日志、任务记录、诊断、更新和卸载都集中在本面板。",
        "cli_session_bridge": crate::node_agent_cli_session_bridge::status_payload(),
    });

    with_install_status(payload)
}

fn with_install_status(mut payload: Value) -> Value {
    let install = crate::node_agent_client_install_status::status_payload();
    if let Some(object) = payload.as_object_mut() {
        object.insert("install".to_string(), install.clone());
        if let Some(install_object) = install.as_object() {
            for key in [
                "install_dir",
                "client_exe",
                "uninstall_exe",
                "internal_dir",
                "version_file",
                "installed",
                "running_from_install_dir",
                "installed_git_sha",
                "installed_package_version",
                "layout_status",
                "layout",
                "version_manifest",
            ] {
                if let Some(value) = install_object.get(key) {
                    object.insert(key.to_string(), value.clone());
                }
            }
        }
        object.insert(
            "maintenance_actions".to_string(),
            maintenance_actions(&install),
        );
    }
    payload
}

fn maintenance_actions(install: &Value) -> Value {
    let supported = install
        .get("supported")
        .and_then(Value::as_bool)
        .unwrap_or(cfg!(windows));
    let installed = install
        .get("installed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let layout_status = install
        .get("layout_status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let can_use_installed_client = supported && installed;

    Value::Array(vec![
        action(
            "open_install_dir",
            "open_target",
            "安装目录",
            "打开安装目录，确认用户只需要主程序、卸载程序和 _internal。",
            "install_dir",
            supported,
            "neutral",
            None,
        ),
        action(
            "open_client_logs",
            "open_target",
            "运行日志",
            "打开客户端维护日志目录，用于排查更新、卸载、打开目录等本机维护动作。",
            "logs",
            supported,
            "neutral",
            None,
        ),
        action(
            "open_task_journal",
            "open_target",
            "任务日志",
            "打开本机任务 journal 目录，用于查看任务生命周期和恢复记录。",
            "task_journal",
            supported,
            "neutral",
            None,
        ),
        action(
            "open_diagnostics_dir",
            "open_target",
            "诊断目录",
            "打开脱敏诊断文件保存目录。",
            "diagnostics_dir",
            supported,
            "neutral",
            None,
        ),
        action(
            "export_diagnostics",
            "export_diagnostics",
            "导出诊断",
            "生成一份脱敏 JSON，包含安装、环境和任务摘要，不包含 prompt/API key。",
            "",
            supported,
            "neutral",
            None,
        ),
        action(
            "check_update",
            "update",
            "检查更新",
            if layout_status == "clean" {
                "后台检查完整客户端包更新；有新版本会替换并重启。"
            } else {
                "后台检查更新，并尝试收敛旧脚本或额外文件造成的安装布局问题。"
            },
            "",
            can_use_installed_client,
            "primary",
            None,
        ),
        action(
            "uninstall_client",
            "uninstall",
            "卸载",
            "退出本机节点、移除自启动和 URL 协议，并清理安装目录。",
            "",
            can_use_installed_client,
            "danger",
            Some("确认卸载一龙 PC 节点客户端？卸载会退出本机节点并清理安装目录。"),
        ),
    ])
}

fn action(
    id: &str,
    kind: &str,
    label: &str,
    description: &str,
    target: &str,
    enabled: bool,
    tone: &str,
    confirmation: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "label": label,
        "description": description,
        "target": target,
        "enabled": enabled,
        "tone": tone,
        "confirmation": confirmation,
    })
}

fn open_target(raw_target: &str) -> Result<PathBuf, String> {
    let target = maintenance_target(raw_target)?;
    if target.ensure_dir {
        std::fs::create_dir_all(&target.path)
            .map_err(|error| format!("无法创建目录 {}: {error}", target.path.display()))?;
    }
    if target.must_exist && !target.path.exists() {
        return Err(format!("路径不存在: {}", target.path.display()));
    }
    open_path(&target.path, target.select_file)?;
    Ok(target.path)
}

fn maintenance_target(raw_target: &str) -> Result<MaintenanceTarget, String> {
    let paths = maintenance_paths();
    match raw_target.trim() {
        "logs" | "logs_dir" => Ok(MaintenanceTarget {
            path: paths.logs_dir,
            select_file: false,
            ensure_dir: true,
            must_exist: true,
        }),
        "maintenance_log" | "maintenance_log_file" => Ok(MaintenanceTarget {
            path: paths.maintenance_log_file,
            select_file: true,
            ensure_dir: false,
            must_exist: false,
        }),
        "task_journal" => Ok(MaintenanceTarget {
            path: paths.task_journal_dir,
            select_file: false,
            ensure_dir: true,
            must_exist: true,
        }),
        "config_dir" => Ok(MaintenanceTarget {
            path: paths.config_dir,
            select_file: false,
            ensure_dir: true,
            must_exist: true,
        }),
        "diagnostics" | "diagnostics_dir" => Ok(MaintenanceTarget {
            path: paths.diagnostics_dir,
            select_file: false,
            ensure_dir: true,
            must_exist: true,
        }),
        "config_file" | "state_file" => Ok(MaintenanceTarget {
            path: paths.state_file,
            select_file: true,
            ensure_dir: false,
            must_exist: false,
        }),
        "install_dir" => {
            #[cfg(windows)]
            {
                let installed = installed_paths()?;
                Ok(MaintenanceTarget {
                    path: installed.install_dir,
                    select_file: false,
                    ensure_dir: false,
                    must_exist: true,
                })
            }
            #[cfg(not(windows))]
            {
                Err("当前平台没有 Windows 客户端安装目录。".to_string())
            }
        }
        _ => Err("未知维护入口。".to_string()),
    }
}

fn open_path(path: &Path, select_file: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut command = Command::new("explorer.exe");
        if select_file && path.exists() {
            command.arg(format!("/select,{}", path.display()));
        } else if select_file {
            let parent = path
                .parent()
                .ok_or_else(|| format!("无法定位父目录: {}", path.display()))?;
            command.arg(parent);
        } else {
            command.arg(path);
        }
        apply_hidden_window(&mut command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法打开 {}: {error}", path.display()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (path, select_file);
        Err("当前平台不支持打开 Windows 文件夹。".to_string())
    }
}

fn spawn_client_action(action: ClientAction) -> Result<(), String> {
    #[cfg(windows)]
    {
        let installed = installed_paths()?;
        let (program, arg) = match action {
            ClientAction::Update => (&installed.client_exe, "--update"),
            ClientAction::Uninstall => (&installed.uninstall_exe, "--uninstall"),
        };
        if !program.exists() {
            return Err(format!("缺少客户端程序: {}", program.display()));
        }
        let mut command = Command::new(program);
        command.arg(arg).current_dir(&installed.install_dir);
        apply_hidden_window(&mut command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法启动 {}: {error}", program.display()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        Err("当前平台不支持 Windows 客户端维护动作。".to_string())
    }
}

fn maintenance_paths() -> MaintenancePaths {
    let state_file = crate::state_path();
    let config_dir = state_file
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let logs_dir = config_dir.join("logs");
    MaintenancePaths {
        diagnostics_dir: config_dir.join("diagnostics"),
        task_journal_dir: state_file.with_file_name("task-journal"),
        maintenance_log_file: logs_dir.join("client-maintenance.jsonl"),
        logs_dir,
        state_file,
        config_dir,
    }
}

fn record_maintenance_event(action: &str, ok: bool, detail: &str) {
    let paths = maintenance_paths();
    let _ = fs::create_dir_all(&paths.logs_dir);
    let line = json!({
        "at_ms": now_ms(),
        "action": action,
        "ok": ok,
        "detail": truncate_chars(detail, 500)
    })
    .to_string();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.maintenance_log_file)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(windows)]
fn installed_paths() -> Result<InstalledPaths, String> {
    let install_dir =
        install_dir_from_local_app_data(std::env::var("LOCALAPPDATA").ok().as_deref())
            .ok_or_else(|| "无法读取 LOCALAPPDATA，不能确定安装目录。".to_string())?;
    Ok(InstalledPaths {
        client_exe: install_dir.join(crate::node_client_launcher::CLIENT_EXE_NAME),
        uninstall_exe: install_dir.join(crate::node_client_launcher::UNINSTALL_EXE_NAME),
        install_dir,
    })
}

#[cfg(any(windows, test))]
fn install_dir_from_local_app_data(local_app_data: Option<&str>) -> Option<PathBuf> {
    local_app_data
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| PathBuf::from(value).join("ElonNode"))
}

#[cfg(windows)]
fn apply_hidden_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn error_response(status: StatusCode, error: String) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "ok": false, "error": error })))
}

struct MaintenancePaths {
    state_file: PathBuf,
    config_dir: PathBuf,
    task_journal_dir: PathBuf,
    logs_dir: PathBuf,
    maintenance_log_file: PathBuf,
    diagnostics_dir: PathBuf,
}

struct MaintenanceTarget {
    path: PathBuf,
    select_file: bool,
    ensure_dir: bool,
    must_exist: bool,
}

#[cfg(windows)]
struct InstalledPaths {
    install_dir: PathBuf,
    client_exe: PathBuf,
    uninstall_exe: PathBuf,
}

enum ClientAction {
    Update,
    Uninstall,
}

#[cfg(test)]
mod tests {
    use super::{
        install_dir_from_local_app_data, maintenance_target, status_payload, truncate_chars,
    };
    use std::path::PathBuf;

    #[test]
    fn install_dir_is_under_local_app_data_elon_node() {
        assert_eq!(
            install_dir_from_local_app_data(Some(r"C:\Users\ELon\AppData\Local")).unwrap(),
            PathBuf::from(r"C:\Users\ELon\AppData\Local").join("ElonNode")
        );
        assert!(install_dir_from_local_app_data(Some(" ")).is_none());
        assert!(install_dir_from_local_app_data(None).is_none());
    }

    #[test]
    fn only_fixed_open_targets_are_supported() {
        assert!(maintenance_target("task_journal").is_ok());
        assert!(maintenance_target("logs").is_ok());
        assert!(maintenance_target("maintenance_log").is_ok());
        assert!(maintenance_target("diagnostics_dir").is_ok());
        assert!(maintenance_target("config_dir").is_ok());
        assert!(maintenance_target("state_file").is_ok());
        assert!(maintenance_target(r"C:\Windows").is_err());
    }

    #[test]
    fn status_exposes_productized_maintenance_targets() {
        let status = status_payload();
        assert!(status["logs_dir"].as_str().is_some());
        assert!(status["maintenance_log_file"].as_str().is_some());
        assert!(status["diagnostics_dir"].as_str().is_some());
        assert!(status["maintenance_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| target["target"].as_str() == Some("logs")));
        assert!(status["maintenance_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| target["target"].as_str() == Some("diagnostics_dir")));
        assert!(status["client_care_summary"]
            .as_str()
            .unwrap()
            .contains("运行日志"));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["id"].as_str() == Some("open_client_logs")
                    && action["target"].as_str() == Some("logs")
            }));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["kind"].as_str() == Some("export_diagnostics")));
        assert!(status["maintenance_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["kind"].as_str() == Some("uninstall")
                    && action["confirmation"].as_str().is_some()
            }));
    }

    #[test]
    fn maintenance_log_details_are_bounded() {
        let long = "x".repeat(700);
        assert_eq!(truncate_chars(&long, 500).chars().count(), 500);
        assert_eq!(truncate_chars("  ok  ", 500), "ok");
    }
}
