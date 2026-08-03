use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;

const LIST_TOOL: &str = "ui_list_design_targets";
const OPEN_TOOL: &str = "ui_open_design_target";
const CAPTURE_TOOL: &str = "ui_capture_design_surface";
const GET_TOOL: &str = "ui_get_design_surface";
const MAX_TREE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum DesignPlatform {
    Web,
    Pwa,
    Tauri,
    Android,
}

impl DesignPlatform {
    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" => Ok(Self::Web),
            "pwa" => Ok(Self::Pwa),
            "tauri" => Ok(Self::Tauri),
            "android" => Ok(Self::Android),
            _ => bail!("platform 只支持 web、pwa、tauri、android"),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Pwa => "pwa",
            Self::Tauri => "tauri",
            Self::Android => "android",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignTarget {
    pub(super) id: String,
    pub(super) platform: DesignPlatform,
    pub(super) label: String,
    pub(super) adapter: String,
    pub(super) evidence_level: String,
    pub(super) source_roots: Vec<String>,
    pub(super) config_files: Vec<String>,
    pub(super) capabilities: Vec<String>,
    pub(super) native_host_verified: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesignSessionRecord {
    schema_version: u32,
    design_session_id: String,
    mcp_session_id: String,
    platform: DesignPlatform,
    target: DesignTarget,
    route: String,
    url: Option<String>,
    viewport: Value,
    state: String,
    last_evidence: Option<Value>,
    created_at: String,
    updated_at: String,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    let capture_schema = crate::node_agent_pwa_runtime::tool_definition()
        .get("inputSchema")
        .cloned()
        .unwrap_or_else(|| json!({"type":"object"}));
    vec![
        tool(
            LIST_TOOL,
            "发现项目可由 AI 后台设计的 Web、PWA、Tauri 和 Android 目标；只返回小型技术栈与适配器索引，不读取页面正文。",
            json!({"type":"object","additionalProperties":false,"properties":{}}),
            true,
        ),
        tool(
            OPEN_TOOL,
            "打开独立后台设计会话。用户无需进入微调画布；后续捕获、交互和读取都绑定 designSessionId。",
            json!({
                "type":"object","additionalProperties":false,"required":["platform"],
                "properties":{
                    "platform":{"enum":["web","pwa","tauri","android"]},
                    "route":{"type":"string","minLength":1,"maxLength":2048,"default":"/"},
                    "url":{"type":"string","minLength":1,"maxLength":4096,"description":"可选本机/项目白名单 URL；秘密不得放入 query"},
                    "viewport":{"type":"object","additionalProperties":false,"properties":{
                        "width":{"type":"integer","minimum":240,"maximum":4096},
                        "height":{"type":"integer","minimum":240,"maximum":4096},
                        "deviceScaleFactor":{"type":"number","minimum":0.5,"maximum":4}
                    }}
                }
            }),
            false,
        ),
        tool(
            CAPTURE_TOOL,
            "在后台设计会话中执行受限点击/等待/文本断言并捕获 PNG 与 UI 语义树。Web/PWA/Tauri 复用受控无头浏览器；Tauri 第一阶段只证明前端 WebView，不冒充原生宿主验证。",
            json!({
                "type":"object","additionalProperties":false,"required":["designSessionId"],
                "properties":{
                    "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
                    "capture":capture_schema
                }
            }),
            false,
        ),
        tool(
            GET_TOOL,
            "按查询读取后台设计会话的紧凑 UI 节点、像素工件引用和平台覆盖状态；默认最多返回 40 个节点，不嵌入 PNG/Base64。",
            json!({
                "type":"object","additionalProperties":false,"required":["designSessionId"],
                "properties":{
                    "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
                    "query":{"type":"string","maxLength":240,"description":"匹配 selector、role 或 label"},
                    "limit":{"type":"integer","minimum":1,"maximum":80,"default":40}
                }
            }),
            true,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, LIST_TOOL | OPEN_TOOL | CAPTURE_TOOL | GET_TOOL)
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        LIST_TOOL => list(session),
        OPEN_TOOL => open(session, &arguments).await,
        CAPTURE_TOOL => capture(session, &arguments).await,
        GET_TOOL => get_surface(session, &arguments),
        _ => bail!("未知后台设计工具: {name}"),
    }
}

fn list(session: &LiveUiSession) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let (targets, inspected, truncated) = super::design_target_discovery::discover_targets(&root)?;
    Ok(json!({
        "schemaVersion":1,
        "targets":targets,
        "scan":{"filesInspected":inspected,"truncated":truncated,"contentEmbedded":false},
        "defaultWorkflow":[OPEN_TOOL, CAPTURE_TOOL, GET_TOOL],
    }))
}

async fn open(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let platform = DesignPlatform::parse(required_string(arguments, "platform")?)?;
    let (targets, _, _) = super::design_target_discovery::discover_targets(&root)?;
    let target = targets
        .into_iter()
        .find(|target| target.platform == platform)
        .ok_or_else(|| anyhow!("项目未发现 {} 设计目标", platform.as_str()))?;
    let route = clean_route(
        arguments
            .get("route")
            .and_then(Value::as_str)
            .unwrap_or("/"),
    )?;
    let url = arguments
        .get("url")
        .and_then(Value::as_str)
        .map(sanitize_session_url)
        .transpose()?;
    let viewport = viewport(arguments.get("viewport"), platform)?;
    let runtime_connected = session.view().await.connected;
    let state = if platform == DesignPlatform::Android && !runtime_connected {
        "PREPARATION_REQUIRED"
    } else {
        "READY_FOR_CAPTURE"
    };
    let now = chrono::Utc::now().to_rfc3339();
    let record = DesignSessionRecord {
        schema_version: 1,
        design_session_id: format!("design_{}", uuid::Uuid::new_v4().simple()),
        mcp_session_id: session.id.clone(),
        platform,
        target,
        route,
        url,
        viewport,
        state: state.to_string(),
        last_evidence: None,
        created_at: now.clone(),
        updated_at: now,
    };
    persist_record(&root, &record)?;
    Ok(json!({
        "session":record,
        "next": if platform == DesignPlatform::Android && !runtime_connected {
            "ui_prepare_debug_runtime"
        } else {
            CAPTURE_TOOL
        },
    }))
}

async fn capture(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let design_session_id = required_design_session_id(arguments)?;
    let mut record = read_record(&root, design_session_id)?;
    ensure_mcp_session(session, &record)?;
    if record.platform == DesignPlatform::Android {
        return Ok(json!({
            "designSessionId":design_session_id,
            "platform":"android",
            "status":"PREPARATION_REQUIRED",
            "next":["ui_get_runtime_status","ui_prepare_debug_runtime","ui_get_screen_summary","ui_get_current_crop"],
            "message":"Android 继续复用真实 Runtime；后台会话不会用浏览器画面冒充 Android。"
        }));
    }
    let capture_arguments = arguments
        .get("capture")
        .cloned()
        .ok_or_else(|| anyhow!("Web/PWA/Tauri 后台捕获缺少 capture"))?;
    let root_text = root.to_string_lossy().to_string();
    let mut result =
        crate::node_agent_pwa_runtime::capture_tool(Some(&root_text), capture_arguments).await;
    if result.get("ok").and_then(Value::as_bool) == Some(true) {
        record.last_evidence = Some(compact_evidence(&result));
        record.state = "CAPTURED".to_string();
        record.updated_at = chrono::Utc::now().to_rfc3339();
        persist_record(&root, &record)?;
    }
    result["designSessionId"] = json!(design_session_id);
    result["platform"] = json!(record.platform);
    result["hostCoverage"] = json!(host_coverage(record.platform));
    result["nativeHostVerified"] = json!(record.platform != DesignPlatform::Tauri);
    Ok(result)
}

fn get_surface(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let design_session_id = required_design_session_id(arguments)?;
    let record = read_record(&root, design_session_id)?;
    ensure_mcp_session(session, &record)?;
    let Some(evidence) = record.last_evidence.as_ref() else {
        return Ok(json!({
            "session":record,
            "status":"AWAITING_CAPTURE",
            "nodes":[],
            "next":if record.platform == DesignPlatform::Android {"ui_get_screen_summary"} else {CAPTURE_TOOL},
        }));
    };
    let tree = read_verified_tree(&root, evidence)?;
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(40)
        .clamp(1, 80) as usize;
    let all_nodes = tree
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let matching = all_nodes
        .iter()
        .filter(|node| query.is_empty() || node_matches(node, &query))
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "session":record,
        "status":"CAPTURED",
        "surface":{
            "title":tree.get("title"),"route":tree.get("route"),"viewport":tree.get("viewport"),
            "nodeCount":tree.get("nodeCount"),"interactiveCount":tree.get("interactiveCount"),
            "treeTruncated":tree.get("truncated"),"returnedNodeCount":matching.len(),
            "query":query,"returnTruncated":matching.len() < all_nodes.len(),
        },
        "nodes":matching,
        "pixels":evidence.get("artifact"),
        "uiTree":evidence.get("uiTree"),
        "base64Embedded":false,
    }))
}

fn compact_evidence(value: &Value) -> Value {
    json!({
        "status":value.get("status"),"artifact":value.get("artifact"),"uiTree":value.get("uiTree"),
        "route":value.get("route"),"revision":value.get("revision"),"viewport":value.get("viewport"),
        "browser":value.get("browser"),"contextPackReference":value.get("contextPackReference"),
    })
}

fn read_verified_tree(root: &Path, evidence: &Value) -> Result<Value> {
    let path = evidence
        .pointer("/uiTree/path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("后台设计证据缺少 UI tree path"))?;
    let expected = evidence
        .pointer("/uiTree/sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("后台设计证据缺少 UI tree sha256"))?;
    let path = PathBuf::from(path)
        .canonicalize()
        .context("后台设计 UI tree 工件不存在")?;
    if !path.starts_with(root) || fs::metadata(&path)?.len() > MAX_TREE_BYTES {
        bail!("后台设计 UI tree 越出项目或超过大小上限");
    }
    let bytes = fs::read(path)?;
    if !expected.eq_ignore_ascii_case(&hex::encode(Sha256::digest(&bytes))) {
        bail!("后台设计 UI tree 哈希不匹配");
    }
    serde_json::from_slice(&bytes).context("后台设计 UI tree JSON 无效")
}

fn node_matches(node: &Value, query: &str) -> bool {
    ["selector", "role", "label", "tag"]
        .iter()
        .filter_map(|key| node.get(key).and_then(Value::as_str))
        .any(|value| value.to_ascii_lowercase().contains(query))
}

fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("后台设计 MCP 未绑定项目目录"))?;
    PathBuf::from(root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {root}"))
}

fn persist_record(root: &Path, record: &DesignSessionRecord) -> Result<()> {
    let path = record_path(root, &record.design_session_id, true)?;
    fs::write(path, serde_json::to_vec_pretty(record)?)?;
    Ok(())
}

fn read_record(root: &Path, id: &str) -> Result<DesignSessionRecord> {
    let path = record_path(root, id, false)?;
    serde_json::from_slice(&fs::read(path)?).context("后台设计会话 JSON 无效")
}

fn record_path(root: &Path, id: &str, create: bool) -> Result<PathBuf> {
    validate_design_session_id(id)?;
    let directory = root.join(".elon/ui-tuner/headless-design/sessions");
    if create {
        fs::create_dir_all(&directory)?;
    }
    let canonical = directory.canonicalize().context("后台设计会话目录不存在")?;
    if !canonical.starts_with(root) {
        bail!("后台设计会话目录越出项目");
    }
    Ok(canonical.join(format!("{id}.json")))
}

fn required_design_session_id(arguments: &Value) -> Result<&str> {
    let value = required_string(arguments, "designSessionId")?;
    validate_design_session_id(value)?;
    Ok(value)
}

fn validate_design_session_id(value: &str) -> Result<()> {
    if value.len() != 39
        || !value.starts_with("design_")
        || !value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("designSessionId 无效");
    }
    Ok(())
}

fn ensure_mcp_session(session: &LiveUiSession, record: &DesignSessionRecord) -> Result<()> {
    if record.mcp_session_id != session.id {
        bail!("后台设计会话不属于当前 MCP session");
    }
    Ok(())
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 {key}"))
}

fn clean_route(value: &str) -> Result<String> {
    let route = value.trim();
    if !route.starts_with('/') || route.chars().count() > 2048 {
        bail!("route 必须是 1..2048 字符的项目内绝对路径");
    }
    Ok(route.to_string())
}

fn sanitize_session_url(value: &str) -> Result<String> {
    let url = reqwest::Url::parse(value).context("后台设计 URL 无效")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("后台设计 URL 只允许不含凭据的 http(s)");
    }
    let query = url.query().unwrap_or_default().to_ascii_lowercase();
    if [
        "token",
        "secret",
        "password",
        "authorization",
        "signature",
        "api_key",
    ]
    .iter()
    .any(|marker| query.contains(marker))
    {
        bail!("后台设计 URL query 疑似包含秘密");
    }
    Ok(value.to_string())
}

fn viewport(value: Option<&Value>, platform: DesignPlatform) -> Result<Value> {
    let defaults = match platform {
        DesignPlatform::Pwa | DesignPlatform::Android => (390, 844),
        DesignPlatform::Web | DesignPlatform::Tauri => (1280, 800),
    };
    let width = value
        .and_then(|value| value.get("width"))
        .and_then(Value::as_u64)
        .unwrap_or(defaults.0);
    let height = value
        .and_then(|value| value.get("height"))
        .and_then(Value::as_u64)
        .unwrap_or(defaults.1);
    let scale = value
        .and_then(|value| value.get("deviceScaleFactor"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    if !(240..=4096).contains(&width)
        || !(240..=4096).contains(&height)
        || !(0.5..=4.0).contains(&scale)
    {
        bail!("viewport 超过后台设计安全范围");
    }
    Ok(json!({"width":width,"height":height,"deviceScaleFactor":scale}))
}

fn host_coverage(platform: DesignPlatform) -> &'static str {
    match platform {
        DesignPlatform::Web => "BROWSER_RUNTIME",
        DesignPlatform::Pwa => "PWA_RUNTIME",
        DesignPlatform::Tauri => "TAURI_FRONTEND_WEBVIEW_ONLY",
        DesignPlatform::Android => "ANDROID_RUNTIME",
    }
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({
        "name":name,"description":description,"inputSchema":input_schema,
        "annotations":{"readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":read_only,"openWorldHint":false}
    })
}
