use super::*;
use serde_json::{json, Value};

pub(super) fn maintenance_actions(install: &Value) -> Value {
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

pub(super) fn primary_maintenance_action(actions: &Value) -> Value {
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

pub(super) fn maintenance_overview(
    install: &Value,
    primary_action: &Value,
    recent_events: &Value,
) -> Value {
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

pub(super) fn primary_action_recommendation(
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

pub(super) fn action(
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
