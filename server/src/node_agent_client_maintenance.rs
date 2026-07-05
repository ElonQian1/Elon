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

#[derive(Deserialize)]
pub(crate) struct AutostartSetRequest {
    enabled: bool,
}

pub(crate) async fn status_handler() -> Json<Value> {
    Json(status_payload())
}

pub(crate) async fn autostart_status_handler() -> Json<Value> {
    Json(autostart_status_payload())
}

pub(crate) async fn autostart_set_handler(
    Json(req): Json<AutostartSetRequest>,
) -> (StatusCode, Json<Value>) {
    match set_autostart(req.enabled) {
        Ok(()) => {
            record_maintenance_event(
                "autostart",
                true,
                if req.enabled { "enabled" } else { "disabled" },
            );
            let mut payload = autostart_status_payload();
            if let Some(object) = payload.as_object_mut() {
                object.insert("ok".to_string(), Value::Bool(true));
                object.insert(
                    "message".to_string(),
                    Value::String(
                        if req.enabled {
                            "已开启开机自动守护。"
                        } else {
                            "已关闭开机自动守护。"
                        }
                        .to_string(),
                    ),
                );
            }
            (StatusCode::OK, Json(payload))
        }
        Err(error) => {
            record_maintenance_event("autostart", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
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
                    "message": "Win 端正在更新升级；如需替换重启，通信临时中断，会自动恢复。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("update", false, &error);
            error_response(StatusCode::BAD_REQUEST, error)
        }
    }
}

pub(crate) async fn repair_handler() -> (StatusCode, Json<Value>) {
    match spawn_client_action(ClientAction::Repair) {
        Ok(()) => {
            record_maintenance_event("repair", true, "scheduled");
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已开始后台修复客户端入口；会重新创建主程序、卸载程序、开始菜单和网页唤起协议。已开启的开机守护会保留并迁移为当前用户计划任务。"
                })),
            )
        }
        Err(error) => {
            record_maintenance_event("repair", false, &error);
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

fn autostart_status_payload() -> Value {
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
            } else {
                "检测到旧版开机自启动；修复或更新后会迁移为当前用户计划任务。"
            }
        } else {
            "未开启开机自动守护；开启后无需每次手动启动 Win 端。"
        };
        json!({
            "supported": true,
            "enabled": enabled,
            "source": source,
            "strategy": "current_user_scheduled_task",
            "task_name": crate::node_client_launcher::AUTOSTART_TASK_NAME,
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
            "strategy": "current_user_scheduled_task",
            "task_name": "ElonNodeAgent",
            "run_value_name": "ElonNodeAgent",
            "legacy_detected": false,
            "expected_command": Value::Null,
            "actual_command": Value::Null,
            "install_error": "当前平台不支持 Windows 开机自启动设置。",
            "summary": "请在安装 Win 端的一龙开发平台电脑上配置开机自启动。",
        })
    }
}

fn set_autostart(enabled: bool) -> Result<(), String> {
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
    let product_status = install
        .get("product_status")
        .and_then(Value::as_object)
        .and_then(|object| object.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let repair_tone = if matches!(
        product_status,
        "needs_repair" | "repair_recommended" | "cleanup_recommended"
    ) {
        "primary"
    } else {
        "neutral"
    };
    let can_use_installed_client = supported && installed;

    let mut actions = vec![
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
            "open_launcher_logs",
            "open_target",
            "启动器日志",
            "打开双击启动、安装、自动更新和卸载入口日志目录。",
            "launcher_logs",
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
            "open_config_dir",
            "open_target",
            "配置目录",
            "打开本机节点凭证和运行配置所在目录。",
            "config_dir",
            supported,
            "neutral",
            None,
        ),
        action(
            "open_state_file",
            "open_target",
            "配置文件",
            "定位本机节点状态配置文件。",
            "state_file",
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
            "repair_client",
            "repair",
            "修复客户端入口",
            "重新创建主程序、卸载程序、开始菜单和网页唤起协议；只保留已开启的开机守护。",
            "",
            supported,
            repair_tone,
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
    ];
    let recommendation =
        primary_action_recommendation(supported, installed, layout_status, product_status);
    for action in &mut actions {
        if action.get("id").and_then(Value::as_str) == Some(recommendation.action_id) {
            if let Some(object) = action.as_object_mut() {
                object.insert("recommended".to_string(), Value::Bool(true));
                object.insert(
                    "recommendation".to_string(),
                    Value::String(recommendation.reason.to_string()),
                );
            }
            break;
        }
    }
    Value::Array(actions)
}

fn primary_maintenance_action(actions: &Value) -> Value {
    actions
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|action| action.get("recommended").and_then(Value::as_bool) == Some(true))
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn maintenance_overview(install: &Value, primary_action: &Value, recent_events: &Value) -> Value {
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
    let product_status = install
        .get("product_status")
        .and_then(Value::as_object)
        .and_then(|object| object.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let product_summary = install
        .get("product_status")
        .and_then(Value::as_object)
        .and_then(|object| object.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let primary_action_id = primary_action
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let primary_action_label = primary_action
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("");
    let primary_recommendation = primary_action
        .get("recommendation")
        .and_then(Value::as_str)
        .unwrap_or("");
    let recent_failure_count = recent_events
        .as_array()
        .map(|events| {
            events
                .iter()
                .filter(|event| event.get("ok").and_then(Value::as_bool) == Some(false))
                .count()
        })
        .unwrap_or(0);
    let latest_failure_action = recent_events.as_array().and_then(|events| {
        events
            .iter()
            .find(|event| event.get("ok").and_then(Value::as_bool) == Some(false))
            .and_then(|event| event.get("action"))
            .and_then(Value::as_str)
    });
    let needs_repair = matches!(
        product_status,
        "needs_repair" | "repair_recommended" | "cleanup_recommended"
    ) || !matches!(layout_status, "clean" | "unknown");

    let (status, severity, title, detail) = if !supported {
        (
            "unsupported",
            "warning",
            "当前平台不支持 Win 客户端维护",
            primary_recommendation,
        )
    } else if recent_failure_count > 0 {
        (
            "attention",
            "warning",
            "最近维护动作失败",
            "最近维护日志里有失败记录，建议先导出诊断或执行首要建议。",
        )
    } else if !installed || needs_repair {
        (
            "attention",
            "warning",
            "建议修复客户端入口",
            if primary_recommendation.is_empty() {
                product_summary
            } else {
                primary_recommendation
            },
        )
    } else {
        (
            "ready",
            "ok",
            "Win 客户端入口正常",
            if product_summary.is_empty() {
                primary_recommendation
            } else {
                product_summary
            },
        )
    };

    json!({
        "status": status,
        "severity": severity,
        "title": title,
        "detail": detail,
        "primary_action_id": primary_action_id,
        "primary_action_label": primary_action_label,
        "recent_failure_count": recent_failure_count,
        "latest_failure_action": latest_failure_action,
        "safe_to_share_diagnostics": true,
    })
}

struct MaintenanceRecommendation {
    action_id: &'static str,
    reason: &'static str,
}

fn primary_action_recommendation(
    supported: bool,
    installed: bool,
    layout_status: &str,
    product_status: &str,
) -> MaintenanceRecommendation {
    if !supported {
        return MaintenanceRecommendation {
            action_id: "repair_client",
            reason: "当前平台不支持 Windows 客户端维护；请在安装了 Win 客户端的电脑上操作。",
        };
    }
    if matches!(
        product_status,
        "needs_repair" | "repair_recommended" | "cleanup_recommended"
    ) || !matches!(layout_status, "clean" | "unknown")
    {
        return MaintenanceRecommendation {
            action_id: "repair_client",
            reason: "检测到客户端入口、开始菜单或安装目录需要收敛，建议先修复客户端入口。",
        };
    }
    if installed {
        return MaintenanceRecommendation {
            action_id: "check_update",
            reason: "客户端布局正常，建议检查是否有新的完整客户端包。",
        };
    }
    MaintenanceRecommendation {
        action_id: "repair_client",
        reason: "未检测到完整安装，建议先修复客户端入口。",
    }
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

/// 服务端推送 UpdateClient 消息时调用：优先走已安装的更新程序，否则自行下载替换。
/// 返回 Ok(message) 表示已安排更新，Err(reason) 表示无法自动更新。
pub(crate) mod maintenance_ops;
#[cfg(test)]
mod maintenance_test;

pub(crate) use self::maintenance_ops::*;
