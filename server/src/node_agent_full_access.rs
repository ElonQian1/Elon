// server/src/node_agent_full_access.rs

use anyhow::{anyhow, bail, Result};
use axum::{extract::State, http::StatusCode, Json};
use elon_pc_dev_runtime::{safe_path_part, workspace_root};
use homecli_proto::CliProjectContext;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

const ROUTE_A_CLIS: &[&str] = &["codex", "copilot", "claude", "gemini"];
const ROUTE_BC_APPROVAL_REQUIRED_TOOLS: &[&str] = &["write_file", "apply_patch", "run_command"];
const ROUTE_BC_HIGH_RISK_GIT_PUSH_DENIED: &[&str] = &[
    "-f", "-d", "--force*", "--delete", "--mirror", "--all", "--tags", "--prune", "+refspec",
    ":branch",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FullAccessGrant {
    /// Identity fields deliberately default to empty only for backwards
    /// compatible deserialization. Empty legacy grants never match a live
    /// runtime identity and therefore fail closed.
    #[serde(default)]
    pub owner_user_id: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub install_id: String,
    pub project_id: String,
    pub workspace_path: String,
    pub granted_at_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FullAccessGrantIdentity {
    owner_user_id: String,
    agent_id: String,
    install_id: String,
}

impl FullAccessGrantIdentity {
    pub(crate) fn new(owner_user_id: &str, agent_id: &str, install_id: &str) -> Result<Self> {
        Ok(Self {
            owner_user_id: clean_required("owner_user_id", owner_user_id)?.to_string(),
            agent_id: clean_required("agent_id", agent_id)?.to_string(),
            install_id: clean_required("install_id", install_id)?.to_string(),
        })
    }

    fn matches(&self, grant: &FullAccessGrant) -> bool {
        grant.owner_user_id == self.owner_user_id
            && grant.agent_id == self.agent_id
            && grant.install_id == self.install_id
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FullAccessGrantFile {
    #[serde(default)]
    grants: Vec<FullAccessGrant>,
}

#[derive(Debug)]
pub(crate) struct FullAccessGrantState {
    path: PathBuf,
    grants: RwLock<Vec<FullAccessGrant>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GrantFullAccessReq {
    #[serde(alias = "projectId")]
    project_id: String,
    #[serde(alias = "workspacePath")]
    workspace_path: String,
    #[serde(default, alias = "confirmFullAccess")]
    confirm_full_access: bool,
}

impl FullAccessGrantState {
    pub(crate) fn load_default() -> Self {
        Self::load_from_path(default_grants_path())
    }

    pub(crate) fn load_from_path(path: PathBuf) -> Self {
        let grants = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<FullAccessGrantFile>(&text).ok())
            .map(|file| file.grants)
            .unwrap_or_default();
        Self {
            path,
            grants: RwLock::new(grants),
        }
    }

    pub(crate) async fn grant_project(
        &self,
        identity: &FullAccessGrantIdentity,
        project_id: &str,
        workspace_path: &str,
    ) -> Result<FullAccessGrant> {
        let project_id = clean_required("project_id", project_id)?;
        let workspace_path = canonical_workspace_path(workspace_path)?;
        let grant = FullAccessGrant {
            owner_user_id: identity.owner_user_id.clone(),
            agent_id: identity.agent_id.clone(),
            install_id: identity.install_id.clone(),
            project_id: project_id.to_string(),
            workspace_path,
            granted_at_ms: now_ms(),
        };
        let snapshot = {
            let mut grants = self.grants.write().await;
            // Each bound identity keeps only one directory per project. Grants
            // belonging to another owner, node credential, or installation are
            // retained but can never authorize the current runtime.
            grants.retain(|item| {
                !(identity.matches(item)
                    && project_ids_equivalent(&item.project_id, &grant.project_id))
            });
            grants.push(grant.clone());
            grants.clone()
        };
        self.save(snapshot)?;
        Ok(grant)
    }

    pub(crate) async fn list(&self, identity: &FullAccessGrantIdentity) -> Vec<FullAccessGrant> {
        self.grants
            .read()
            .await
            .iter()
            .filter(|grant| identity.matches(grant))
            .cloned()
            .collect()
    }

    async fn require_project(
        &self,
        identity: &FullAccessGrantIdentity,
        project_id: &str,
        workspace_path: &str,
    ) -> Result<()> {
        let project_id = clean_required("project_id", project_id)?;
        let workspace_path = canonical_workspace_path(workspace_path)?;
        let grants = self.grants.read().await;
        let project_granted = grants.iter().any(|grant| {
            identity.matches(grant) && project_ids_equivalent(&grant.project_id, project_id)
        });
        let granted = grants.iter().any(|grant| {
            identity.matches(grant)
                && project_ids_equivalent(&grant.project_id, project_id)
                && same_workspace(&grant.workspace_path, &workspace_path)
        });
        if granted {
            return Ok(());
        }
        if project_granted {
            bail!("WORKSPACE_IDENTITY_MISMATCH: 请求目录与项目权威完全访问目录不一致。")
        }
        bail!("PROJECT_FULL_ACCESS_DISABLED: 项目尚未在本机启用完全访问。")
    }

    fn save(&self, grants: Vec<FullAccessGrant>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = FullAccessGrantFile { grants };
        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

/// `elon-project` was the Desktop supervisor's original default before the
/// self project was registered under its durable `elon-self` id. Keep this
/// compatibility deliberately narrow: aliases may share an authorization only
/// when they are two names for the same built-in project, never for arbitrary
/// user projects.
pub(crate) fn project_ids_equivalent(left: &str, right: &str) -> bool {
    canonical_project_id(left).eq_ignore_ascii_case(canonical_project_id(right))
}

fn canonical_project_id(value: &str) -> &str {
    match value.trim() {
        value if value.eq_ignore_ascii_case("elon-project") => "elon-self",
        value => value,
    }
}

pub(crate) async fn require_route_a_full_access_grant(
    grants: &FullAccessGrantState,
    identity: &FullAccessGrantIdentity,
    cli_name: &str,
    runtime_permission: Option<&str>,
    project_context: Option<&CliProjectContext>,
    cwd: Option<&str>,
    allow_personal_chat_bypass: bool,
    task_record: Option<&crate::node_agent_local_task_store::LocalTaskRecord>,
) -> Result<()> {
    if !is_route_a_cli(cli_name) || !is_full_access(runtime_permission) {
        return Ok(());
    }
    let context = project_context
        .ok_or_else(|| anyhow!("Route A 完全访问必须携带项目上下文，已拒绝执行。"))?;
    // AI 聊天模式（project_id = "chat"）不需要本机 grant，直接放行。
    if allow_personal_chat_bypass && context.project_id == "chat" {
        return Ok(());
    }
    let cwd = cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Route A 完全访问必须携带项目目录，已拒绝执行。"))?;
    if platform_managed_workspace_matches(&context.project_id, cwd)
        && !is_legacy_conversation_worktree(cwd, &context.project_id, &context.conversation_id)
    {
        return Ok(());
    }
    if let Some(record) = task_record {
        if task_record_proves_managed_isolated_workspace(grants, identity, context, cwd, record)
            .await
        {
            return Ok(());
        }
        if recorded_workspace_paths(record)
            .iter()
            .any(|path| same_workspace(path, cwd))
        {
            bail!("ISOLATED_WORKTREE_AUTH_MISSING: 隔离 worktree 缺少有效的平台 provenance、Git 身份或 root lease 证据。")
        }
    }
    grants
        .require_project(identity, &context.project_id, cwd)
        .await
}

async fn task_record_proves_managed_isolated_workspace(
    grants: &FullAccessGrantState,
    identity: &FullAccessGrantIdentity,
    context: &CliProjectContext,
    cwd: &str,
    record: &crate::node_agent_local_task_store::LocalTaskRecord,
) -> bool {
    if record.owner_user_id != identity.owner_user_id
        || record.agent_id != identity.agent_id
        || record.install_id != identity.install_id
        || !project_ids_equivalent(&record.project_id, &context.project_id)
        || record.conversation_id != context.conversation_id
    {
        return false;
    }
    let Some(status) = record.workspace_status.as_ref() else {
        return false;
    };
    if status.get("isolated").and_then(serde_json::Value::as_bool) != Some(true)
        || status
            .get("platform_provenance")
            .and_then(serde_json::Value::as_str)
            != Some("elon.conversation_worktree.v1")
        || status
            .get("project_id")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|id| !project_ids_equivalent(id, &context.project_id))
    {
        return false;
    }
    let Some(root_task_id) = status
        .get("root_task_id")
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.is_empty())
    else {
        return false;
    };
    let Some(base) = status
        .get("base_workspace_path")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    let Some(active) = status
        .get("active_workspace_path")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    let expected_lease = format!("elon-supervision:{root_task_id}");
    if !same_existing_path(active, cwd)
        || grants
            .require_project(identity, &context.project_id, base)
            .await
            .is_err()
        || crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            Path::new(base),
            Path::new(active),
        )
        .ok()
        .flatten()
        .as_deref()
            != Some(expected_lease.as_str())
    {
        return false;
    }
    git_identity_matches(base, active, status)
}

fn same_existing_path(left: &str, right: &str) -> bool {
    canonical_existing_path(left)
        .zip(canonical_existing_path(right))
        .is_some_and(|(a, b)| a == b)
}

fn git_value(cwd: &str, args: &[&str]) -> Option<String> {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\\', "/")
    })
}

fn git_identity_matches(base: &str, active: &str, status: &serde_json::Value) -> bool {
    let base_common = git_value(
        base,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    let active_common = git_value(
        active,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    if base_common.is_none() || base_common != active_common {
        return false;
    }
    if status
        .get("git_common_dir")
        .and_then(serde_json::Value::as_str)
        .map(|v| v.replace('\\', "/"))
        != active_common
    {
        return false;
    }
    let revision = status
        .get("base_revision")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if revision.is_empty()
        || git_value(active, &["merge-base", "--is-ancestor", revision, "HEAD"]).is_none()
    {
        return false;
    }
    let remote = status
        .get("git_remote")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    git_value(base, &["config", "--get", "remote.origin.url"])
        .as_deref()
        .unwrap_or("")
        == remote
}

fn task_record_proves_legacy_managed_workspace(
    record: &crate::node_agent_local_task_store::LocalTaskRecord,
    identity: &FullAccessGrantIdentity,
    context: &CliProjectContext,
    cwd: &str,
) -> bool {
    if record.owner_user_id != identity.owner_user_id
        || record.agent_id != identity.agent_id
        || record.install_id != identity.install_id
        || !project_ids_equivalent(&record.project_id, &context.project_id)
        || record.conversation_id != context.conversation_id
    {
        return false;
    }
    let Some(cwd) = canonical_existing_path(cwd) else {
        return false;
    };
    let paths = recorded_workspace_paths(record);
    let cwd_is_recorded = paths.iter().any(|candidate| {
        canonical_existing_path(candidate).is_some_and(|candidate| candidate == cwd)
    });
    cwd_is_recorded
        && paths
            .into_iter()
            .filter_map(canonical_existing_path)
            .any(|candidate| {
                is_legacy_conversation_worktree(
                    &candidate,
                    &context.project_id,
                    &context.conversation_id,
                )
            })
}

fn recorded_workspace_paths(
    record: &crate::node_agent_local_task_store::LocalTaskRecord,
) -> Vec<&str> {
    let mut paths = vec![record.workspace_path.as_str()];
    if let Some(status) = record.workspace_status.as_ref() {
        for key in [
            "base_workspace_path",
            "active_workspace_path",
            "workspace_path",
        ] {
            if let Some(path) = status.get(key).and_then(serde_json::Value::as_str) {
                paths.push(path);
            }
        }
    }
    paths
}

fn canonical_existing_path(value: &str) -> Option<String> {
    std::fs::canonicalize(Path::new(value))
        .ok()
        .map(|path| normalize_workspace_path(&path))
}

fn is_legacy_conversation_worktree(cwd: &str, project_id: &str, conversation_id: &str) -> bool {
    let parts = cwd
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| normalize_workspace_component(part.to_string()))
        .collect::<Vec<_>>();
    let project = normalize_workspace_component(safe_path_part(
        canonical_project_id(project_id),
        "project",
        80,
    ));
    let conversation =
        normalize_workspace_component(safe_path_part(conversation_id, "conversation", 100));
    parts.windows(4).any(|window| {
        window[0] == "workspaces"
            && window[1] == "conversation-worktrees"
            && window[2] == project
            && window[3] == conversation
    }) && parts.last() == Some(&conversation)
}

pub(crate) async fn current_grant_identity(
    runtime: &crate::NodeRuntime,
) -> Result<FullAccessGrantIdentity> {
    let credentials = runtime
        .creds()
        .await
        .ok_or_else(|| anyhow!("本机节点未绑定当前账号，不能使用完全访问授权。"))?;
    FullAccessGrantIdentity::new(
        &credentials.owner_user_id,
        &credentials.agent_id,
        &runtime.install_id,
    )
}

pub(crate) async fn grant_handler(
    State(runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<GrantFullAccessReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !req.confirm_full_access {
        return json_error(StatusCode::BAD_REQUEST, "缺少完全访问本机确认。");
    }
    let identity = match current_grant_identity(&runtime).await {
        Ok(identity) => identity,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match runtime
        .full_access_grants
        .grant_project(&identity, &req.project_id, &req.workspace_path)
        .await
    {
        Ok(grant) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "grant": grant,
            })),
        ),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn list_handler(
    State(runtime): State<Arc<crate::NodeRuntime>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let identity = match current_grant_identity(&runtime).await {
        Ok(identity) => identity,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "grants": runtime.full_access_grants.list(&identity).await,
        })),
    )
}

pub(crate) fn runtime_policy_summary() -> serde_json::Value {
    json!({
        "schema": "elon.pc_node.runtime_policy.v1",
        "fullAccess": {
            "routeAInstalledCliOnly": true,
            "routeARequiresLocalProjectGrant": true,
            "routeBCFullAccessEffect": "keeps_workspace_path_checks_command_allowlist_and_tool_approvals",
            "routeBCDangerFullAccessEffect": "danger_full_access_allows_absolute_paths_arbitrary_shell_and_skips_tool_approvals",
        },
        "routeBC": {
            "workspaceBoundary": "workspace_relative_no_git_no_symlink_escape_or_danger_full_access_absolute",
            "approvalRequiredTools": ROUTE_BC_APPROVAL_REQUIRED_TOOLS,
            "commandPolicy": "structured_project_command_allowlist_or_danger_full_access_shell",
            "highRiskGitPushDenied": ROUTE_BC_HIGH_RISK_GIT_PUSH_DENIED,
        },
        "operatorVisibility": {
            "statusEndpoint": "/api/status",
            "policyField": "runtime_policy",
            "grantCountField": "full_access_grant_count",
        }
    })
}

fn default_grants_path() -> PathBuf {
    let mut path = crate::state_path();
    path.set_file_name("full-access-grants.json");
    path
}

fn clean_required<'a>(label: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{label} 不能为空");
    }
    Ok(value)
}

fn canonical_workspace_path(workspace_path: &str) -> Result<String> {
    let value = clean_required("workspace_path", workspace_path)?;
    let path = Path::new(value);
    let full = std::fs::canonicalize(path)
        .map_err(|error| anyhow!("项目目录不可用: {} ({error})", path.display()))?;
    if !full.is_dir() {
        bail!("workspace_path 必须指向一个目录: {}", full.display());
    }
    Ok(full.to_string_lossy().to_string())
}

fn platform_managed_workspace_matches(project_id: &str, cwd: &str) -> bool {
    platform_managed_workspace_matches_under(project_id, cwd, &workspace_root())
}

fn platform_managed_workspace_matches_under(project_id: &str, cwd: &str, root: &Path) -> bool {
    let project_part = normalize_workspace_component(safe_path_part(project_id, "project", 80));
    let root = match std::fs::canonicalize(root) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let cwd = match std::fs::canonicalize(Path::new(cwd)) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let root = normalize_workspace_path(&root);
    let cwd = normalize_workspace_path(&cwd);
    if cwd != root && !cwd.starts_with(&format!("{root}/")) {
        return false;
    }
    let rel = cwd
        .strip_prefix(&root)
        .unwrap_or(cwd.as_str())
        .trim_start_matches('/');
    let parts = rel
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| normalize_workspace_component(part.to_string()))
        .collect::<Vec<_>>();

    let is_project_repo = (parts.len() >= 3 && parts[1] == project_part && parts[2] == "repo")
        || (parts.len() >= 2 && parts[0] == project_part && parts[1] == "repo");
    let is_conversation_worktree =
        parts.len() >= 3 && parts[0] == "conversation-worktrees" && parts[1] == project_part;
    is_project_repo || is_conversation_worktree
}

fn normalize_workspace_path(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('\\', "/");
    while value.ends_with('/') {
        value.pop();
    }
    normalize_workspace_component(value)
}

fn normalize_workspace_component(value: String) -> String {
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn is_route_a_cli(cli_name: &str) -> bool {
    ROUTE_A_CLIS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(cli_name.trim()))
}

fn is_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("full_access" | "danger_full_access")
    )
}

fn same_workspace(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
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

#[cfg(test)]
#[path = "node_agent_full_access_tests.rs"]
mod tests;
