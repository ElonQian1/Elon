use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

use crate::{
    project_attachment_paths::safe_project_path_part,
    project_auth::{auth_from_headers, json_error},
    store::ProjectDeletionTarget,
    types::AppState,
};

#[derive(Default, Serialize)]
struct ProjectFilesystemCleanup {
    removed_paths: Vec<String>,
    skipped_paths: Vec<String>,
}

/// DELETE /api/projects/:id — owner 删除项目，并清理服务端托管文件。
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    delete_project_for_user(state, &user.id, &project_id)
}

/// DELETE /api/user/:user_id/projects/:project_id — APK 旧匿名身份兼容入口。
pub async fn delete_user_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id)): Path<(String, String)>,
) -> Response {
    let user_id = match deletion_user_id(&state, &headers, &user_id) {
        Ok(user_id) => user_id,
        Err((status, message)) => return json_error(status, message),
    };
    delete_project_for_user(state, &user_id, &project_id)
}

fn deletion_user_id(
    state: &AppState,
    headers: &HeaderMap,
    path_user_id: &str,
) -> Result<String, (StatusCode, String)> {
    if has_authorization_header(headers) {
        let user = auth_from_headers(state, headers)
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
        if user.id != path_user_id {
            return Err((StatusCode::FORBIDDEN, "登录用户与路径用户不一致".into()));
        }
        return Ok(user.id);
    }

    if state.require_login {
        return Err((
            StatusCode::UNAUTHORIZED,
            "请先登录后再删除服务器项目".into(),
        ));
    }

    state
        .store
        .ensure_device_user(path_user_id)
        .map(|user| user.id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

fn has_authorization_header(headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn delete_project_for_user(state: Arc<AppState>, user_id: &str, project_id: &str) -> Response {
    let target = match state.store.project_deletion_target(user_id, project_id) {
        Ok(target) => target,
        Err(e) => return json_error(delete_error_status(&e.to_string()), e.to_string()),
    };

    let cleanup = match cleanup_project_files(&state, &target) {
        Ok(cleanup) => cleanup,
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("项目文件清理失败，已停止删除以避免服务器残留：{}", e),
            );
        }
    };

    if let Err(e) = state.store.purge_project_records(user_id, project_id) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    Json(serde_json::json!({
        "ok": true,
        "project_id": target.id,
        "project_name": target.name,
        "cleanup": cleanup,
    }))
    .into_response()
}

fn delete_error_status(message: &str) -> StatusCode {
    if message.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if message.contains("无权")
        || message.contains("owner")
        || message.contains("平台自身项目")
    {
        StatusCode::FORBIDDEN
    } else if message.contains("正在运行") {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    }
}

fn cleanup_project_files(
    state: &AppState,
    target: &ProjectDeletionTarget,
) -> anyhow::Result<ProjectFilesystemCleanup> {
    let mut cleanup = ProjectFilesystemCleanup::default();
    let managed_projects_root = state.project_root.join("projects");
    let managed_workspace = state.get_project_workspace(&target.workspace_key);
    let actual_workspace =
        state.resolve_project_workspace(&target.workspace_key, target.workspace_path.as_deref());

    if target.workspace_path.is_none() || same_path(&actual_workspace, &managed_workspace) {
        remove_managed_path(
            &managed_workspace,
            &managed_projects_root,
            "项目工作区",
            &mut cleanup,
        )?;
    } else {
        cleanup.skipped_paths.push(format!(
            "跳过外部 local_path 工作区（{}）：{}",
            target.source_type,
            actual_workspace.display()
        ));
    }

    let worktree_root = state
        .project_root
        .join("conversation-worktrees")
        .join(safe_project_path_part(&target.id, 64));
    let worktree_names = child_dir_names(&worktree_root);
    remove_managed_path(
        &worktree_root,
        &state.project_root.join("conversation-worktrees"),
        "会话 worktree",
        &mut cleanup,
    )?;

    remove_gradle_home_for_workspace(&managed_workspace, &mut cleanup)?;
    for name in worktree_names {
        remove_gradle_home(&name, &mut cleanup)?;
    }

    Ok(cleanup)
}

fn same_path(a: &FsPath, b: &FsPath) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn child_dir_names(path: &FsPath) -> Vec<String> {
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if !is_dir {
                return None;
            }
            entry.file_name().to_str().map(ToOwned::to_owned)
        })
        .collect()
}

fn remove_gradle_home_for_workspace(
    workspace: &FsPath,
    cleanup: &mut ProjectFilesystemCleanup,
) -> anyhow::Result<()> {
    if let Some(name) = workspace.file_name().and_then(|value| value.to_str()) {
        remove_gradle_home(name, cleanup)?;
    }
    Ok(())
}

fn remove_gradle_home(
    workspace_name: &str,
    cleanup: &mut ProjectFilesystemCleanup,
) -> anyhow::Result<()> {
    if workspace_name.trim().is_empty() {
        return Ok(());
    }
    let root = PathBuf::from("/opt/elon/gradle-homes");
    let path = root.join(workspace_name);
    remove_managed_path(&path, &root, "Gradle 缓存", cleanup)
}

fn remove_managed_path(
    path: &FsPath,
    allowed_root: &FsPath,
    label: &str,
    cleanup: &mut ProjectFilesystemCleanup,
) -> anyhow::Result<()> {
    if !path.exists() {
        cleanup
            .skipped_paths
            .push(format!("{}不存在：{}", label, path.display()));
        return Ok(());
    }

    let root = std::fs::canonicalize(allowed_root)?;
    let target = std::fs::canonicalize(path)?;
    if target == root || !target.starts_with(&root) {
        anyhow::bail!(
            "{}路径不在允许清理根目录内：{}（根目录：{}）",
            label,
            target.display(),
            root.display()
        );
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    cleanup
        .removed_paths
        .push(format!("{}：{}", label, path.display()));
    Ok(())
}
