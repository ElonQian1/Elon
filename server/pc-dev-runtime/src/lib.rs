use homecli_proto::{DevToolchainStatus, NodeDevRuntimeProfile};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn collect_dev_runtime_profile(allowed_clis: &[String]) -> NodeDevRuntimeProfile {
    let root = workspace_root();
    let workspace_root_path = Some(root.to_string_lossy().to_string());
    let (workspace_root_writable, root_issue) = check_workspace_root(&root);

    let git = command_status("git", &["--version"]);
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
    let ai_cli_ready = allowed_clis.iter().any(|cli| {
        cli.eq_ignore_ascii_case("codex")
            || cli.eq_ignore_ascii_case("copilot")
            || cli.eq_ignore_ascii_case("claude")
    });

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

    NodeDevRuntimeProfile {
        workspace_root_path,
        workspace_root_writable,
        git_ready,
        workspace_provision_ready: workspace_root_writable && git_ready,
        dev_env_ready,
        ai_cli_ready,
        toolchains: vec![git, java, gradle, android_sdk, rustc, cargo, node, npm],
        issues,
    }
}

pub fn workspace_root() -> PathBuf {
    for key in [
        "ELON_NODE_WORKSPACE_ROOT",
        "ELON_PC_WORKSPACE_ROOT",
        "NODE_WORKSPACE_ROOT",
    ] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join("Elon").join("workspaces");
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".elon").join("workspaces");
        }
    }

    std::env::temp_dir().join("elon").join("workspaces")
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
    let path = command_path(name);
    let output = Command::new(name)
        .args(version_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) if output.status.success() => DevToolchainStatus {
            name: name.to_string(),
            available: true,
            version: first_non_empty_line(&output.stdout, &output.stderr),
            path,
        },
        Ok(output) => DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: first_non_empty_line(&output.stdout, &output.stderr),
            path,
        },
        Err(_) => DevToolchainStatus {
            name: name.to_string(),
            available: false,
            version: None,
            path,
        },
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
            values.push(PathBuf::from(home).join("Android").join("Sdk"));
            values.push(PathBuf::from(home).join("android-sdk"));
        }
    }

    values
}

fn command_path(name: &str) -> Option<String> {
    let which = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(which).arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
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
