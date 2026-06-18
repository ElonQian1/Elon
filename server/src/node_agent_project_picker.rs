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
    };
    Ok((project, inspect))
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
        .creation_flags(CREATE_NO_WINDOW)
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
