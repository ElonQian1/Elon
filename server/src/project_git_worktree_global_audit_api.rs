use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_manage_project_members, json_error},
    project_git_worktree_audit_api::{
        audit_project_git_worktrees_for_access, ProjectGitWorktreeAuditEntry,
        ProjectGitWorktreeAuditProject, ProjectGitWorktreeAuditResponse,
        ProjectGitWorktreeAuditSummary,
    },
    store::{AdminProjectDetail, ProjectAccess, ProjectSummary},
    types::AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/git/worktrees/audit", get(audit_all_git_worktrees))
}

#[derive(Debug, Deserialize)]
struct GlobalGitWorktreeAuditQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct GlobalGitWorktreeAuditResponse {
    pub summary: GlobalGitWorktreeAuditSummary,
    pub projects: Vec<GlobalGitWorktreeAuditProjectResult>,
}

#[derive(Debug, Default, Serialize)]
pub struct GlobalGitWorktreeAuditSummary {
    pub total_projects: usize,
    pub audited_projects: usize,
    pub skipped_projects: usize,
    pub error_projects: usize,
    pub total_worktrees: usize,
    pub dirty_worktrees: usize,
    pub uncommitted_entries: u64,
    pub untracked_entries: u64,
    pub matched_worktrees: usize,
    pub unknown_dirty_worktrees: usize,
}

#[derive(Debug, Serialize)]
pub struct GlobalGitWorktreeAuditProjectResult {
    pub project: ProjectGitWorktreeAuditProject,
    pub status: String,
    pub error: Option<String>,
    pub workspace_path: Option<String>,
    pub git_root: Option<String>,
    pub warnings: Vec<String>,
    pub summary: ProjectGitWorktreeAuditSummary,
    pub worktrees: Vec<ProjectGitWorktreeAuditEntry>,
}

async fn audit_all_git_worktrees(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GlobalGitWorktreeAuditQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e),
    };
    let projects = match list_projects_for_global_audit(&state, &user.id) {
        Ok(projects) => projects,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let limit = query.limit.unwrap_or(30).clamp(1, 50);
    let mut summary = GlobalGitWorktreeAuditSummary {
        total_projects: projects.len().min(limit),
        ..GlobalGitWorktreeAuditSummary::default()
    };
    let mut results = Vec::new();

    for access in projects.into_iter().take(limit) {
        let project = project_from_access(&access);
        if !can_manage_project_members(&access.role) {
            summary.skipped_projects += 1;
            results.push(skipped_result(
                project,
                access.workspace_path.clone(),
                "需要项目 owner/admin 权限",
            ));
            continue;
        }
        if access
            .node_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            summary.skipped_projects += 1;
            results.push(skipped_result(
                project,
                access.workspace_path.clone(),
                "项目未绑定 PC 节点",
            ));
            continue;
        }
        if access
            .workspace_path
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            summary.skipped_projects += 1;
            results.push(skipped_result(project, None, "项目缺少 workspace_path"));
            continue;
        }

        let fallback_workspace_path = access.workspace_path.clone();
        match audit_project_git_worktrees_for_access(state.clone(), access).await {
            Ok(audit) => {
                summary.audited_projects += 1;
                add_project_summary(&mut summary, &audit.summary);
                results.push(audited_result(audit));
            }
            Err(e) => {
                summary.error_projects += 1;
                results.push(error_result(project, fallback_workspace_path, e.message));
            }
        }
    }

    Json(GlobalGitWorktreeAuditResponse {
        summary,
        projects: results,
    })
    .into_response()
}

fn list_projects_for_global_audit(
    state: &AppState,
    user_id: &str,
) -> anyhow::Result<Vec<ProjectAccess>> {
    if state.owner_token.is_some() && user_id == "local-owner" {
        return state.store.list_all_projects_admin().map(|projects| {
            projects
                .into_iter()
                .map(access_from_admin_project)
                .collect()
        });
    }
    state.store.list_projects_for_user(user_id).map(|projects| {
        projects
            .into_iter()
            .map(access_from_project_summary)
            .collect()
    })
}

fn audited_result(audit: ProjectGitWorktreeAuditResponse) -> GlobalGitWorktreeAuditProjectResult {
    GlobalGitWorktreeAuditProjectResult {
        project: audit.project,
        status: "audited".to_string(),
        error: None,
        workspace_path: Some(audit.workspace_path),
        git_root: audit.git_root,
        warnings: audit.warnings,
        summary: audit.summary,
        worktrees: audit.worktrees,
    }
}

fn skipped_result(
    project: ProjectGitWorktreeAuditProject,
    workspace_path: Option<String>,
    reason: &str,
) -> GlobalGitWorktreeAuditProjectResult {
    GlobalGitWorktreeAuditProjectResult {
        project,
        status: "skipped".to_string(),
        error: Some(reason.to_string()),
        workspace_path,
        git_root: None,
        warnings: Vec::new(),
        summary: zero_project_summary(),
        worktrees: Vec::new(),
    }
}

fn error_result(
    project: ProjectGitWorktreeAuditProject,
    workspace_path: Option<String>,
    message: String,
) -> GlobalGitWorktreeAuditProjectResult {
    GlobalGitWorktreeAuditProjectResult {
        project,
        status: "error".to_string(),
        error: Some(message),
        workspace_path,
        git_root: None,
        warnings: Vec::new(),
        summary: zero_project_summary(),
        worktrees: Vec::new(),
    }
}

fn add_project_summary(
    global: &mut GlobalGitWorktreeAuditSummary,
    project: &ProjectGitWorktreeAuditSummary,
) {
    global.total_worktrees += project.total_worktrees;
    global.dirty_worktrees += project.dirty_worktrees;
    global.uncommitted_entries += project.uncommitted_entries;
    global.untracked_entries += project.untracked_entries;
    global.matched_worktrees += project.matched_worktrees;
    global.unknown_dirty_worktrees += project.unknown_dirty_worktrees;
}

fn zero_project_summary() -> ProjectGitWorktreeAuditSummary {
    ProjectGitWorktreeAuditSummary {
        total_worktrees: 0,
        dirty_worktrees: 0,
        uncommitted_entries: 0,
        untracked_entries: 0,
        matched_worktrees: 0,
        unknown_dirty_worktrees: 0,
    }
}

fn project_from_access(access: &ProjectAccess) -> ProjectGitWorktreeAuditProject {
    ProjectGitWorktreeAuditProject {
        id: access.id.clone(),
        name: access.name.clone(),
        workspace_path: access.workspace_path.clone(),
        node_id: access.node_id.clone(),
        role: access.role.clone(),
    }
}

fn access_from_project_summary(project: ProjectSummary) -> ProjectAccess {
    ProjectAccess {
        id: project.id,
        name: project.name,
        workspace_key: project.workspace_key,
        template: project.template,
        source_type: project.source_type,
        repo_url: project.repo_url,
        branch: project.branch,
        workspace_path: project.workspace_path,
        node_id: project.node_id,
        storage_node_id: project.storage_node_id,
        storage_repo_path: project.storage_repo_path,
        storage_repo_url: project.storage_repo_url,
        storage_worktree_path: project.storage_worktree_path,
        storage_status: project.storage_status,
        role: project.role,
        status: project.status,
        runtime_permission: project.runtime_permission,
    }
}

fn access_from_admin_project(project: AdminProjectDetail) -> ProjectAccess {
    ProjectAccess {
        id: project.id,
        name: project.name,
        workspace_key: project.workspace_key,
        template: project.template,
        source_type: project.source_type,
        repo_url: None,
        branch: None,
        workspace_path: project.workspace_path,
        node_id: project.node_id,
        storage_node_id: None,
        storage_repo_path: None,
        storage_repo_url: None,
        storage_worktree_path: None,
        storage_status: "none".to_string(),
        role: "owner".to_string(),
        status: project.status,
        runtime_permission: "project_write".to_string(),
    }
}
