// server/src/node_agent_full_access.rs

use anyhow::{anyhow, bail, Result};
use axum::{extract::State, http::StatusCode, Json};
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
    let cwd = cwd
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Route A 完全访问必须携带项目目录，已拒绝执行。"))?;
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
}
