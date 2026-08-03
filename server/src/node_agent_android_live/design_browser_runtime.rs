use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use super::{
    broker::LiveUiSession,
    design_session_store::{
        persist_record, read_record, validate_design_session_id, DesignSessionRecord,
    },
    design_targets::DesignPlatform,
};

const PREPARE_TOOL: &str = "ui_prepare_design_browser";
const INTERACT_TOOL: &str = "ui_interact_design_browser";
const STOP_TOOL: &str = "ui_stop_design_browser";

pub(super) fn tool_definitions() -> Vec<Value> {
    let mut capture_schema = crate::node_agent_pwa_runtime::tool_definition()
        .get("inputSchema")
        .cloned()
        .unwrap_or_else(|| json!({"type":"object"}));
    if let Some(schema) = capture_schema.as_object_mut() {
        schema.remove("required");
        if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            properties.remove("url");
            properties.remove("viewport");
            properties.remove("evidence");
        }
    }
    vec![
        tool(
            PREPARE_TOOL,
            "为 Web/PWA/Tauri WebView designSession 创建有界持久浏览器并立即捕获；保留页面、Cookie、localStorage、滚动和组件状态，最多 4 会话、空闲 15 分钟、生命周期 60 分钟或 128 次操作。",
            json!({"type":"object","additionalProperties":false,"required":["designSessionId"],"properties":{
                "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
                "restart":{"type":"boolean","default":false},"capture":capture_schema
            }}),
            false,
        ),
        tool(
            INTERACT_TOOL,
            "在同一持久 designSession 浏览器中执行安全交互并重新捕获；表单值只能引用已审查 fixtureProfile.formValues，不接受任意脚本或 MCP 明文秘密。",
            json!({"type":"object","additionalProperties":false,"required":["designSessionId"],"properties":{
                "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
                "navigateTo":{"type":"string","minLength":1,"maxLength":4096,"description":"可选同 origin http(s) URL；省略时保留当前页面"},
                "capture":capture_schema
            }}),
            false,
        ),
        tool(
            STOP_TOOL,
            "停止当前 designSession 登记的持久浏览器并回收进程树和临时 profile，不影响其他会话。",
            json!({"type":"object","additionalProperties":false,"required":["designSessionId"],"properties":{
                "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"}
            }}),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, PREPARE_TOOL | INTERACT_TOOL | STOP_TOOL)
}

pub(super) async fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        PREPARE_TOOL => prepare(session, &arguments).await,
        INTERACT_TOOL => interact(session, &arguments).await,
        STOP_TOOL => stop(session, &arguments).await,
        _ => bail!("未知持久设计浏览器工具: {name}"),
    }
}

async fn prepare(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = browser_record(session, arguments)?;
    let capture = capture_input(&record, arguments, None)?;
    let input = serde_json::from_value(capture).context("持久浏览器 capture 参数无效")?;
    let restart = arguments
        .get("restart")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let root_text = root.to_string_lossy().to_string();
    let result = crate::node_agent_pwa_runtime::start_stateful_browser(
        &record.design_session_id,
        &root_text,
        input,
        restart,
    )
    .await;
    persist_capture(&root, &mut record, &result, "BROWSER_RUNTIME_READY")?;
    Ok(with_session(result, &record))
}

async fn interact(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = browser_record(session, arguments)?;
    let navigate_to = arguments
        .get("navigateTo")
        .and_then(Value::as_str)
        .map(sanitize_url)
        .transpose()?;
    let capture = capture_input(&record, arguments, navigate_to.as_deref())?;
    let input = serde_json::from_value(capture).context("持久浏览器交互参数无效")?;
    let root_text = root.to_string_lossy().to_string();
    let result = crate::node_agent_pwa_runtime::interact_stateful_browser(
        &record.design_session_id,
        &root_text,
        input,
        navigate_to.is_some(),
    )
    .await;
    if let Some(url) = navigate_to {
        if result.get("ok").and_then(Value::as_bool) == Some(true) {
            record.url = Some(url);
            if let Some(path) = result.pointer("/route/path").and_then(Value::as_str) {
                record.route = path.to_string();
            }
        }
    }
    persist_capture(&root, &mut record, &result, "BROWSER_RUNTIME_CAPTURED")?;
    Ok(with_session(result, &record))
}

async fn stop(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let (root, mut record) = browser_record(session, arguments)?;
    let result =
        crate::node_agent_pwa_runtime::stop_stateful_browser(&record.design_session_id).await;
    record.state = if result.get("status").and_then(Value::as_str) == Some("STOPPED") {
        "BROWSER_RUNTIME_STOPPED"
    } else {
        "READY_FOR_CAPTURE"
    }
    .into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(&root, &record)?;
    Ok(with_session(result, &record))
}

fn browser_record(
    session: &LiveUiSession,
    arguments: &Value,
) -> Result<(PathBuf, DesignSessionRecord)> {
    let id = arguments
        .get("designSessionId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 designSessionId"))?;
    validate_design_session_id(id)?;
    let root = PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("持久设计浏览器需要绑定项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")?;
    let record = read_record(&root, id)?;
    if record.platform == DesignPlatform::Android {
        bail!("BROWSER_DESIGN_SESSION_REQUIRED：Android 必须继续使用 Live Runtime");
    }
    Ok((root, record))
}

fn capture_input(
    record: &DesignSessionRecord,
    arguments: &Value,
    navigate_to: Option<&str>,
) -> Result<Value> {
    let mut capture = arguments
        .get("capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let input = capture.as_object_mut().context("capture 必须是对象")?;
    input.insert(
        "url".into(),
        json!(navigate_to
            .or(record.url.as_deref())
            .context("designSession 没有 URL；请重新 open")?),
    );
    input.insert("viewport".into(), record.viewport.clone());
    input.insert(
        "evidence".into(),
        json!({"sourceRevision":"design-session-source-unverified",
            "routeRevision":format!("stateful:{}",record.design_session_id)}),
    );
    Ok(capture)
}

fn persist_capture(
    root: &std::path::Path,
    record: &mut DesignSessionRecord,
    result: &Value,
    state: &str,
) -> Result<()> {
    if result.get("ok").and_then(Value::as_bool) != Some(true) || result.get("artifact").is_none() {
        return Ok(());
    }
    let native_host = record
        .last_evidence
        .as_ref()
        .and_then(|value| value.get("nativeHost"))
        .cloned();
    let native_behavior = record
        .last_evidence
        .as_ref()
        .and_then(|value| value.get("nativeBehavior"))
        .cloned();
    let mut evidence = json!({
        "status":result.get("status"),"artifact":result.get("artifact"),
        "uiTree":result.get("uiTree"),"route":result.get("route"),
        "revision":result.get("revision"),"viewport":result.get("viewport"),
        "browser":result.get("browser"),"browserRuntime":result.get("runtime"),
        "contextPackReference":result.get("contextPackReference"),
    });
    if let Some(value) = native_host {
        evidence["nativeHost"] = value;
    }
    if let Some(value) = native_behavior {
        evidence["nativeBehavior"] = value;
    }
    record.last_evidence = Some(evidence);
    record.state = state.into();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    persist_record(root, record)
}

fn sanitize_url(value: &str) -> Result<String> {
    let url = reqwest::Url::parse(value).context("navigateTo URL 无效")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("navigateTo 只允许不含凭据的 http(s) URL");
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
        bail!("navigateTo query 疑似包含秘密");
    }
    Ok(value.to_string())
}

fn with_session(mut result: Value, record: &DesignSessionRecord) -> Value {
    result["designSessionId"] = json!(record.design_session_id);
    result["platform"] = json!(record.platform);
    result["statePreserved"] = json!(result.get("runtime").is_some());
    result
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,
        "annotations":{"readOnlyHint":read_only,"destructiveHint":false,
            "idempotentHint":read_only,"openWorldHint":false}})
}
