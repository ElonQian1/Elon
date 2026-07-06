use axum::{http::StatusCode, Json};
use serde_json::json;
use std::path::Path;

use super::AgentRuntimeFreshness;

pub(super) fn inspect_agent_runtime_freshness(project_root: &Path) -> AgentRuntimeFreshness {
    let script = project_root.join("scripts").join("elon-agent.ps1");
    let script_path = script.to_string_lossy().to_string();
    let Ok(text) = std::fs::read_to_string(&script) else {
        return AgentRuntimeFreshness {
            status: "missing".to_string(),
            summary: "未生成项目内便携一龙入口 scripts\\elon-agent.ps1；不影响 Win 端节点内置开发能力，仅影响离线或无一龙客户端时在项目目录直接运行一龙 agent。"
                .to_string(),
            script_path,
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: false,
            has_command_budget: false,
            has_output_limit: false,
            max_run_commands_default: None,
        };
    };

    let has_command_budget =
        text.contains("MaxRunCommands") && text.contains("Use-AgentRunCommandBudget");
    let has_output_limit =
        text.contains("AgentCommandOutputMaxChars") && text.contains("Limit-AgentText");
    let max_run_commands_default = parse_max_run_commands_default(&text);
    if has_command_budget && has_output_limit {
        AgentRuntimeFreshness {
            status: "current".to_string(),
            summary: format!(
                "项目内便携一龙入口已包含命令预算和输出截断保护，默认每轮最多 {} 个 run_command；可作为离线或无一龙客户端时的高级入口。",
                max_run_commands_default.unwrap_or(8)
            ),
            script_path,
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: true,
            has_command_budget,
            has_output_limit,
            max_run_commands_default,
        }
    } else {
        AgentRuntimeFreshness {
            status: "stale".to_string(),
            summary: "项目内便携一龙入口 scripts\\elon-agent.ps1 是旧版模板，缺少 run_command 预算或输出截断保护；不影响 Win 端节点内置开发能力，需要离线或无客户端使用时再重新生成。".to_string(),
            script_path,
            runtime_scope: "project_portable_cli_entry",
            registration_required: false,
            has_elon_agent: true,
            has_command_budget,
            has_output_limit,
            max_run_commands_default,
        }
    }
}

pub(super) fn parse_max_run_commands_default(script: &str) -> Option<u32> {
    let marker = "[int]$MaxRunCommands =";
    let start = script.find(marker)? + marker.len();
    let digits = script[start..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

pub(super) fn clean_project_text(value: &str, max_chars: usize) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

pub(super) fn default_project_description(name: &str) -> String {
    format!("绑定到本 PC 节点的本地项目: {name}")
}

pub(super) fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("本地项目")
        .to_string()
}

pub(super) fn json_error(
    status: StatusCode,
    error: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": error.into(),
        })),
    )
}

#[cfg(windows)]
pub(super) fn pick_folder() -> anyhow::Result<Option<String>> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    let script = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = '选择要绑定到一龙 PC 节点的项目目录'
$dialog.ShowNewFolderButton = $false
$owner = New-Object System.Windows.Forms.Form
$owner.StartPosition = 'CenterScreen'
$owner.Width = 1
$owner.Height = 1
$owner.ShowInTaskbar = $false
$owner.TopMost = $true
$owner.Opacity = 0
try {
  $owner.Show()
  $owner.Activate()
  $result = $dialog.ShowDialog($owner)
  if ($result -eq [System.Windows.Forms.DialogResult]::OK) {
    Write-Output $dialog.SelectedPath
  }
} finally {
  $owner.Dispose()
}
"#;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        // PowerShell 仅用于系统文件夹选择器；隐藏并隔离控制台，避免用户看到黑窗。
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(if stderr.is_empty() {
            "PowerShell 文件夹选择器返回失败".to_string()
        } else {
            stderr
        });
    }
    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!selected.is_empty()).then_some(selected))
}

#[cfg(not(windows))]
pub(super) fn pick_folder() -> anyhow::Result<Option<String>> {
    anyhow::bail!("本机文件夹选择器目前仅支持 Windows 客户端");
}
