use crate::{command_probe, workspace_root};
use homecli_proto::{DevToolchainStatus, NodeDevRuntimeProfile, NodeDevRuntimeToolContract};
use std::{
    path::PathBuf,
    process::{Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(3);
const VERSION_CHECK_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub fn collect_dev_runtime_profile(allowed_clis: &[String]) -> NodeDevRuntimeProfile {
    collect_dev_runtime_profile_with_server_runtime(allowed_clis, false)
}

pub fn collect_dev_runtime_profile_with_server_runtime(
    allowed_clis: &[String],
    authenticated_server_runtime: bool,
) -> NodeDevRuntimeProfile {
    let root = workspace_root();
    let workspace_root_path = Some(root.to_string_lossy().to_string());
    let (workspace_root_writable, root_issue) = check_workspace_root(&root);

    let git = command_status("git", &["--version"]);
    let codex = command_status("codex", &["--version"]);
    let claude = command_status("claude", &["--version"]);
    let gemini = command_status("gemini", &["--version"]);
    let copilot = command_status("copilot", &["--version"]);
    let java = command_status("java", &["-version"]);
    let gradle = command_status("gradle", &["--version"]);
    let rustc = command_status("rustc", &["--version"]);
    let cargo = command_status("cargo", &["--version"]);
    let node = command_status("node", &["--version"]);
    let npm = command_status("npm", &["--version"]);
    let android_sdk = android_sdk_status();

    let git_ready = git.available;
    let android_ready = android_sdk.available && java.available;
    let rust_ready = rustc.available && cargo.available;
    let node_ready = node.available;
    let dev_env_ready = android_ready || rust_ready || node_ready;
    let route_a_ready = route_a_cli_ready(allowed_clis, &[&codex, &claude, &gemini, &copilot]);
    let api_runtime_ready = api_runtime_available();
    let server_runtime_ready = authenticated_server_runtime || server_runtime_available();
    let ai_cli_ready = route_a_ready || api_runtime_ready || server_runtime_ready;

    let mut issues = Vec::new();
    if let Some(issue) = root_issue {
        issues.push(issue);
    }
    if !git_ready {
        issues.push("未检测到 Git，无法初始化或克隆项目仓库".to_string());
    }
    if !dev_env_ready {
        issues.push(
            "未检测到 Android/Rust/Node 等构建工具链，项目可创建但可能暂不能本机编译".to_string(),
        );
    }

    if !ai_cli_ready {
        issues.push(
            "未检测到 Route A AI CLI、Route B API key/model 或 Route C 服务器 token；项目仍可创建，但本机 AI agent 入口暂不可用".to_string(),
        );
    }

    NodeDevRuntimeProfile {
        workspace_root_path,
        workspace_root_writable,
        git_ready,
        workspace_provision_ready: workspace_root_writable && git_ready,
        dev_env_ready,
        ai_cli_ready,
        route_a_ready,
        api_runtime_ready,
        server_runtime_ready,
        server_runtime_status: None,
        local_tool_contract: local_tool_contract(),
        toolchains: vec![
            git,
            java,
            gradle,
            android_sdk,
            rustc,
            cargo,
            node,
            npm,
            codex,
            claude,
            gemini,
            copilot,
        ],
        issues,
    }
}

fn local_tool_contract() -> NodeDevRuntimeToolContract {
    NodeDevRuntimeToolContract {
        routes: vec![
            "route_b_api_runtime".to_string(),
            "route_c_server_runtime".to_string(),
        ],
        supported_tools: vec![
            "list_dir".to_string(),
            "search_files".to_string(),
            "file_info".to_string(),
            "read_file".to_string(),
            "read_file_range".to_string(),
            "git_status".to_string(),
            "git_diff".to_string(),
            "git_log".to_string(),
            "download_router_status".to_string(),
            "download_router_doctor".to_string(),
            "download_router_configure".to_string(),
            "write_file".to_string(),
            "apply_patch".to_string(),
            "run_command".to_string(),
        ],
        approval_required_tools: vec![
            "write_file".to_string(),
            "apply_patch".to_string(),
            "download_router_configure".to_string(),
            "run_command".to_string(),
        ],
        path_policy: Some(
            "workspace_relative_no_git_no_symlink_escape_or_danger_full_access_absolute"
                .to_string(),
        ),
        command_policy: Some(
            "structured_project_command_allowlist_or_danger_full_access_shell".to_string(),
        ),
        audit_policy: Some("tool_events_redact_content_and_secrets".to_string()),
        recovery_policy: Some("task_journal_replay_without_original_tty_reattach".to_string()),
    }
}

fn route_a_cli_ready(allowed_clis: &[String], toolchains: &[&DevToolchainStatus]) -> bool {
    toolchains
        .iter()
        .any(|tool| tool.available && route_a_cli_allowed(allowed_clis, &tool.name))
}

fn route_a_cli_allowed(allowed_clis: &[String], cli_name: &str) -> bool {
    allowed_clis
        .iter()
        .any(|cli| cli.eq_ignore_ascii_case(cli_name))
}

fn api_runtime_available() -> bool {
    api_runtime_available_from_lookup(|key| std::env::var(key).ok())
}

fn api_runtime_available_from_lookup(lookup: impl Fn(&str) -> Option<String>) -> bool {
    let has_key = ["ELON_AGENT_API_KEY", "OPENAI_API_KEY", "HUNYUAN_API_KEY"]
        .iter()
        .any(|key| lookup(key).is_some_and(|value| !value.trim().is_empty()));
    let has_model = ["ELON_AGENT_MODEL", "OPENAI_MODEL", "HUNYUAN_MODEL"]
        .iter()
        .any(|key| lookup(key).is_some_and(|value| !value.trim().is_empty()));
    has_key && has_model
}

fn server_runtime_available() -> bool {
    let has_url = ["ELON_SERVER_URL", "ELON_AGENT_SERVER_URL"]
        .iter()
        .any(|key| env_present(key));
    let has_token = [
        "ELON_SERVER_TOKEN",
        "ELON_AGENT_SERVER_TOKEN",
        "OWNER_TOKEN",
    ]
    .iter()
    .any(|key| env_present(key));
    has_url && has_token
}

fn env_present(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{api_runtime_available_from_lookup, local_tool_contract, route_a_cli_ready};
    use homecli_proto::DevToolchainStatus;

    #[test]
    fn api_runtime_requires_key_and_model() {
        assert!(!api_runtime_available_from_lookup(|key| match key {
            "ELON_AGENT_API_KEY" => Some("secret".to_string()),
            _ => None,
        }));
        assert!(!api_runtime_available_from_lookup(|key| match key {
            "ELON_AGENT_MODEL" => Some("gpt-test".to_string()),
            _ => None,
        }));
        assert!(api_runtime_available_from_lookup(|key| match key {
            "OPENAI_API_KEY" => Some("secret".to_string()),
            "OPENAI_MODEL" => Some("gpt-test".to_string()),
            _ => None,
        }));
    }

    #[test]
    fn route_a_ready_requires_allowed_cli_and_successful_probe() {
        let allowed = vec!["codex".to_string()];
        let failed_codex = tool("codex", false);
        let ready_claude = tool("claude", true);
        assert!(
            !route_a_cli_ready(&allowed, &[&failed_codex, &ready_claude]),
            "allowed CLI path without a successful version probe should not make Route A ready"
        );

        let ready_codex = tool("codex", true);
        assert!(route_a_cli_ready(&allowed, &[&ready_codex]));
    }

    #[test]
    fn local_tool_contract_exposes_route_b_c_guardrails() {
        let contract = local_tool_contract();

        assert!(contract.routes.contains(&"route_b_api_runtime".to_string()));
        assert!(contract
            .routes
            .contains(&"route_c_server_runtime".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"apply_patch".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"search_files".to_string()));
        assert!(contract.supported_tools.contains(&"file_info".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"read_file_range".to_string()));
        assert!(contract.supported_tools.contains(&"git_status".to_string()));
        assert!(contract
            .supported_tools
            .contains(&"download_router_status".to_string()));
        assert!(contract
            .approval_required_tools
            .contains(&"download_router_configure".to_string()));
        assert!(contract
            .approval_required_tools
            .contains(&"run_command".to_string()));
        assert!(contract
            .path_policy
            .as_deref()
            .unwrap_or_default()
            .contains("workspace_relative"));
        assert!(contract
            .recovery_policy
            .as_deref()
            .unwrap_or_default()
            .contains("without_original_tty"));
    }

    fn tool(name: &str, available: bool) -> DevToolchainStatus {
        DevToolchainStatus {
            name: name.to_string(),
            available,
            version: None,
            path: Some(format!("C:/tools/{name}.cmd")),
        }
    }
}

fn check_workspace_root(root: &PathBuf) -> (bool, Option<String>) {
    if let Err(e) = std::fs::create_dir_all(root) {
        return (
            false,
            Some(format!("无法创建工作区根目录 {}：{e}", root.display())),
        );
    }

    let probe = root.join(format!(".elon-dev-runtime-check-{}", std::process::id()));
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            (true, None)
        }
        Err(e) => (
            false,
            Some(format!("工作区根目录不可写 {}：{e}", root.display())),
        ),
    }
}

fn command_status(name: &str, version_args: &[&str]) -> DevToolchainStatus {
    let path = command_probe::command_path(name);
    let path_string = path.as_ref().map(|path| path.to_string_lossy().to_string());

    if path.is_none() {
        return DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: None,
            path: path_string,
        };
    }

    if path_only_tool(name) {
        return DevToolchainStatus {
            name: name.to_string(),
            available: true,
            version: None,
            path: path_string,
        };
    }

    let program = path.as_deref().unwrap();
    match run_version_command(program, version_args, VERSION_CHECK_TIMEOUT) {
        VersionCommandResult::Output(output) if output.status.success() => DevToolchainStatus {
            name: name.to_string(),
            available: true,
            version: first_non_empty_line(&output.stdout, &output.stderr),
            path: path_string,
        },
        VersionCommandResult::Output(output) => DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: first_non_empty_line(&output.stdout, &output.stderr),
            path: path_string,
        },
        VersionCommandResult::TimedOut => DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: Some("version check timed out".to_string()),
            path: path_string,
        },
        VersionCommandResult::Failed => DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: None,
            path: path_string,
        },
    }
}

fn path_only_tool(_name: &str) -> bool {
    false
}

enum VersionCommandResult {
    Output(Output),
    TimedOut,
    Failed,
}

fn run_version_command(
    program: &std::path::Path,
    version_args: &[&str],
    timeout: Duration,
) -> VersionCommandResult {
    let mut command = command_probe::command_from_path(program);
    command
        .args(version_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return VersionCommandResult::Failed,
    };
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map(VersionCommandResult::Output)
                    .unwrap_or(VersionCommandResult::Failed);
            }
            Ok(None) if started_at.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait_with_output();
                return VersionCommandResult::TimedOut;
            }
            Ok(None) => thread::sleep(VERSION_CHECK_POLL_INTERVAL),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return VersionCommandResult::Failed;
            }
        }
    }
}

fn android_sdk_status() -> DevToolchainStatus {
    let candidates = android_sdk_candidates();
    let path = candidates
        .into_iter()
        .find(|path| path.join("platforms").exists() && path.join("build-tools").exists());

    DevToolchainStatus {
        name: "android_sdk".to_string(),
        available: path.is_some(),
        version: None,
        path: path.map(|path| path.to_string_lossy().to_string()),
    }
}

fn android_sdk_candidates() -> Vec<PathBuf> {
    let mut values = Vec::new();
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                values.push(PathBuf::from(value));
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            values.push(PathBuf::from(local).join("Android").join("Sdk"));
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            values.push(PathBuf::from(&home).join("Android").join("Sdk"));
            values.push(PathBuf::from(home).join("android-sdk"));
        }
    }

    values
}

fn first_non_empty_line(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    [stdout, stderr].into_iter().find_map(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    })
}
