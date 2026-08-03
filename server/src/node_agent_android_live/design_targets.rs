use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::broker::LiveUiSession;
use super::design_session_store::{
    list_records, persist_record, read_record, read_verified_pixels, read_verified_tree,
    validate_design_session_id, DesignSessionRecord, VerifiedPixelArtifact,
};

const LIST_TOOL: &str = "ui_list_design_targets";
const LIST_SESSIONS_TOOL: &str = "ui_list_design_sessions";
const OPEN_TOOL: &str = "ui_open_design_target";
const CAPTURE_TOOL: &str = "ui_capture_design_surface";
const GET_TOOL: &str = "ui_get_design_surface";

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

pub(super) fn tool_definitions() -> Vec<Value> {
    let mut capture_schema = crate::node_agent_pwa_runtime::tool_definition()
        .get("inputSchema")
        .cloned()
        .unwrap_or_else(|| json!({"type":"object"}));
    if let Some(schema) = capture_schema.as_object_mut() {
        schema.remove("required");
    }
    vec![
        tool(
            LIST_TOOL,
            "发现项目可由 AI 后台设计的 Web、PWA、Tauri 和 Android 目标；只返回小型技术栈与适配器索引，不读取页面正文。",
            json!({"type":"object","additionalProperties":false,"properties":{}}),
            true,
        ),
        tool(
            LIST_SESSIONS_TOOL,
            "列出项目最近的后台设计会话，使 AI 与 PC 画布可恢复同一 designSessionId；只返回小型会话摘要和工件引用状态。",
            json!({"type":"object","additionalProperties":false,"properties":{
                "limit":{"type":"integer","minimum":1,"maximum":50,"default":20}
            }}),
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
            "在后台设计会话中执行受限点击/等待/文本断言并捕获 PNG 与 UI 语义树。open 时已保存 url/viewport，可只传 designSessionId；Tauri 本工具只证明 WebView，原生窗口另用 ui_prepare_tauri_runtime/ui_capture_tauri_host。",
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
    matches!(
        name,
        LIST_TOOL | LIST_SESSIONS_TOOL | OPEN_TOOL | CAPTURE_TOOL | GET_TOOL
    )
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        LIST_TOOL => list(session),
        LIST_SESSIONS_TOOL => list_sessions(session, &arguments),
        OPEN_TOOL => open(session, &arguments).await,
        CAPTURE_TOOL => capture(session, &arguments).await,
        GET_TOOL => get_surface(session, &arguments),
        _ => bail!("未知后台设计工具: {name}"),
    }
}

pub(super) fn pixel_artifact(
    session: &LiveUiSession,
    design_session_id: &str,
) -> Result<VerifiedPixelArtifact> {
    let root = canonical_project_root(session)?;
    let record = read_record(&root, design_session_id)?;
    read_verified_pixels(&root, &record)
}

fn list(session: &LiveUiSession) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let (targets, inspected, truncated) = super::design_target_discovery::discover_targets(&root)?;
    Ok(json!({
        "schemaVersion":1,
        "targets":targets,
        "scan":{"filesInspected":inspected,"truncated":truncated,"contentEmbedded":false},
        "defaultWorkflow":[LIST_SESSIONS_TOOL, OPEN_TOOL, CAPTURE_TOOL, GET_TOOL],
    }))
}

fn list_sessions(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 50) as usize;
    let (records, invalid) = list_records(&root, limit)?;
    let sessions = records.iter().map(session_summary).collect::<Vec<_>>();
    Ok(json!({
        "schemaVersion":1,
        "sessions":sessions,
        "invalidRecordCount":invalid,
        "contentEmbedded":false,
    }))
}

fn session_summary(record: &DesignSessionRecord) -> Value {
    let native_host = native_host_evidence(record.last_evidence.as_ref());
    json!({
        "designSessionId":record.design_session_id,
        "platform":record.platform,
        "label":record.target.label,
        "adapter":record.target.adapter,
        "evidenceLevel":record.target.evidence_level,
        "nativeHostVerified":record.target.native_host_verified || native_host.is_some(),
        "nativeHost":native_host,
        "route":record.route,
        "url":record.url,
        "viewport":record.viewport,
        "state":record.state,
        "hasEvidence":record.last_evidence.is_some(),
        "pixels":record.last_evidence.as_ref().and_then(|value| value.get("artifact")),
        "uiTree":record.last_evidence.as_ref().and_then(|value| value.get("uiTree")),
        "createdAt":record.created_at,
        "updatedAt":record.updated_at,
    })
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
    if record.platform == DesignPlatform::Android {
        return Ok(json!({
            "designSessionId":design_session_id,
            "platform":"android",
            "status":"PREPARATION_REQUIRED",
            "next":["ui_get_runtime_status","ui_prepare_debug_runtime","ui_get_screen_summary","ui_get_current_crop"],
            "message":"Android 继续复用真实 Runtime；后台会话不会用浏览器画面冒充 Android。"
        }));
    }
    let capture_arguments = capture_arguments(&record, arguments)?;
    let root_text = root.to_string_lossy().to_string();
    let mut result =
        crate::node_agent_pwa_runtime::capture_tool(Some(&root_text), capture_arguments).await;
    if result.get("ok").and_then(Value::as_bool) == Some(true) {
        let native_host = native_host_evidence(record.last_evidence.as_ref()).cloned();
        let mut evidence = compact_evidence(&result);
        if let Some(native_host) = native_host {
            evidence["nativeHost"] = native_host;
        }
        record.last_evidence = Some(evidence);
        record.state = "CAPTURED".to_string();
        record.updated_at = chrono::Utc::now().to_rfc3339();
        persist_record(&root, &record)?;
    }
    result["designSessionId"] = json!(design_session_id);
    result["platform"] = json!(record.platform);
    result["hostCoverage"] = json!(host_coverage(record.platform));
    result["nativeHostVerified"] = json!(
        record.platform != DesignPlatform::Tauri
            || native_host_evidence(record.last_evidence.as_ref()).is_some()
    );
    Ok(result)
}

fn get_surface(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_project_root(session)?;
    let design_session_id = required_design_session_id(arguments)?;
    let record = read_record(&root, design_session_id)?;
    let Some(evidence) = record.last_evidence.as_ref() else {
        return Ok(json!({
            "session":record,
            "status":"AWAITING_CAPTURE",
            "nodes":[],
            "next":if record.platform == DesignPlatform::Android {"ui_get_screen_summary"} else {CAPTURE_TOOL},
        }));
    };
    let native_host = evidence.get("nativeHost").cloned();
    if evidence.pointer("/uiTree/path").is_none() {
        return Ok(json!({
            "session":record,"status":"NATIVE_CAPTURED","nodes":[],
            "nativeHost":native_host,"nativeHostVerified":native_host.is_some(),
            "next":CAPTURE_TOOL,"base64Embedded":false,
        }));
    }
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
        "nativeHost":native_host,
        "nativeHostVerified":native_host.is_some() || record.target.native_host_verified,
        "base64Embedded":false,
    }))
}

fn capture_arguments(record: &DesignSessionRecord, arguments: &Value) -> Result<Value> {
    let mut capture = arguments
        .get("capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let input = capture
        .as_object_mut()
        .ok_or_else(|| anyhow!("capture 必须是对象"))?;
    if !input.contains_key("url") {
        input.insert(
            "url".to_string(),
            json!(record
                .url
                .as_deref()
                .ok_or_else(|| anyhow!("后台设计会话没有 URL；请重新 open 并提供 url"))?),
        );
    }
    input
        .entry("viewport".to_string())
        .or_insert_with(|| record.viewport.clone());
    input.entry("evidence".to_string()).or_insert_with(|| {
        json!({
            "sourceRevision":"design-session-source-unverified",
            "routeRevision":format!("design-session:{}", record.design_session_id),
        })
    });
    Ok(capture)
}

fn compact_evidence(value: &Value) -> Value {
    json!({
        "status":value.get("status"),"artifact":value.get("artifact"),"uiTree":value.get("uiTree"),
        "route":value.get("route"),"revision":value.get("revision"),"viewport":value.get("viewport"),
        "browser":value.get("browser"),"contextPackReference":value.get("contextPackReference"),
    })
}

fn native_host_evidence(evidence: Option<&Value>) -> Option<&Value> {
    evidence.and_then(|value| value.get("nativeHost"))
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

fn required_design_session_id(arguments: &Value) -> Result<&str> {
    let value = required_string(arguments, "designSessionId")?;
    validate_design_session_id(value)?;
    Ok(value)
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
