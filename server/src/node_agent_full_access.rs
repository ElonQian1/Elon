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
    pub project_id: String,
    pub workspace_path: String,
    pub granted_at_ms: u128,
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
        project_id: &str,
        workspace_path: &str,
    ) -> Result<FullAccessGrant> {
        let project_id = clean_required("project_id", project_id)?;
        let workspace_path = canonical_workspace_path(workspace_path)?;
        let grant = FullAccessGrant {
            project_id: project_id.to_string(),
            workspace_path,
            granted_at_ms: now_ms(),
        };
        let snapshot = {
            let mut grants = self.grants.write().await;
            // 一个云端项目在一台 PC 上只保留一个完全访问目录，避免旧目录继续扩大权限。
            grants.retain(|item| item.project_id != grant.project_id);
            grants.push(grant.clone());
            grants.clone()
        };
        self.save(snapshot)?;
        Ok(grant)
    }

    pub(crate) async fn list(&self) -> Vec<FullAccessGrant> {
        self.grants.read().await.clone()
    }

    async fn require_project(&self, project_id: &str, workspace_path: &str) -> Result<()> {
        let project_id = clean_required("project_id", project_id)?;
        let workspace_path = canonical_workspace_path(workspace_path)?;
        let granted = self.grants.read().await.iter().any(|grant| {
            grant.project_id == project_id && same_workspace(&grant.workspace_path, &workspace_path)
        });
        if granted {
            return Ok(());
        }
        bail!(
            "Route A 完全访问尚未在本机授权：请在 PC 工作台设置中重新选择该项目目录并确认完全访问。"
        )
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

pub(crate) async fn require_route_a_full_access_grant(
    grants: &FullAccessGrantState,
    cli_name: &str,
    runtime_permission: Option<&str>,
    project_context: Option<&CliProjectContext>,
    cwd: Option<&str>,
) -> Result<()> {
    if !is_route_a_cli(cli_name) || !is_full_access(runtime_permission) {
        return Ok(());
    }
    let context = project_context
        .ok_or_else(|| anyhow!("Route A 完全访问必须携带项目上下文，已拒绝执行。"))?;
    // AI 聊天模式（project_id = "chat"）不需要本机 grant，直接放行。
    if context.project_id == "chat" {
        return Ok(());
    }
    let cwd = cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Route A 完全访问必须携带项目目录，已拒绝执行。"))?;
    if platform_managed_workspace_matches(&context.project_id, cwd) {
        return Ok(());
    }
    grants.require_project(&context.project_id, cwd).await
}

pub(crate) async fn grant_handler(
    State(runtime): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<GrantFullAccessReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !req.confirm_full_access {
        return json_error(StatusCode::BAD_REQUEST, "缺少完全访问本机确认。");
    }
    match runtime
        .full_access_grants
        .grant_project(&req.project_id, &req.workspace_path)
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
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "grants": runtime.full_access_grants.list().await,
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
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_workspace(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "elon_full_access_{label}_{}",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&path).expect("create temp workspace");
        path
    }

    fn grant_file(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon_full_access_grants_{label}_{}.json",
            Uuid::new_v4().simple()
        ))
    }

    #[test]
    fn platform_managed_workspace_allows_project_repo_and_conversation_worktree() {
        let root = temp_workspace("managed_root");
        let project_id = "prj_abc123";
        let project_part = safe_path_part(project_id, "project", 80);
        let repo = root.join("usr_1").join(&project_part).join("repo");
        let worktree = root
            .join("conversation-worktrees")
            .join(&project_part)
            .join("conv_1");
        std::fs::create_dir_all(&repo).expect("create managed repo");
        std::fs::create_dir_all(&worktree).expect("create managed worktree");

        assert!(platform_managed_workspace_matches_under(
            project_id,
            repo.to_string_lossy().as_ref(),
            &root
        ));
        assert!(platform_managed_workspace_matches_under(
            project_id,
            worktree.to_string_lossy().as_ref(),
            &root
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn platform_managed_workspace_rejects_other_project_paths() {
        let root = temp_workspace("managed_mismatch");
        let repo = root.join("usr_1").join("prj_other").join("repo");
        std::fs::create_dir_all(&repo).expect("create other repo");

        assert!(!platform_managed_workspace_matches_under(
            "prj_expected",
            repo.to_string_lossy().as_ref(),
            &root
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    fn context(project_id: &str) -> CliProjectContext {
        CliProjectContext {
            project_id: project_id.to_string(),
            conversation_id: "conv".to_string(),
            runtime_permission: Some("full_access".to_string()),
        }
    }

    #[tokio::test]
    async fn grant_and_require_full_access_for_same_project_path() {
        let workspace = temp_workspace("ok");
        let state = FullAccessGrantState::load_from_path(grant_file("ok"));
        state
            .grant_project("project_1", workspace.to_string_lossy().as_ref())
            .await
            .expect("grant project");

        require_route_a_full_access_grant(
            &state,
            "codex",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("grant should authorize matching project path");
    }

    #[tokio::test]
    async fn route_a_full_access_requires_local_grant() {
        let workspace = temp_workspace("missing");
        let state = FullAccessGrantState::load_from_path(grant_file("missing"));
        let error = require_route_a_full_access_grant(
            &state,
            "codex",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect_err("missing grant should reject full access");

        assert!(
            error.to_string().contains("完全访问尚未在本机授权"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn project_write_and_builtin_runtime_do_not_need_full_access_grant() {
        let workspace = temp_workspace("bypass");
        let state = FullAccessGrantState::load_from_path(grant_file("bypass"));

        require_route_a_full_access_grant(
            &state,
            "codex",
            Some("project_write"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("project_write route A should not require full-access grant");

        require_route_a_full_access_grant(
            &state,
            "api-runtime",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
        )
        .await
        .expect("built-in runtime keeps its own sandbox guard");
    }

    #[test]
    fn runtime_policy_summary_exposes_route_bc_safety_limits() {
        let summary = runtime_policy_summary();

        assert_eq!(summary["schema"], "elon.pc_node.runtime_policy.v1");
        assert_eq!(summary["fullAccess"]["routeAInstalledCliOnly"], true);
        assert_eq!(
            summary["fullAccess"]["routeBCFullAccessEffect"],
            "keeps_workspace_path_checks_command_allowlist_and_tool_approvals"
        );
        assert_eq!(
            summary["fullAccess"]["routeBCDangerFullAccessEffect"],
            "danger_full_access_allows_absolute_paths_arbitrary_shell_and_skips_tool_approvals"
        );
        assert_eq!(
            summary["operatorVisibility"]["policyField"],
            "runtime_policy"
        );

        let approval_tools = summary["routeBC"]["approvalRequiredTools"]
            .as_array()
            .expect("approvalRequiredTools should be an array");
        for tool in ["write_file", "apply_patch", "run_command"] {
            assert!(
                approval_tools
                    .iter()
                    .any(|item| item.as_str() == Some(tool)),
                "missing approval tool {tool}"
            );
        }

        let denied = summary["routeBC"]["highRiskGitPushDenied"]
            .as_array()
            .expect("highRiskGitPushDenied should be an array");
        for arg in ["--force*", "--delete", "--mirror", "+refspec", ":branch"] {
            assert!(
                denied.iter().any(|item| item.as_str() == Some(arg)),
                "missing high-risk git push marker {arg}"
            );
        }
    }
}
