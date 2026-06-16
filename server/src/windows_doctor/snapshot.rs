use serde_json::{json, Value};

use super::{
    command::{powershell_check, run_command, MAX_COMMAND_OUTPUT},
    now_ms,
    repair::allowed_repairs,
};

pub(super) fn collect_snapshot() -> Value {
    let mut checks = Vec::new();
    if cfg!(windows) {
        checks.push(run_command("cmd", &["/C", "ver"], MAX_COMMAND_OUTPUT));
        checks.push(run_command("ipconfig", &["/all"], MAX_COMMAND_OUTPUT));
        checks.push(run_command(
            "netsh",
            &["winhttp", "show", "proxy"],
            MAX_COMMAND_OUTPUT,
        ));
        checks.push(powershell_check(
            "Get-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings' | Select-Object ProxyEnable,ProxyServer,ProxyOverride | ConvertTo-Json -Compress",
        ));
        checks.push(powershell_check(
            "Get-DnsClientServerAddress -AddressFamily IPv4 | Select-Object InterfaceAlias,ServerAddresses | ConvertTo-Json -Depth 4 -Compress",
        ));
        checks.push(powershell_check(
            "Get-NetIPConfiguration | Select-Object InterfaceAlias,IPv4Address,IPv4DefaultGateway,DNSServer | ConvertTo-Json -Depth 5 -Compress",
        ));
        checks.push(powershell_check(
            "Get-NetAdapter | Select-Object Name,InterfaceDescription,Status,LinkSpeed,MacAddress | ConvertTo-Json -Depth 3 -Compress",
        ));
        checks.push(powershell_check(
            "Get-Service | Where-Object {$_.Name -in 'Dnscache','WinHttpAutoProxySvc','WlanSvc','Dhcp','NlaSvc'} | Select-Object Name,DisplayName,Status,StartType | ConvertTo-Json -Compress",
        ));
    }

    json!({
        "supported": cfg!(windows),
        "mode": "read_only_snapshot",
        "collectedAtMs": now_ms(),
        "computerName": std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).ok(),
        "userName": std::env::var("USERNAME").or_else(|_| std::env::var("USER")).ok(),
        "commands": checks,
        "allowedRepairs": allowed_repairs(),
        "note": if cfg!(windows) {
            "默认只读采集系统代理、网卡、DNS 和关键 Windows 服务状态；修复动作需要用户确认。"
        } else {
            "电脑医生当前只支持 Windows 节点。"
        },
    })
}
