//! Project-scoped, vendor-neutral Streamable HTTP MCP for document governance.

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    extract::{Path as AxumPath, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::node_agent_project_docs_mcp_tools::{call_tool, tool_definitions};
use crate::NodeRuntime;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const SESSION_TTL_SECONDS: u64 = 2 * 60 * 60;
const MAX_SESSION_FILE_BYTES: u64 = 32 * 1024;
const INVALID_SESSION_GRACE_SECONDS: u64 = 5 * 60;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapRequest {
    #[serde(default)]
    project_root: Option<String>,
    #[serde(default)]
    vault_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDocsMcpSession {
    id: String,
    token: String,
    project_root: String,
    #[serde(default)]
    managed_vault_id: Option<String>,
    created_at: u64,
    expires_at: u64,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/project-docs/mcp/bootstrap", post(bootstrap_handler))
        .route("/api/project-docs/mcp/:session_id", post(mcp_handler))
        .merge(crate::project_document_observability_api::routes())
        .merge(crate::project_document_governance_api::routes())
}

#[cfg(test)]
pub(crate) fn test_transport_routes() -> Router {
    Router::new().route("/api/project-docs/mcp/:session_id", post(mcp_handler))
}

pub(crate) fn descriptor_for_project(project_root: &str, host_port: u16) -> Result<Value> {
    let root = validate_project_root(project_root)?;
    create_descriptor(&root, None, host_port)
}

pub(crate) fn descriptor_for_vault(vault_id: &str, host_port: u16) -> Result<Value> {
    let vault = crate::project_document_vault::resolve_or_create(vault_id)?;
    create_descriptor(&vault.workspace, Some(vault.vault_id), host_port)
}

fn create_descriptor(
    root: &Path,
    managed_vault_id: Option<String>,
    host_port: u16,
) -> Result<Value> {
    let _guard = session_create_lock();
    cleanup_expired_sessions();
    let now = unix_seconds();
    let session = ProjectDocsMcpSession {
        id: format!("docs_{}", uuid::Uuid::new_v4().simple()),
        token: format!(
            "pd_{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ),
        project_root: root.to_string_lossy().to_string(),
        managed_vault_id,
        created_at: now,
        expires_at: now.saturating_add(SESSION_TTL_SECONDS),
    };
    let final_session_dir = session_dir(&session.id);
    let staging_dir = session_root().join(format!(".creating_{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&staging_dir)
        .with_context(|| format!("创建项目文档 MCP 会话目录失败：{}", staging_dir.display()))?;
    let create_result = write_session_files(&staging_dir, &session, host_port).and_then(|_| {
        fs::rename(&staging_dir, &final_session_dir).with_context(|| {
            format!(
                "发布项目文档 MCP 会话目录失败：{}",
                final_session_dir.display()
            )
        })
    });
    if let Err(error) = create_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }
    let operation_id =
        match crate::project_document_observability::mark_session_ready(&root, &session.id) {
            Ok(operation_id) => operation_id,
            Err(error) => {
                let _ = fs::remove_dir_all(&final_session_dir);
                return Err(error);
            }
        };
    let url = format!(
        "http://127.0.0.1:{host_port}/api/project-docs/mcp/{}?token={}",
        session.id, session.token
    );
    let config_path = final_session_dir.join("mcp.json");
    let copilot_config_path = final_session_dir.join("copilot-mcp.json");
    let claude_config_path = final_session_dir.join("claude-mcp.json");
    let gemini_config_path = final_session_dir.join("gemini-settings.json");
    Ok(json!({
        "name": "yilong-project-docs",
        "transport": "streamable-http",
        "url": url,
        "configPath": config_path.display().to_string(),
        "configPaths": {
            "codex": config_path.display().to_string(),
            "copilot": copilot_config_path.display().to_string(),
            "claude": claude_config_path.display().to_string(),
            "gemini": gemini_config_path.display().to_string()
        },
        "sessionId": session.id,
        "operationId": operation_id,
        "projectRoot": session.project_root,
        "managedVaultId": session.managed_vault_id,
        "expiresAt": session.expires_at,
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "purpose": "低 token 分析项目文档权威性、质量与联邦节点；支持项目 Git 工作区和平台托管版本的个人知识库。",
    }))
}

fn write_session_files(
    directory: &Path,
    session: &ProjectDocsMcpSession,
    host_port: u16,
) -> Result<()> {
    write_private_json(&directory.join("session.json"), session)?;
    let url = format!(
        "http://127.0.0.1:{host_port}/api/project-docs/mcp/{}?token={}",
        session.id, session.token
    );
    write_private_json(
        &directory.join("mcp.json"),
        &json!({"mcpServers":{"yilong_project_docs":{
            "url":url,"required":false,"toolTimeoutSec":60
        }}}),
    )?;
    write_private_json(
        &directory.join("copilot-mcp.json"),
        &json!({"mcpServers":{"yilong_project_docs":{
            "type":"http","url":url,"tools":["*"],"timeout":60000
        }}}),
    )?;
    write_private_json(
        &directory.join("claude-mcp.json"),
        &json!({"mcpServers":{"yilong_project_docs":{
            "type":"http","url":url,"timeout":60000
        }}}),
    )?;
    write_private_json(
        &directory.join("gemini-settings.json"),
        &json!({"mcpServers":{"yilong_project_docs":{
            "httpUrl":url,"timeout":60000,"trust":false
        }}}),
    )
}

pub(crate) async fn handle_request(workspace: &Path, request: McpRequest) -> Option<Value> {
    let id = request.id.clone().unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-project-docs", "version": "1.0.0" },
            "instructions": "先调用 project_docs_analyze；再按任务局部查询图谱、节点和 token 阅读计划。大型项目按 federation scope_id 分页。目录与图谱预分类不读正文且 classification_model_tokens=0；用 project_docs_get_issues 取得质量证据，只用 project_docs_read 按需读歧义文档。聊天整理使用 project_discussions_get_graph/get_node 先看已有讨论结构；新来源先 get_source_manifest，再按稳定 chunk id 逐块 read_source_chunk，每块只读一次并在 source.processed_chunk_ids 记录进度，expected_source_revision 必须逐字复制刚读取的 manifest.source_revision，禁止用普通文档读取截断长聊天。每个节点的 root_id 都填写所属根主题的稳定 ID，根节点自身 root_id=id。除 topic 外，每个新增或修改节点必须有 1 至 3 句话的可复用 summary；节点 status 只允许 open、exploring、accepted、rejected、superseded、implemented，不能使用 proposed。关系类型只使用 save_proposal schema 的 relation enum，不能自造 contains 等关系。回顾演化优先使用 project_discussions_get_history/get_graph_at_version/compare_versions/trace_node，禁止为了看版本而重读聊天正文。修改前调用 project_discussions_review_graph；确定性问题可 prepare_safe_repair→save_proposal→apply，语义问题只读取 issue 命中的来源后再生成 proposal。apply 始终创建可回看的新版本。save_proposal 保存来源、分支和晋升建议，apply 才更新讨论图和创建新文档。讨论来源默认无权威性，假设、意见、证据、决策和当前事实必须分开。authorization_mode 默认 git_backed_full：普通文档建议和讨论图应用均创建整理前后提交。普通 Git 与托管知识库均可 get_history/get_version_diff；restore 只创建新提交并拒绝混合代码版本。review_all 必须用户审核；suggestions_only 只保存建议。禁止覆盖、越界、修改代码或自动 push。"
        })),
        "notifications/initialized" => return None,
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(workspace, request.params),
        "ping" => Ok(json!({})),
        _ => Err(anyhow!("不支持 MCP method: {}", request.method)),
    };
    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": format!("{error:#}") }
        }),
    })
}

async fn bootstrap_handler(Json(request): Json<BootstrapRequest>) -> Response {
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    let descriptor = match (request.project_root.as_deref(), request.vault_id.as_deref()) {
        (Some(project_root), None) => descriptor_for_project(project_root, host_port),
        (None, Some(vault_id)) => descriptor_for_vault(vault_id, host_port),
        _ => Err(anyhow!("必须且只能提供 projectRoot 或 vaultId")),
    };
    match descriptor {
        Ok(mcp) => Json(json!({ "ok": true, "mcp": mcp })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn mcp_handler(
    AxumPath(session_id): AxumPath<String>,
    Query(query): Query<McpQuery>,
    Json(request): Json<McpRequest>,
) -> Response {
    let workspace = match authorize_session(&session_id, &query.token) {
        Ok(workspace) => workspace,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, format!("{error:#}")),
    };
    match handle_request(&workspace, request).await {
        Some(response) => Json(response).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

pub(crate) fn authorize_session(session_id: &str, token: &str) -> Result<PathBuf> {
    if !valid_session_id(session_id) || token.trim().is_empty() {
        bail!("项目文档 MCP 会话凭证无效");
    }
    let path = session_dir(session_id).join("session.json");
    let metadata = fs::metadata(&path).context("项目文档 MCP 会话不存在")?;
    if metadata.len() > MAX_SESSION_FILE_BYTES {
        bail!("项目文档 MCP 会话文件异常");
    }
    let session: ProjectDocsMcpSession = serde_json::from_slice(&fs::read(&path)?)?;
    if session.id != session_id || session.token != token || session.expires_at < unix_seconds() {
        bail!("项目文档 MCP 会话已过期或令牌无效");
    }
    validate_project_root(&session.project_root)
}

fn validate_project_root(project_root: &str) -> Result<PathBuf> {
    let root = PathBuf::from(project_root.trim())
        .canonicalize()
        .context("projectRoot 不存在或不可访问")?;
    if !root.is_dir() || !root.join(".git").exists() {
        bail!("projectRoot 必须是现存 Git 工作区");
    }
    Ok(root)
}

fn session_root() -> PathBuf {
    std::env::temp_dir().join("elon-project-docs-mcp")
}

fn session_dir(session_id: &str) -> PathBuf {
    session_root().join(session_id)
}

fn valid_session_id(value: &str) -> bool {
    value.starts_with("docs_")
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn cleanup_expired_sessions() {
    let Ok(entries) = fs::read_dir(session_root()) else {
        return;
    };
    let now = unix_seconds();
    for entry in entries.flatten().take(128) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() {
            continue;
        }
        if name.starts_with(".creating_") {
            if directory_age_seconds(&path) > INVALID_SESSION_GRACE_SECONDS {
                let _ = fs::remove_dir_all(path);
            }
            continue;
        }
        if !valid_session_id(&name) {
            continue;
        }
        let session = fs::read(path.join("session.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ProjectDocsMcpSession>(&bytes).ok());
        let expired = session
            .map(|session| session.expires_at < now)
            .unwrap_or_else(|| directory_age_seconds(&path) > INVALID_SESSION_GRACE_SECONDS);
        if expired {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn directory_age_seconds(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .unwrap_or_default()
}

fn session_create_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_private_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    crate::node_agent_atomic_file::write(path, &bytes)
        .with_context(|| format!("写入项目文档 MCP 文件失败：{}", path.display()))
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": message.into() })),
    )
        .into_response()
}
