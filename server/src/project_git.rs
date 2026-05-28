use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use crate::{
    agent,
    project_auth::{can_edit, json_error},
    project_mobile::ensure_mobile_project,
    store::ProjectAccess,
    types::AppState,
};

#[derive(Deserialize)]
pub struct GitConfigRequest {
    pub repo_url: String,
    pub branch: Option<String>,
    pub auth_type: Option<String>,
}

pub fn configured_local_project_workspace(project_id: &str) -> Option<PathBuf> {
    let env_key = format!("ELON_PROJECT_{}_PATH", env_key_suffix(project_id));
    if let Ok(path) = std::env::var(env_key) {
        let path = path.trim();
        if !path.is_empty() {
            return Some(path.into());
        }
    }

    if project_id == "elon-self" {
        return Some(agent::elon_self_workspace());
    }

    None
}

pub fn project_git_status_json(state: &AppState, project: &ProjectAccess) -> serde_json::Value {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let has_git = workspace.join(".git").exists();
    let origin = git_output(&workspace, &["remote", "get-url", "origin"]).ok();
    let branch = git_output(&workspace, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let (public_key, has_deploy_key) = read_deploy_public_key(state, &project.id)
        .map(|key| (Some(key), true))
        .unwrap_or((None, false));
    let remote_check = if has_git && origin.is_some() {
        Some(check_remote_access(
            &workspace,
            branch.as_deref().unwrap_or("main"),
        ))
    } else {
        None
    };
    let deploy_keys_url = origin
        .as_deref()
        .and_then(github_deploy_keys_url)
        .unwrap_or_else(|| "https://github.com/settings/keys".into());

    serde_json::json!({
        "project_id": project.id,
        "source_type": project.source_type,
        "workspace": workspace.to_string_lossy(),
        "git": {
            "has_git": has_git,
            "origin": origin,
            "branch": branch,
            "remote_check": remote_check,
        },
        "deploy_key": {
            "exists": has_deploy_key,
            "public_key": public_key,
            "github_deploy_keys_url": deploy_keys_url,
        },
        "recommended_auth": "deploy_key",
        "github_app": {
            "enabled": false,
            "message": "GitHub App 授权适合多用户正式版；当前版本先使用每项目 Deploy Key。"
        },
        "workflow": project_workflow_json(),
    })
}

pub async fn user_project_git_status(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    Json(project_git_status_json(&state, &project)).into_response()
}

pub async fn user_project_deploy_key(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (_user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());

    match ensure_project_deploy_key(&state, &project, &workspace) {
        Ok(public_key) => Json(serde_json::json!({
            "project_id": project.id,
            "public_key": public_key,
            "status": project_git_status_json(&state, &project),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn user_project_git_config(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<GitConfigRequest>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有配置项目的权限");
    }

    let repo_url = req.repo_url.trim();
    if repo_url.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Git 仓库地址不能为空");
    }
    let branch = req.branch.as_deref().unwrap_or("main").trim();
    let branch = if branch.is_empty() { "main" } else { branch };
    let auth_type = req.auth_type.as_deref().unwrap_or("deploy_key");

    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    if let Err(e) = configure_git_remote(&state, &project, &workspace, repo_url, branch, auth_type)
    {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let project =
        match state
            .store
            .update_project_git_config(&user.id, &project.id, repo_url, branch)
        {
            Ok(project) => project,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        };

    Json(project_git_status_json(&state, &project)).into_response()
}

pub fn ensure_project_deploy_key(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
) -> anyhow::Result<String> {
    std::fs::create_dir_all(workspace)?;
    if !workspace.join(".git").exists() {
        let _ = Command::new("git")
            .arg("init")
            .current_dir(workspace)
            .output();
    }

    let (private_key, _) = deploy_key_paths(state, &project.id);
    if !private_key.exists() {
        if let Some(parent) = private_key.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let comment = format!("elon-project-{}@server", project.id);
        let output = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", &comment, "-f"])
            .arg(&private_key)
            .output()?;
        if !output.status.success() {
            anyhow::bail!(
                "生成 SSH key 失败: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private_key, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    configure_deploy_key_ssh(workspace, &private_key)?;
    read_deploy_public_key(state, &project.id)
}

pub fn configure_git_remote(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
    repo_url: &str,
    branch: &str,
    auth_type: &str,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(workspace)?;
    if !workspace.join(".git").exists() {
        let output = Command::new("git")
            .arg("init")
            .current_dir(workspace)
            .output()?;
        if !output.status.success() {
            anyhow::bail!("git init 失败: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    let remote_exists = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    let args = if remote_exists {
        vec!["remote", "set-url", "origin", repo_url]
    } else {
        vec!["remote", "add", "origin", repo_url]
    };
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "设置 Git 远端失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let _ = Command::new("git")
        .args(["branch", "-M", branch])
        .current_dir(workspace)
        .output();

    if auth_type == "deploy_key" {
        let _ = ensure_project_deploy_key(state, project, workspace)?;
    }

    Ok(())
}

pub fn git_output(workspace: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn project_workflow_json() -> serde_json::Value {
    serde_json::json!({
        "title": "通用项目工作流",
        "summary": "所有项目都走同一套流程：先识别项目和授权，再读取项目文档，之后修改、验证、提交、推送；同项目共享动作由服务器排队保护。",
        "steps": [
            "项目准备：确认项目路径、Git 仓库、远端和写权限。",
            "读取文档：优先读取 AGENTS.md、CODEX.md、README.md、.github/instructions 和任务相关 docs。",
            "会话连续：其他 AI 模型以后只能作为旁路分析，结论必须回灌到当前 Codex CLI 原生 session。",
            "执行任务：按项目自己的技术栈修改代码，不把一龙自项目规则套到普通项目。",
            "验证保存：运行必要检查，commit；有可用远端时 push。",
            "共享动作：合并 main、版本号递增、APK 发布、服务器部署必须串行。"
        ],
        "codex_memory": "Codex CLI 不依赖长期记忆；服务器每次任务都会在提示词中注入这套通用流程，同时要求它读取当前项目仓库内的说明文档。以后接入的其他模型只能做旁路分析，结论会被整理后回灌到当前会话绑定的 Codex CLI 原生 session。"
    })
}

fn deploy_key_paths(state: &AppState, project_id: &str) -> (PathBuf, PathBuf) {
    let private_key = state
        .data_dir
        .join("git-keys")
        .join(env_key_suffix(project_id).to_ascii_lowercase())
        .join("deploy_ed25519");
    let public_key = private_key.with_extension("pub");
    (private_key, public_key)
}

fn read_deploy_public_key(state: &AppState, project_id: &str) -> anyhow::Result<String> {
    let (_, public_key) = deploy_key_paths(state, project_id);
    Ok(std::fs::read_to_string(public_key)?.trim().to_string())
}

fn configure_deploy_key_ssh(workspace: &Path, private_key: &Path) -> anyhow::Result<()> {
    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
        private_key.to_string_lossy()
    );
    let output = Command::new("git")
        .args(["config", "core.sshCommand", &ssh_command])
        .current_dir(workspace)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "配置项目 SSH key 失败: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn check_remote_access(workspace: &Path, branch: &str) -> serde_json::Value {
    let output = Command::new("git")
        .args(["ls-remote", "--heads", "origin", branch])
        .current_dir(workspace)
        .output();
    match output {
        Ok(out) if out.status.success() => serde_json::json!({
            "ok": true,
            "message": "远端读取正常"
        }),
        Ok(out) => serde_json::json!({
            "ok": false,
            "message": String::from_utf8_lossy(&out.stderr).trim()
        }),
        Err(e) => serde_json::json!({
            "ok": false,
            "message": e.to_string()
        }),
    }
}

fn github_deploy_keys_url(repo_url: &str) -> Option<String> {
    let trimmed = repo_url.trim().trim_end_matches(".git");
    let path = if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        path
    } else if let Some(path) = trimmed.strip_prefix("https://github.com/") {
        path
    } else if let Some(path) = trimmed.strip_prefix("http://github.com/") {
        path
    } else {
        return None;
    };
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    Some(format!("https://github.com/{owner}/{repo}/settings/keys"))
}

fn env_key_suffix(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}
