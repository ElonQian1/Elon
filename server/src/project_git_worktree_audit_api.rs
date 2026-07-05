use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use homecli_proto::{AgentToServer, ProjectGitWorktreeEntry};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, can_manage_project_members, json_error, project_access},
    store::{AdminConversationEntry, ProjectExecutionSession},
    types::AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/:project_id/git/worktrees/audit",
        get(audit_project_git_worktrees),
    )
}

#[derive(Debug, Serialize)]
pub struct ProjectGitWorktreeAuditResponse {
    pub project: ProjectGitWorktreeAuditProject,
    pub workspace_path: String,
    pub git_root: Option<String>,
    pub warnings: Vec<String>,
    pub summary: ProjectGitWorktreeAuditSummary,
    pub worktrees: Vec<ProjectGitWorktreeAuditEntry>,
}

#[derive(Debug, Serialize)]
pub struct ProjectGitWorktreeAuditProject {
    pub id: String,
    pub name: String,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectGitWorktreeAuditSummary {
    pub total_worktrees: usize,
    pub dirty_worktrees: usize,
    pub uncommitted_entries: u64,
    pub untracked_entries: u64,
    pub matched_worktrees: usize,
    pub unknown_dirty_worktrees: usize,
}

#[derive(Debug, Serialize)]
pub struct ProjectGitWorktreeAuditEntry {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub current: bool,
    pub has_uncommitted_changes: bool,
    pub uncommitted_count: u32,
    pub untracked_count: u32,
    pub modified_count: u32,
    pub staged_count: u32,
    pub status_preview: Vec<String>,
    pub status_truncated: bool,
    pub status_error: Option<String>,
    pub conversation: Option<ProjectGitWorktreeConversation>,
    pub recommended_action: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectGitWorktreeConversation {
    pub conversation_id: String,
    pub user_id: String,
    pub user_account: Option<String>,
    pub title: Option<String>,
    pub conversation_status: Option<String>,
    pub message_count: Option<i64>,
    pub task_count: Option<i64>,
    pub last_task_status: Option<String>,
    pub execution_session_id: Option<String>,
    pub execution_status: Option<String>,
    pub merge_status: Option<String>,
    pub active_workspace_path: Option<String>,
    pub branch: Option<String>,
    pub updated_at: Option<String>,
    pub codex_thread_id: Option<String>,
    pub match_kind: String,
    pub match_confidence: u8,
}

#[derive(Clone)]
struct MatchCandidate {
    conversation_id: String,
    user_id: String,
    conversation: Option<AdminConversationEntry>,
    session: Option<ProjectExecutionSession>,
    match_kind: &'static str,
    confidence: u8,
}

struct MatchContext {
    sessions: Vec<ProjectExecutionSession>,
    conversations: Vec<AdminConversationEntry>,
    conversations_by_key: HashMap<String, AdminConversationEntry>,
    latest_session_by_key: HashMap<String, ProjectExecutionSession>,
    project_part: String,
}

impl MatchContext {
    fn new(
        project_id: &str,
        sessions: Vec<ProjectExecutionSession>,
        conversations: Vec<AdminConversationEntry>,
    ) -> Self {
        let mut conversations_by_key = HashMap::new();
        for conversation in &conversations {
            conversations_by_key.insert(
                conversation_key(&conversation.user_id, &conversation.id),
                conversation.clone(),
            );
        }

        let mut latest_session_by_key = HashMap::new();
        for session in &sessions {
            latest_session_by_key
                .entry(conversation_key(&session.user_id, &session.conversation_id))
                .or_insert_with(|| session.clone());
        }

        Self {
            sessions,
            conversations,
            conversations_by_key,
            latest_session_by_key,
            project_part: elon_pc_dev_runtime::safe_path_part(project_id, "project", 80),
        }
    }

    fn match_entry(&self, entry: &ProjectGitWorktreeEntry) -> Option<MatchCandidate> {
        let entry_path = path_key(&entry.path);
        for session in &self.sessions {
            if session
                .active_workspace_path
                .as_deref()
                .is_some_and(|path| path_key(path) == entry_path)
            {
                return Some(self.from_session(session, "active_workspace_path", 100));
            }
        }

        if let Some(branch) = entry.branch.as_deref().filter(|value| !value.is_empty()) {
            for session in &self.sessions {
                if session.branch.as_deref() == Some(branch) {
                    return Some(self.from_session(session, "branch", 90));
                }
            }
        }

        self.match_platform_convention(entry)
    }

    fn from_session(
        &self,
        session: &ProjectExecutionSession,
        match_kind: &'static str,
        confidence: u8,
    ) -> MatchCandidate {
        let key = conversation_key(&session.user_id, &session.conversation_id);
        MatchCandidate {
            conversation_id: session.conversation_id.clone(),
            user_id: session.user_id.clone(),
            conversation: self.conversations_by_key.get(&key).cloned(),
            session: Some(session.clone()),
            match_kind,
            confidence,
        }
    }

    fn match_platform_convention(&self, entry: &ProjectGitWorktreeEntry) -> Option<MatchCandidate> {
        let entry_path = path_key(&entry.path);
        let entry_branch = entry.branch.as_deref();

        for conversation in &self.conversations {
            let conversation_part =
                elon_pc_dev_runtime::safe_path_part(&conversation.id, "conversation", 80);
            let expected_branch = format!("ai/session/{}/{}", self.project_part, conversation_part);
            let key = conversation_key(&conversation.user_id, &conversation.id);
            if entry_branch == Some(expected_branch.as_str()) {
                return Some(MatchCandidate {
                    conversation_id: conversation.id.clone(),
                    user_id: conversation.user_id.clone(),
                    conversation: Some(conversation.clone()),
                    session: self.latest_session_by_key.get(&key).cloned(),
                    match_kind: "platform_branch_convention",
                    confidence: 75,
                });
            }

            let expected_path = format!(
                "/conversation-worktrees/{}/{}",
                self.project_part, conversation_part
            )
            .to_ascii_lowercase();
            if entry_path.ends_with(&expected_path) {
                return Some(MatchCandidate {
                    conversation_id: conversation.id.clone(),
                    user_id: conversation.user_id.clone(),
                    conversation: Some(conversation.clone()),
                    session: self.latest_session_by_key.get(&key).cloned(),
                    match_kind: "platform_path_convention",
                    confidence: 70,
                });
            }
        }
        None
    }
}

pub async fn audit_project_git_worktrees(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(e) => return json_error(StatusCode::NOT_FOUND, e),
    };
    if !can_manage_project_members(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "需要项目管理员权限查看 Git 工作现场");
    }

    let Some(node_id) = access
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return json_error(StatusCode::BAD_REQUEST, "项目未绑定 PC 节点");
    };
    let Some(workspace_path) = access
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return json_error(StatusCode::BAD_REQUEST, "项目缺少 workspace_path");
    };

    let audit = match state
        .agent_manager
        .dispatch_project_git_worktree_audit(node_id, workspace_path.to_string())
        .await
    {
        Ok(AgentToServer::ProjectGitWorktreesAudited { audit, .. }) => audit,
        Ok(AgentToServer::ProjectGitWorktreeAuditError { message, .. }) => {
            return json_error(StatusCode::BAD_GATEWAY, message)
        }
        Ok(other) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                format!("unexpected git worktree audit response: {other:?}"),
            )
        }
        Err(e) => return json_error(StatusCode::BAD_GATEWAY, e),
    };

    let sessions = match state.store.list_project_execution_sessions(&access.id, 200) {
        Ok(sessions) => sessions,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let conversations = match state.store.list_conversations_for_project_admin(&access.id) {
        Ok(conversations) => conversations,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let match_context = MatchContext::new(&access.id, sessions, conversations);

    let workspace_path = audit.workspace_path;
    let git_root = audit.git_root;
    let warnings = audit.warnings;
    let worktrees = audit
        .worktrees
        .into_iter()
        .map(|entry| enrich_worktree_entry(&state, &access.id, &match_context, entry))
        .collect::<Vec<_>>();
    let summary = summarize_worktrees(&worktrees);

    Json(ProjectGitWorktreeAuditResponse {
        project: ProjectGitWorktreeAuditProject {
            id: access.id,
            name: access.name,
            workspace_path: access.workspace_path,
            node_id: access.node_id,
            role: access.role,
        },
        workspace_path,
        git_root,
        warnings,
        summary,
        worktrees,
    })
    .into_response()
}

fn enrich_worktree_entry(
    state: &AppState,
    project_id: &str,
    match_context: &MatchContext,
    entry: ProjectGitWorktreeEntry,
) -> ProjectGitWorktreeAuditEntry {
    let candidate = match_context.match_entry(&entry);
    let conversation = candidate.map(|candidate| {
        let codex_thread_id = state
            .store
            .latest_task_codex_thread_id(project_id, &candidate.user_id, &candidate.conversation_id)
            .ok()
            .flatten();
        conversation_from_candidate(candidate, codex_thread_id)
    });
    let recommended_action =
        recommended_action(entry.current, entry.has_uncommitted_changes, &conversation);

    ProjectGitWorktreeAuditEntry {
        path: entry.path,
        branch: entry.branch,
        head: entry.head,
        detached: entry.detached,
        bare: entry.bare,
        current: entry.current,
        has_uncommitted_changes: entry.has_uncommitted_changes,
        uncommitted_count: entry.uncommitted_count,
        untracked_count: entry.untracked_count,
        modified_count: entry.modified_count,
        staged_count: entry.staged_count,
        status_preview: entry.status_preview,
        status_truncated: entry.status_truncated,
        status_error: entry.status_error,
        conversation,
        recommended_action,
    }
}

fn conversation_from_candidate(
    candidate: MatchCandidate,
    codex_thread_id: Option<String>,
) -> ProjectGitWorktreeConversation {
    let conversation = candidate.conversation;
    let session = candidate.session;
    ProjectGitWorktreeConversation {
        conversation_id: candidate.conversation_id,
        user_id: candidate.user_id,
        user_account: conversation
            .as_ref()
            .map(|value| value.user_account.clone()),
        title: conversation.as_ref().and_then(|value| value.title.clone()),
        conversation_status: conversation.as_ref().map(|value| value.status.clone()),
        message_count: conversation.as_ref().map(|value| value.message_count),
        task_count: conversation.as_ref().map(|value| value.task_count),
        last_task_status: conversation
            .as_ref()
            .and_then(|value| value.last_task_status.clone()),
        execution_session_id: session.as_ref().map(|value| value.id.clone()),
        execution_status: session.as_ref().map(|value| value.status.clone()),
        merge_status: session
            .as_ref()
            .and_then(|value| value.merge_status.clone()),
        active_workspace_path: session
            .as_ref()
            .and_then(|value| value.active_workspace_path.clone()),
        branch: session.as_ref().and_then(|value| value.branch.clone()),
        updated_at: session
            .as_ref()
            .map(|value| value.updated_at.clone())
            .or_else(|| conversation.as_ref().map(|value| value.updated_at.clone())),
        codex_thread_id,
        match_kind: candidate.match_kind.to_string(),
        match_confidence: candidate.confidence,
    }
}

fn summarize_worktrees(
    worktrees: &[ProjectGitWorktreeAuditEntry],
) -> ProjectGitWorktreeAuditSummary {
    let dirty_worktrees = worktrees
        .iter()
        .filter(|entry| entry.has_uncommitted_changes)
        .count();
    let matched_worktrees = worktrees
        .iter()
        .filter(|entry| entry.conversation.is_some())
        .count();
    ProjectGitWorktreeAuditSummary {
        total_worktrees: worktrees.len(),
        dirty_worktrees,
        uncommitted_entries: worktrees
            .iter()
            .map(|entry| u64::from(entry.uncommitted_count))
            .sum(),
        untracked_entries: worktrees
            .iter()
            .map(|entry| u64::from(entry.untracked_count))
            .sum(),
        matched_worktrees,
        unknown_dirty_worktrees: worktrees
            .iter()
            .filter(|entry| entry.has_uncommitted_changes && entry.conversation.is_none())
            .count(),
    }
}

fn recommended_action(
    current: bool,
    has_uncommitted_changes: bool,
    conversation: &Option<ProjectGitWorktreeConversation>,
) -> String {
    if !has_uncommitted_changes {
        return "无需处理".to_string();
    }
    if conversation.is_some() {
        return "询问会话或继续处理后再提交/清理".to_string();
    }
    if current {
        return "检查项目主工作区是否需要提交或清理".to_string();
    }
    "先人工确认归属，不要直接清理".to_string()
}

fn conversation_key(user_id: &str, conversation_id: &str) -> String {
    format!("{user_id}\0{conversation_id}")
}

fn path_key(raw: &str) -> String {
    raw.replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}
