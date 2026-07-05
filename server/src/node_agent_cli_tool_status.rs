use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use super::{candidate_tool_paths, codex_tool_catalog, path_to_string, InstallPolicy, ToolSpec};

const TOOL_VERSION_TIMEOUT: Duration = Duration::from_millis(700);

pub(super) fn codex_toolbox_status(codex_program: Option<&Path>) -> Value {
    let discovered_codex = codex_program
        .map(Path::to_path_buf)
        .or_else(|| elon_pc_dev_runtime::command_path("codex"));
    let codex_program = discovered_codex.as_deref();
    let tools = codex_tool_catalog()
        .iter()
        .map(|spec| tool_status_payload(spec, codex_program))
        .collect::<Vec<_>>();
    let rg_ready = tools.iter().any(|tool| {
        tool.get("id").and_then(Value::as_str) == Some("rg")
            && tool.get("status").and_then(Value::as_str) == Some("ready")
    });
    json!({
        "ok": true,
        "codex_program": codex_program.map(path_to_string),
        "path_scope": "codex_child_process_only",
        "summary": if rg_ready {
            "rg 已可注入 Codex CLI 子进程。"
        } else {
            "rg 暂未就绪；Codex CLI 会退回较慢的 PowerShell 搜索。"
        },
        "tools": tools,
    })
}

fn tool_status_payload(spec: &'static ToolSpec, codex_program: Option<&Path>) -> Value {
    let candidates = candidate_tool_paths(spec, codex_program);
    let candidate_values = candidates
        .iter()
        .map(|path| {
            json!({
                "path": path_to_string(path),
                "source": classify_tool_source(path, spec),
            })
        })
        .collect::<Vec<_>>();
    let Some(path) = candidates.first() else {
        return json!({
            "id": spec.id,
            "name": spec.primary_bin,
            "aliases": spec.aliases,
            "tier": spec.tier.as_str(),
            "install_policy": spec.install_policy.as_str(),
            "env_path_var": spec.env_path_var,
            "managed_dir": spec.managed_dir,
            "installed": false,
            "runnable": false,
            "status": "missing",
            "source": "missing",
            "path": Value::Null,
            "version": Value::Null,
            "candidate_count": 0,
            "candidates": candidate_values,
            "will_inject": false,
            "repair_action": repair_action(spec),
            "detail": missing_detail(spec),
        });
    };
    let version = probe_tool_version(spec, path);
    json!({
        "id": spec.id,
        "name": spec.primary_bin,
        "aliases": spec.aliases,
        "tier": spec.tier.as_str(),
        "install_policy": spec.install_policy.as_str(),
        "env_path_var": spec.env_path_var,
        "managed_dir": spec.managed_dir,
        "installed": true,
        "runnable": version.runnable,
        "status": if version.runnable { "ready" } else { "not_runnable" },
        "source": classify_tool_source(path, spec),
        "path": path_to_string(path),
        "version": version.summary,
        "reason": version.reason,
        "candidate_count": candidates.len(),
        "candidates": candidate_values,
        "will_inject": version.runnable,
        "repair_action": repair_action(spec),
        "detail": if version.runnable {
            format!("{} 已找到，会临时注入 Codex CLI 子进程。", spec.primary_bin)
        } else {
            format!("{} 已找到，但版本检测无法运行。", spec.primary_bin)
        },
    })
}

fn missing_detail(spec: &ToolSpec) -> String {
    match spec.install_policy {
        InstallPolicy::AutoSmall => {
            format!(
                "{} 未找到；点击 Codex 环境修复会自动补齐。",
                spec.primary_bin
            )
        }
        InstallPolicy::ManualRepair => {
            format!("{} 未找到；当前只在用户已安装时暴露。", spec.primary_bin)
        }
        InstallPolicy::NeverAuto => {
            format!("{} 未找到；该工具不会自动安装。", spec.primary_bin)
        }
    }
}

fn repair_action(spec: &ToolSpec) -> &'static str {
    match spec.install_policy {
        InstallPolicy::AutoSmall => "install_env_codex",
        InstallPolicy::ManualRepair => "manual_repair",
        InstallPolicy::NeverAuto => "none",
    }
}

struct VersionProbe {
    runnable: bool,
    summary: Option<String>,
    reason: Option<String>,
}

fn probe_tool_version(spec: &ToolSpec, path: &Path) -> VersionProbe {
    let mut command = elon_pc_dev_runtime::command_from_path(path);
    command
        .args(spec.version_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    apply_hidden_window(&mut command);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return VersionProbe {
                runnable: false,
                summary: None,
                reason: Some(format!("spawn_failed: {error}")),
            };
        }
    };
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait_with_output() {
                    Ok(output) => VersionProbe {
                        runnable: output.status.success(),
                        summary: first_output_line(&output.stdout, &output.stderr),
                        reason: (!output.status.success()).then(|| "exit_failed".to_string()),
                    },
                    Err(error) => VersionProbe {
                        runnable: false,
                        summary: None,
                        reason: Some(format!("output_failed: {error}")),
                    },
                };
            }
            Ok(None) if started_at.elapsed() >= TOOL_VERSION_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return VersionProbe {
                    runnable: false,
                    summary: None,
                    reason: Some("timeout".to_string()),
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return VersionProbe {
                    runnable: false,
                    summary: None,
                    reason: Some(format!("run_failed: {error}")),
                };
            }
        }
    }
}

fn classify_tool_source(path: &Path, spec: &ToolSpec) -> &'static str {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if lower.contains("\\elonnode\\tools\\") || lower.contains("/elonnode/tools/") {
        return "elon_managed";
    }
    if is_codex_runtime_path(path) {
        return "codex_desktop";
    }
    if lower.contains("\\.cargo\\bin\\") || lower.contains("/.cargo/bin/") {
        return "cargo";
    }
    if lower.contains("\\scoop\\shims\\") || lower.contains("/scoop/shims/") {
        return "scoop";
    }
    if lower.contains("\\chocolatey\\bin\\") || lower.contains("/chocolatey/bin/") {
        return "chocolatey";
    }
    if lower.contains(&format!("\\{}\\", spec.managed_dir.to_ascii_lowercase()))
        || lower.contains(&format!("/{}/", spec.managed_dir.to_ascii_lowercase()))
    {
        return "program_files";
    }
    "path"
}

fn is_codex_runtime_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    (lower.contains("\\openai\\codex\\bin\\") || lower.contains("/openai/codex/bin/"))
        || (lower.contains("\\windowsapps\\")
            && lower.contains("\\openai.codex_")
            && lower.contains("\\app\\resources\\"))
}

fn first_output_line(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    [stdout, stderr].into_iter().find_map(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(180).collect())
    })
}

#[cfg(windows)]
fn apply_hidden_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(windows))]
fn apply_hidden_window(_command: &mut std::process::Command) {}
