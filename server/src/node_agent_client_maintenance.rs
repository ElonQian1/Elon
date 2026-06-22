// server/src/node_agent_client_maintenance.rs

use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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
        Ok(path) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "opened": path_to_string(&path),
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

pub(crate) async fn update_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Update) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": "已开始后台检查更新；如有新版本，客户端会自动替换并重启。"
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

pub(crate) async fn uninstall_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Uninstall) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": "已安排卸载；本机节点会退出并清理安装目录。"
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
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
    }
    payload
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
        "task_journal" | "logs" => Ok(MaintenanceTarget {
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
    MaintenancePaths {
        task_journal_dir: state_file.with_file_name("task-journal"),
        state_file,
        config_dir,
    }
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
    use super::{install_dir_from_local_app_data, maintenance_target};
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
        assert!(maintenance_target("config_dir").is_ok());
        assert!(maintenance_target("state_file").is_ok());
        assert!(maintenance_target(r"C:\Windows").is_err());
    }
}
