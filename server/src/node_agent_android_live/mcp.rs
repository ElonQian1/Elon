use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use super::broker::{LiveUiBroker, LiveUiSession};
use super::mcp_tools::tool_definitions;
use super::protocol::{LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch};
use super::source_commit::{build_source_commit_plan, commit_source, SourceCommitRequest};
use super::ui_ir::{load_or_build_ui_ir, UiIrDocument};
use super::visual_diff::{compare_images, PixelRect, VisualDiffRequest};
use super::visual_solver::{solve_visual_style, VisualSolverRequest};

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const MAX_SOURCE_CHARS: usize = 64_000;

#[derive(Debug, Deserialize)]
pub(crate) struct McpQuery {
    #[serde(rename = "token")]
    pub(crate) token: String,
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

pub(crate) fn descriptor(session: &LiveUiSession, host_port: u16) -> Result<Value> {
    let url = format!(
        "http://127.0.0.1:{host_port}/api/android-live/mcp/{}?token={}",
        session.id, session.token
    );
    let config_path = std::env::temp_dir()
        .join("elon-ui-tuner-live")
        .join(&session.id)
        .join("mcp.json");
    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow!("无法确定 ui-tuner MCP 配置目录"))?;
    fs::create_dir_all(config_dir)
        .with_context(|| format!("创建 ui-tuner MCP 配置目录失败: {}", config_dir.display()))?;
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&json!({
            "mcpServers": {
                "yilong_ui_live": {
                    "url": url,
                    "required": false,
                    "toolTimeoutSec": 60
                }
            }
        }))?,
    )
    .with_context(|| format!("写入 ui-tuner MCP 配置失败: {}", config_path.display()))?;
    Ok(json!({
        "name": "yilong-ui-live",
        "transport": "streamable-http",
        "configPath": config_path.display().to_string(),
        "sessionId": session.id,
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "purpose": "Codex 按需读取当前真机 UI IR、目标设计图、局部源码与视觉差异；避免把整棵树反复写入提示词。",
    }))
}

pub(crate) fn cleanup_descriptor(session_id: &str) {
    let path = std::env::temp_dir()
        .join("elon-ui-tuner-live")
        .join(session_id);
    let _ = fs::remove_dir_all(path);
}

pub(crate) async fn handle_request(
    broker: &LiveUiBroker,
    session_id: &str,
    request: McpRequest,
) -> Value {
    let id = request.id.clone().unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "yilong-ui-live", "version": "1.0.0" },
            "instructions": "先读 ui_get_screen_summary；仅在需要时读取节点、局部源码和裁剪。LIVE 数值优先使用 ui_propose_live_patch/ui_apply_live_patch，结构修改才编辑源码。"
        })),
        "notifications/initialized" => return Value::Null,
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(broker, session_id, request.params).await,
        "ping" => Ok(json!({})),
        _ => Err(anyhow!("不支持 MCP method: {}", request.method)),
    };
    match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": format!("{error:#}") }
        }),
    }
}

async fn call_tool(broker: &LiveUiBroker, session_id: &str, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = match name {
        "ui_get_screen_summary" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            json!({
                "sessionId": ir.session_id,
                "revision": ir.revision,
                "treeRevision": ir.tree_revision,
                "screen": ir.summary,
                "snapshot": ir.snapshot,
                "targetDesign": ir.target_design,
                "selectedRuntimeNodeId": ir.selected_runtime_node_id,
            })
        }
        "ui_get_node" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            let node = find_node(&ir, &arguments)?;
            json!({ "revision": ir.revision, "node": node })
        }
        "ui_get_subtree" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            let node = find_node(&ir, &arguments)?;
            let mut ids = vec![node.runtime_node_id.clone()];
            collect_descendants(&ir, &mut ids);
            let nodes = ir
                .nodes
                .into_iter()
                .filter(|node| ids.contains(&node.runtime_node_id))
                .collect::<Vec<_>>();
            json!({ "revision": ir.revision, "nodes": nodes })
        }
        "ui_get_source_bundle" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            source_bundle(&ir, &arguments)?
        }
        "ui_get_target_crop" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            let target = ir
                .target_design
                .ok_or_else(|| anyhow!("尚未绑定目标设计图"))?;
            json!({ "path": target.path, "sha256": target.sha256, "rect": rect_argument(&arguments) })
        }
        "ui_get_current_crop" => {
            let ir = load_or_build_ui_ir(broker, session_id).await?;
            let snapshot = ir.snapshot.ok_or_else(|| anyhow!("尚未绑定当前真机截图"))?;
            json!({
                "path": snapshot.screenshot_path,
                "sha256": snapshot.screenshot_sha256,
                "rect": rect_argument(&arguments),
                "snapshotId": snapshot.snapshot_id,
            })
        }
        "ui_get_visual_diff" => visual_diff_from_ir(broker, session_id, &arguments).await?,
        "ui_propose_live_patch" => propose_live_patch(broker, session_id, &arguments).await?,
        "ui_apply_live_patch" => {
            let patch: LiveStylePatch =
                serde_json::from_value(arguments.get("patch").cloned().unwrap_or(arguments))
                    .context("patch 参数不符合 LiveStylePatch")?;
            let ack = broker.apply_patch(session_id, patch).await?;
            json!({ "ack": ack })
        }
        "ui_run_visual_solver" => {
            let request: VisualSolverRequest =
                serde_json::from_value(arguments).context("视觉求解参数无效")?;
            json!({ "result": solve_visual_style(broker, session_id, request).await? })
        }
        "ui_get_commit_plan" => {
            let session = broker.session(session_id).await?;
            json!({ "plan": build_source_commit_plan(session).await? })
        }
        "ui_commit_bound_styles" => {
            let source_revision = arguments
                .get("sourceRevision")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("缺少 sourceRevision"))?
                .to_string();
            let session = broker.session(session_id).await?;
            json!({
                "result": commit_source(session, SourceCommitRequest { source_revision }).await?
            })
        }
        "ui_build_and_verify" => json!({
            "status": "COMPILE_REQUIRED",
            "message": "源码保存后的自动构建、安装、清空 Patch 与重新捕获由 P10 验证编排器执行；当前可继续使用现有构建验收链路。"
        }),
        _ => bail!("未知 UI MCP 工具: {name}"),
    };
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value)? }],
        "structuredContent": value,
        "isError": false,
    }))
}

fn find_node<'a>(
    ir: &'a UiIrDocument,
    arguments: &Value,
) -> Result<&'a super::protocol::LiveUiNode> {
    let runtime_id = arguments.get("runtimeNodeId").and_then(Value::as_str);
    let definition_id = arguments.get("definitionId").and_then(Value::as_str);
    ir.nodes
        .iter()
        .find(|node| {
            runtime_id == Some(node.runtime_node_id.as_str())
                || definition_id == Some(node.definition_id.as_str())
        })
        .ok_or_else(|| anyhow!("找不到指定 UI 节点"))
}

fn collect_descendants(ir: &UiIrDocument, ids: &mut Vec<String>) {
    loop {
        let before = ids.len();
        for node in &ir.nodes {
            if node
                .parent_runtime_node_id
                .as_ref()
                .map(|parent| ids.contains(parent))
                .unwrap_or(false)
                && !ids.contains(&node.runtime_node_id)
            {
                ids.push(node.runtime_node_id.clone());
            }
        }
        if ids.len() == before {
            break;
        }
    }
}

fn source_bundle(ir: &UiIrDocument, arguments: &Value) -> Result<Value> {
    let node = find_node(ir, arguments)?;
    let root = ir
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("Live 会话未绑定项目目录"))?;
    let root = PathBuf::from(root)
        .canonicalize()
        .context("项目目录不存在")?;
    let mut candidates = ir.source_candidates.clone();
    if let Some(source) = &node.source {
        candidates.insert(0, source.clone());
    }
    let mut snippets = Vec::new();
    let mut total = 0;
    for candidate in candidates.into_iter().take(8) {
        let Some(file) = candidate.get("file").and_then(Value::as_str) else {
            continue;
        };
        let path = safe_source_path(&root, file)?;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取源码失败: {}", path.display()))?;
        let line = candidate.get("line").and_then(Value::as_u64).unwrap_or(1) as usize;
        let snippet = line_window(&content, line, 80);
        if total + snippet.len() > MAX_SOURCE_CHARS {
            break;
        }
        total += snippet.len();
        snippets.push(json!({
            "file": path.strip_prefix(&root).unwrap_or(&path).display().to_string(),
            "line": line,
            "reason": candidate.get("reason"),
            "confidence": candidate.get("confidence"),
            "content": snippet,
        }));
    }
    Ok(json!({
        "runtimeNodeId": node.runtime_node_id,
        "definitionId": node.definition_id,
        "snippets": snippets,
        "sourceCandidates": ir.source_candidates,
        "truncated": total >= MAX_SOURCE_CHARS,
    }))
}

async fn visual_diff_from_ir(
    broker: &LiveUiBroker,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let target = ir
        .target_design
        .ok_or_else(|| anyhow!("尚未绑定目标设计图"))?;
    let snapshot = ir.snapshot.ok_or_else(|| anyhow!("尚未绑定当前真机截图"))?;
    let request = VisualDiffRequest {
        target_path: target.path,
        current_path: snapshot.screenshot_path,
        target_rect: parse_rect(arguments.get("targetRect"))?,
        current_rect: parse_rect(arguments.get("currentRect"))?,
    };
    Ok(json!({ "diff": compare_images(&request)? }))
}

async fn propose_live_patch(
    broker: &LiveUiBroker,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let node = find_node(&ir, arguments)?;
    let target =
        parse_rect(arguments.get("targetRect"))?.ok_or_else(|| anyhow!("缺少 targetRect"))?;
    let current = &node.geometry.bounds_in_display_px;
    let density = node.geometry.density.max(0.01) as f64;
    let mut operations = Vec::new();
    if node
        .properties
        .get("width")
        .map(|p| p.change_level.as_str())
        == Some("LIVE")
    {
        operations.push(numeric_operation(
            "width",
            (target.right - target.left) as f64 / density,
        ));
    }
    if node
        .properties
        .get("height")
        .map(|p| p.change_level.as_str())
        == Some("LIVE")
    {
        operations.push(numeric_operation(
            "height",
            (target.bottom - target.top) as f64 / density,
        ));
    }
    if node
        .properties
        .get("translationX")
        .map(|p| p.change_level.as_str())
        == Some("LIVE")
    {
        operations.push(numeric_operation(
            "translationX",
            (target.left - current.left) as f64 / density,
        ));
    }
    if node
        .properties
        .get("translationY")
        .map(|p| p.change_level.as_str())
        == Some("LIVE")
    {
        operations.push(numeric_operation(
            "translationY",
            (target.top - current.top) as f64 / density,
        ));
    }
    if operations.is_empty() {
        bail!("当前节点没有可映射目标几何的 LIVE 属性");
    }
    Ok(json!({
        "patch": LiveStylePatch {
            protocol_version: 1,
            message_type: "patch.apply".to_string(),
            session_id: session_id.to_string(),
            request_id: String::new(),
            gesture_id: Some("ai-geometry-proposal".to_string()),
            sequence: 0,
            base_tree_revision: Some(ir.tree_revision),
            target: LivePatchTarget {
                scope: "INSTANCE".to_string(),
                runtime_node_id: Some(node.runtime_node_id.clone()),
                definition_id: Some(node.definition_id.clone()),
                instance_key: node.instance_key.clone(),
            },
            atomic: true,
            ephemeral: true,
            operations,
        }
    }))
}

fn numeric_operation(property: &str, value: f64) -> LivePatchOperation {
    LivePatchOperation {
        property: property.to_string(),
        value: LivePropertyValue {
            value_type: "dp".to_string(),
            value: json!((value * 1000.0).round() / 1000.0),
        },
    }
}

fn rect_argument(arguments: &Value) -> Value {
    arguments.get("rect").cloned().unwrap_or(Value::Null)
}

fn parse_rect(value: Option<&Value>) -> Result<Option<PixelRect>> {
    value
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value(value.clone()).context("rect 参数无效"))
        .transpose()
}

fn safe_source_path(root: &Path, file: &str) -> Result<PathBuf> {
    let joined = if Path::new(file).is_absolute() {
        PathBuf::from(file)
    } else {
        root.join(file)
    };
    let canonical = joined
        .canonicalize()
        .with_context(|| format!("源码路径不存在: {}", joined.display()))?;
    if !canonical.starts_with(root) {
        bail!("拒绝读取项目目录之外的源码");
    }
    Ok(canonical)
}

fn line_window(content: &str, line: usize, radius: usize) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    let start = line.saturating_sub(radius / 2 + 1);
    let end = (line + radius / 2).min(lines.len());
    lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, text)| format!("{:>5} | {text}", start + offset + 1))
        .collect::<Vec<_>>()
        .join("\n")
}
