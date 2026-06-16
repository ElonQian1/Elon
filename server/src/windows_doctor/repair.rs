use serde_json::{json, Value};

use super::{
    command::{run_command, MAX_COMMAND_OUTPUT},
    normalize_text,
};

pub(super) struct RepairPlan {
    pub(super) action: &'static str,
    pub(super) title: &'static str,
    pub(super) risk: &'static str,
    pub(super) impact: &'static str,
    program: String,
    args: Vec<String>,
}

pub(super) fn repair_plan(action: &str, adapter_name: Option<&str>) -> Option<RepairPlan> {
    match action {
        "flush_dns" => Some(RepairPlan {
            action: "flush_dns",
            title: "清空 DNS 缓存",
            risk: "low",
            impact: "会清空本机 DNS 解析缓存，不修改网络配置。",
            program: "ipconfig".to_string(),
            args: vec!["/flushdns".to_string()],
        }),
        "reset_winhttp_proxy" => Some(RepairPlan {
            action: "reset_winhttp_proxy",
            title: "重置 WinHTTP 代理",
            risk: "medium",
            impact: "会清除系统 WinHTTP 代理，可能影响后台服务或命令行工具联网。",
            program: "netsh".to_string(),
            args: vec!["winhttp".to_string(), "reset".to_string(), "proxy".to_string()],
        }),
        "clear_user_proxy" => Some(RepairPlan {
            action: "clear_user_proxy",
            title: "关闭当前用户代理",
            risk: "medium",
            impact: "会关闭当前 Windows 用户的系统代理开关，并移除 ProxyServer 注册表值。",
            program: "powershell".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                "Set-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings' -Name ProxyEnable -Type DWord -Value 0; Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings' -Name ProxyServer -ErrorAction SilentlyContinue"
                    .to_string(),
            ],
        }),
        "restart_adapter" => {
            let adapter = normalize_text(adapter_name.unwrap_or_default(), 120);
            if adapter.is_empty() {
                return None;
            }
            Some(RepairPlan {
                action: "restart_adapter",
                title: "重启指定网卡",
                risk: "high",
                impact: "会短暂断开指定网卡连接，通常需要管理员权限。",
                program: "powershell".to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-ExecutionPolicy".to_string(),
                    "Bypass".to_string(),
                    "-Command".to_string(),
                    format!("Restart-NetAdapter -Name {} -Confirm:$false", ps_quote(&adapter)),
                ],
            })
        }
        _ => None,
    }
}

pub(super) fn execute_repair(plan: &RepairPlan) -> Value {
    run_command(&plan.program, &plan.args, MAX_COMMAND_OUTPUT)
}

pub(super) fn allowed_repairs() -> Value {
    json!([
        {
            "action": "flush_dns",
            "title": "清空 DNS 缓存",
            "risk": "low",
            "requiresConfirm": true
        },
        {
            "action": "reset_winhttp_proxy",
            "title": "重置 WinHTTP 代理",
            "risk": "medium",
            "requiresConfirm": true
        },
        {
            "action": "clear_user_proxy",
            "title": "关闭当前用户代理",
            "risk": "medium",
            "requiresConfirm": true
        },
        {
            "action": "restart_adapter",
            "title": "重启指定网卡",
            "risk": "high",
            "requiresConfirm": true,
            "requiresAdapterName": true
        }
    ])
}

fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_adapter_requires_adapter_name() {
        assert!(repair_plan("restart_adapter", None).is_none());
        assert!(repair_plan("restart_adapter", Some("Wi-Fi")).is_some());
    }
}
