// server/src/node_agent_project_picker.rs

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{project_landing, project_workspace_inspect};

#[derive(Debug, Deserialize)]
pub(crate) struct InspectLocalProjectReq {
    workspace_path: String,
}

#[derive(Debug, Serialize)]
struct LocalProjectInfo {
    name: String,
    workspace_path: String,
    description: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
    git_head: Option<String>,
    is_git_worktree: bool,
    has_uncommitted_changes: bool,
    uncommitted_count: Option<u32>,
    project_type: Option<String>,
    package_manager: Option<String>,
    run_command: Option<String>,
    test_command: Option<String>,
    build_command: Option<String>,
    detected_files: Vec<String>,
}

pub(crate) async fn pick_local_project_folder() -> (StatusCode, Json<serde_json::Value>) {
    match pick_folder() {
        Ok(Some(path)) => project_info_response(&path),
        Ok(None) => (
            StatusCode::OK,
            Json(json!({ "ok": true, "cancelled": true })),
        ),
        Err(error) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("打开文件夹选择器失败: {error}"),
        ),
    }
}

pub(crate) async fn inspect_local_project_folder(
    Json(req): Json<InspectLocalProjectReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    project_info_response(req.workspace_path.trim())
}

fn project_info_response(workspace_path: &str) -> (StatusCode, Json<serde_json::Value>) {
    let workspace_path = workspace_path.trim();
    if workspace_path.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "workspace_path 不能为空");
    }

    match local_project_info(workspace_path) {
        Ok((project, inspect)) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "project": project,
                "inspect": inspect,
                "landing": project_landing::load_workspace_landing(Path::new(workspace_path)),
            })),
        ),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn local_project_info(
    workspace_path: &str,
) -> anyhow::Result<(
    LocalProjectInfo,
    homecli_proto::ProjectWorkspaceInspectStatus,
)> {
    let path = PathBuf::from(workspace_path);
    let inspect = project_workspace_inspect::inspect_project_workspace(workspace_path)?;
    if !inspect.path_exists {
        anyhow::bail!("PC 本地路径不存在: {workspace_path}");
    }
    if !inspect.is_dir {
        anyhow::bail!("workspace_path 必须指向一个目录");
    }

    let name = project_name(&path);
    let profile = detect_project_profile(&path);
    let description = Some(format!("绑定到本 PC 节点的本地项目: {name}"));
    let project = LocalProjectInfo {
        name,
        workspace_path: inspect.workspace_path.clone(),
        description,
        repo_url: inspect.git_remote_origin.clone(),
        branch: inspect
            .git_branch
            .as_deref()
            .filter(|value| *value != "HEAD")
            .map(ToOwned::to_owned),
        git_head: inspect.git_head.clone(),
        is_git_worktree: inspect.is_git_worktree,
        has_uncommitted_changes: inspect.has_uncommitted_changes,
        uncommitted_count: inspect.uncommitted_count,
        project_type: profile.project_type,
        package_manager: profile.package_manager,
        run_command: profile.run_command,
        test_command: profile.test_command,
        build_command: profile.build_command,
        detected_files: profile.detected_files,
    };
    Ok((project, inspect))
}

#[derive(Debug, Default)]
struct ProjectProfile {
    project_type: Option<String>,
    package_manager: Option<String>,
    run_command: Option<String>,
    test_command: Option<String>,
    build_command: Option<String>,
    detected_files: Vec<String>,
}

fn detect_project_profile(path: &Path) -> ProjectProfile {
    let mut profile = ProjectProfile::default();
    let cargo = path.join("Cargo.toml");
    let package_json = path.join("package.json");
    let gradle = path.join("build.gradle");
    let gradle_kts = path.join("build.gradle.kts");
    let pyproject = path.join("pyproject.toml");

    if cargo.exists() {
        profile.project_type = Some("Rust".to_string());
        profile.package_manager = Some("Cargo".to_string());
        profile.run_command = Some("cargo run".to_string());
        profile.test_command = Some("cargo test".to_string());
        profile.build_command = Some("cargo build".to_string());
        profile.detected_files.push("Cargo.toml".to_string());
        return profile;
    }

    if package_json.exists() {
        profile.project_type = Some("Node.js".to_string());
        profile.package_manager = Some(node_package_manager(path));
        profile.detected_files.push("package.json".to_string());
        if path.join("pnpm-lock.yaml").exists() {
            profile.detected_files.push("pnpm-lock.yaml".to_string());
        } else if path.join("yarn.lock").exists() {
            profile.detected_files.push("yarn.lock".to_string());
        } else if path.join("package-lock.json").exists() {
            profile.detected_files.push("package-lock.json".to_string());
        }
        apply_package_json_scripts(&mut profile, &package_json);
        return profile;
    }

    if gradle.exists() || gradle_kts.exists() {
        profile.project_type = Some("Gradle".to_string());
        profile.package_manager = Some("Gradle".to_string());
        profile.build_command = Some(gradle_command(path, "build"));
        profile.test_command = Some(gradle_command(path, "test"));
        profile.detected_files.push(
            if gradle.exists() {
                "build.gradle"
            } else {
                "build.gradle.kts"
            }
            .to_string(),
        );
        return profile;
    }

    if pyproject.exists() {
        profile.project_type = Some("Python".to_string());
        profile.package_manager = Some("pyproject".to_string());
        profile.test_command = Some("python -m pytest".to_string());
        profile.detected_files.push("pyproject.toml".to_string());
    }

    profile
}

fn node_package_manager(path: &Path) -> String {
    if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if path.join("package-lock.json").exists() {
        "npm".to_string()
    } else {
        "npm".to_string()
    }
}

fn apply_package_json_scripts(profile: &mut ProjectProfile, package_json: &Path) {
    let Some(scripts) = std::fs::read_to_string(package_json)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.as_object().cloned())
    else {
        return;
    };
    let manager = profile.package_manager.as_deref().unwrap_or("npm");
    profile.run_command = first_script_command(manager, &scripts, &["dev", "start"]);
    profile.test_command = first_script_command(manager, &scripts, &["test"]);
    profile.build_command = first_script_command(manager, &scripts, &["build"]);
}

fn first_script_command(
    manager: &str,
    scripts: &serde_json::Map<String, serde_json::Value>,
    names: &[&str],
) -> Option<String> {
    names
        .iter()
        .find(|name| scripts.get(**name).is_some())
        .map(|name| match manager {
            "yarn" => format!("yarn {name}"),
            "pnpm" => format!("pnpm {name}"),
            _ => format!("npm run {name}"),
        })
}

fn gradle_command(path: &Path, task: &str) -> String {
    if cfg!(windows) && path.join("gradlew.bat").exists() {
        format!("gradlew.bat {task}")
    } else if path.join("gradlew").exists() {
        format!("./gradlew {task}")
    } else {
        format!("gradle {task}")
    }
}

fn project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("本地项目")
        .to_string()
}

fn json_error(
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
fn pick_folder() -> anyhow::Result<Option<String>> {
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
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dialog.SelectedPath
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
fn pick_folder() -> anyhow::Result<Option<String>> {
    anyhow::bail!("本机文件夹选择器目前仅支持 Windows 客户端");
}

#[cfg(test)]
mod tests {
    use super::detect_project_profile;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elon-project-picker-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_rust_project_commands() {
        let dir = temp_project("rust");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Rust"));
        assert_eq!(profile.package_manager.as_deref(), Some("Cargo"));
        assert_eq!(profile.test_command.as_deref(), Some("cargo test"));
        assert!(profile.detected_files.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn detects_node_package_manager_and_scripts() {
        let dir = temp_project("node");
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            dir.join("package.json"),
            r#"{"scripts":{"dev":"vite","test":"vitest","build":"vite build"}}"#,
        )
        .unwrap();

        let profile = detect_project_profile(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(profile.project_type.as_deref(), Some("Node.js"));
        assert_eq!(profile.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(profile.run_command.as_deref(), Some("pnpm dev"));
        assert_eq!(profile.test_command.as_deref(), Some("pnpm test"));
        assert_eq!(profile.build_command.as_deref(), Some("pnpm build"));
    }

    #[test]
    fn detects_gradle_and_python_projects() {
        let gradle = temp_project("gradle");
        std::fs::write(gradle.join("build.gradle.kts"), "plugins {}\n").unwrap();
        let gradle_profile = detect_project_profile(&gradle);
        let _ = std::fs::remove_dir_all(&gradle);
        assert_eq!(gradle_profile.project_type.as_deref(), Some("Gradle"));
        assert!(gradle_profile
            .build_command
            .as_deref()
            .unwrap_or_default()
            .contains("build"));

        let python = temp_project("python");
        std::fs::write(python.join("pyproject.toml"), "[project]\nname='demo'\n").unwrap();
        let python_profile = detect_project_profile(&python);
        let _ = std::fs::remove_dir_all(&python);
        assert_eq!(python_profile.project_type.as_deref(), Some("Python"));
        assert_eq!(
            python_profile.test_command.as_deref(),
            Some("python -m pytest")
        );
    }
}
