use serde_json::{json, Value};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::{Command, Stdio};
use super::*;

pub(super) fn status_payload() -> Value {
    let paths = maintenance_paths();
    let recent_events = recent_maintenance_events(&paths.maintenance_log_file, 5);
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
        "launcher_logs_dir": optional_path_to_value(paths.launcher_logs_dir.as_deref()),
        "launcher_log_file": optional_path_to_value(paths.launcher_log_file.as_deref()),
        "diagnostics_dir": path_to_string(&paths.diagnostics_dir),
        "maintenance_recent_events": recent_events,
        "maintenance_targets": [
            { "target": "install_dir", "label": "安装目录", "purpose": "确认根目录只保留主程序、卸载程序和 _internal。" },
            { "target": "logs", "label": "运行日志", "purpose": "查看客户端维护、更新、卸载等本机运行日志。" },
            { "target": "launcher_logs", "label": "启动器日志", "purpose": "查看双击启动、安装、自动更新和卸载入口日志。" },
            { "target": "task_journal", "label": "任务日志", "purpose": "查看本机任务生命周期 journal，不包含 prompt 或 API key。" },
            { "target": "diagnostics_dir", "label": "诊断目录", "purpose": "保存可发给客服或开发者的脱敏诊断文件。" },
            { "target": "config_dir", "label": "配置目录", "purpose": "查看本机节点凭证和运行配置所在目录。" }
        ],
        "client_care_summary": "普通用户日常只需要运行一龙开发平台.exe；需要移除时运行卸载一龙开发平台.exe。运行日志、任务记录、诊断、更新和卸载都集中在本面板。",
        "autostart": autostart_status_payload(),
        "cli_session_bridge": crate::node_agent_cli_session_bridge::status_payload(),
    });

    with_install_status(payload)
}

pub(super) fn autostart_status_payload() -> Value {
    #[cfg(windows)]
    {
        let installed = installed_paths();
        let (actual_command, source, legacy_detected) = match query_autostart_info() {
            Ok(info) => (info.command, info.source, info.legacy_detected),
            Err(error) => (None, format!("query_error:{error}"), false),
        };
        let expected_command = installed
            .as_ref()
            .ok()
            .map(|paths| format!("\"{}\" --watchdog", paths.client_exe.display()));
        let expected_path = installed
            .as_ref()
            .ok()
            .map(|paths| paths.client_exe.to_string_lossy().to_string());
        let enabled = match (&actual_command, &expected_path) {
            (Some(actual), Some(expected)) => command_targets_path(actual, expected),
            (Some(_), None) => true,
            _ => false,
        };
        let summary = if enabled {
            if source == "scheduled_task" {
                "开机登录后会通过当前用户计划任务启动后台守护，并自动恢复本机节点。"
            } else if source == "startup_shortcut" {
                "开机登录后会通过当前用户启动文件夹快捷方式启动后台守护，并自动恢复本机节点。"
            } else {
                "检测到旧版开机自启动；修复或更新后会优先迁移为当前用户计划任务，权限不足时降级为启动文件夹快捷方式。"
            }
        } else {
            "未开启开机自动守护；开启后无需每次手动启动 Win 端。"
        };
        json!({
            "supported": true,
            "enabled": enabled,
            "source": source,
            "strategy": "current_user_scheduled_task_or_startup_shortcut",
            "task_name": crate::node_client_launcher::AUTOSTART_TASK_NAME,
            "startup_shortcut_name": crate::node_client_launcher::AUTOSTART_STARTUP_SHORTCUT_NAME,
            "run_value_name": crate::node_client_launcher::AUTOSTART_RUN_VALUE_NAME,
            "legacy_detected": legacy_detected,
            "expected_command": expected_command,
            "actual_command": actual_command,
            "install_error": installed.err().map(|error| error.to_string()),
            "summary": summary,
        })
    }
    #[cfg(not(windows))]
    {
        json!({
            "supported": false,
            "enabled": false,
            "source": "unsupported",
            "strategy": "current_user_scheduled_task_or_startup_shortcut",
            "task_name": "ElonNodeAgent",
            "startup_shortcut_name": "一龙开发平台开机守护.lnk",
            "run_value_name": "ElonNodeAgent",
            "legacy_detected": false,
            "expected_command": Value::Null,
            "actual_command": Value::Null,
            "install_error": "当前平台不支持 Windows 开机自启动设置。",
            "summary": "请在安装 Win 端的一龙开发平台电脑上配置开机自启动。",
        })
    }
}

pub(super) fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let installed = installed_paths()?;
        crate::node_client_launcher::set_autostart_enabled(&installed.install_dir, enabled)
            .map_err(|error| format!("开机自启动设置失败: {error:#}"))
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("当前平台不支持 Windows 开机自启动设置。".to_string())
    }
}

pub(super) fn with_install_status(mut payload: Value) -> Value {
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
                "product_status",
                "version_manifest",
            ] {
                if let Some(value) = install_object.get(key) {
                    object.insert(key.to_string(), value.clone());
                }
            }
        }
        let actions = maintenance_actions(&install);
        let primary = primary_maintenance_action(&actions);
        let recent_events = object
            .get("maintenance_recent_events")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));
        object.insert("primary_maintenance_action".to_string(), primary.clone());
        object.insert(
            "maintenance_overview".to_string(),
            maintenance_overview(&install, &primary, &recent_events),
        );
        object.insert("maintenance_actions".to_string(), actions);
    }
    payload
}


pub(super) fn open_target(raw_target: &str) -> Result<PathBuf, String> {
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

pub(super) fn maintenance_target(raw_target: &str) -> Result<MaintenanceTarget, String> {
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
        "launcher_logs" | "launcher_logs_dir" => {
            let path = paths
                .launcher_logs_dir
                .ok_or_else(|| "无法定位客户端启动器日志目录。".to_string())?;
            Ok(MaintenanceTarget {
                path,
                select_file: false,
                ensure_dir: true,
                must_exist: true,
            })
        }
        "launcher_log" | "launcher_log_file" => {
            let path = paths
                .launcher_log_file
                .ok_or_else(|| "无法定位客户端启动器日志文件。".to_string())?;
            Ok(MaintenanceTarget {
                path,
                select_file: true,
                ensure_dir: false,
                must_exist: false,
            })
        }
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

pub(super) fn open_path(path: &Path, select_file: bool) -> Result<(), String> {
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
