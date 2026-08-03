use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::broker::{LiveUiBroker, LiveUiSession};
use super::build_verify::BuildVerifyRequest;
use super::fit_run::{
    workspace_fingerprint, AttachStateReplayRequest, FitRunService, FitSessionContext,
    FitStateReplay,
};
use super::frame_artifact::{capture_latest_frame_artifact, persist_target_crop_artifact};
use super::mcp_fit_command::fit_command;
use super::mcp_tools::tool_definitions;
use super::preview::{activate_preview_scenario, PreviewOpenRequest};
use super::protocol::{LivePatchOperation, LivePatchTarget, LivePropertyValue, LiveStylePatch};
use super::source_commit::{build_source_commit_plan, commit_source, SourceCommitRequest};
use super::ui_ir::{
    bind_ui_ir, load_or_build_ui_ir, persist_target_design, BindUiIrRequest, UiIrDocument,
};
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
        "purpose": "Codex 可在用户不打开画布时按需操作 Web、PWA、Tauri 前端与 Android 设计会话，并读取紧凑 UI tree、像素哈希、局部源码与视觉差异。",
    }))
}

pub(crate) async fn descriptor_for_project(
    broker: &LiveUiBroker,
    project_root: &str,
    host_port: u16,
) -> Result<Option<Value>> {
    let (session, runtime_binding) =
        super::runtime_binding::select_or_control(broker, project_root, host_port).await?;
    let mut value = descriptor(&session, host_port)?;
    value["runtimeBinding"] =
        super::runtime_binding::descriptor_view(project_root, runtime_binding);
    Ok(Some(value))
}

pub(crate) fn cleanup_descriptor(session_id: &str) {
    let path = std::env::temp_dir()
        .join("elon-ui-tuner-live")
        .join(session_id);
    let _ = fs::remove_dir_all(path);
}

pub(crate) async fn handle_request(
    broker: &Arc<LiveUiBroker>,
    fit_runs: &FitRunService,
    session_id: &str,
    request: McpRequest,
) -> Option<Value> {
    let id = request.id.clone().unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => initialize_response(),
        "notifications/initialized" => return None,
        "tools/list" => tools_list_response(),
        "tools/call" => call_tool(broker, fit_runs, session_id, request.params).await,
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

fn initialize_response() -> Result<Value> {
    let tools = tool_definitions();
    let tool_contract = super::mcp_tool_contract::manifest(&tools)?;
    Ok(json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "yilong-ui-live", "version": "1.8.0" },
        "toolContract": tool_contract,
        "instructions": "先调用 ui_get_design_capabilities 证明当前节点 schema；模糊路由任务再调用 ui_confirm_route。无需打开 PC 画布的多端任务按 ui_list_design_targets -> ui_list_design_sessions -> ui_open_design_target 工作；Web/PWA/Tauri 前端可用 ui_prepare_design_browser -> ui_interact_design_browser 持续边看边调，结束调用 ui_stop_design_browser。Tauri 原生证据按 ui_prepare_tauri_runtime -> ui_capture_tauri_host -> ui_capture_tauri_behavior -> ui_stop_tauri_runtime 分层采集；项目插桩 trace 不冒充操作系统证明。设计修改用 ui_create_design_draft 建立可撤销 patch；可用 ui_preview_design_draft / ui_restore_design_draft_preview 临时查看，预览不修改源码也不是完成证据。调用 ui_suggest_design_source_binding 获取有界候选，先保持 CANDIDATE，核对文件哈希和范围后才用 ui_update_design_draft 显式确认 BOUND；再 ui_begin_design_writeback 固定 Git 基线。源码修改与分平台验证后用 ui_complete_design_writeback 形成完成回执，并用 ui_get_design_verification_matrix 查看每个平台的证据缺口。后台证据只返回路径、哈希和紧凑当前状态，不嵌入 Base64/完整历史，不允许秘密表单输入、任意菜单点击或任意 Tauri command。Codex 桌面端 UI 请求先导入任务、读取项目与 Runtime 并检查能力；requiredCapabilities 只能追加系统推导能力。样式优先 Live Patch/FitRun，结构变化使用最小 CODEX_SOURCE_HANDOFF。平台缺口必须声明 deliveryImpact：原业务任务只创建 Worktree handoff，不得就地升级；DELIVERY_NON_BLOCKING 在 businessDeliveryReady=true 后先收尾业务，再由新的 Codex Desktop Worktree 任务后台升级、发布和复检，前台 UI 任务优先使用真机与节点发布资源；DELIVERY_BLOCKING 则暂停业务并分流。全新页面先建骨架和首次构建，再回到真实 Android Renderer。收尾必须调用 ui_check_workflow_completion；仅 completionReady 或经验证的 businessDeliveryReady 允许对应声明。"
    }))
}

fn tools_list_response() -> Result<Value> {
    let tools = tool_definitions();
    let tool_contract = super::mcp_tool_contract::manifest(&tools)?;
    Ok(json!({ "tools": tools, "toolContract": tool_contract }))
}

async fn call_tool(
    broker: &Arc<LiveUiBroker>,
    fit_runs: &FitRunService,
    session_id: &str,
    params: Value,
) -> Result<Value> {
    let session_id = broker.effective_session_id(session_id).await?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call 缺少 name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let value = match name {
        "ui_confirm_route" => {
            let route = arguments
                .get("route")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let reason = arguments
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if reason.is_empty() || reason.chars().count() > 500 {
                bail!("ui_confirm_route.reason 必须是 1 到 500 字的简短理由");
            }
            match route {
                "UI_DESIGN" => json!({
                    "acceptedRoute": "UI_DESIGN",
                    "next": ["ui_get_design_task", "ui_get_project_profile", "ui_get_runtime_status"],
                    "instruction": "继续 UI Tool-first 工作流；样式修改先实时预览，再写回源码。"
                }),
                "NON_UI" => json!({
                    "acceptedRoute": "NON_UI",
                    "next": "NORMAL_DEVELOPMENT",
                    "instruction": "停止 UI 拟合和 Live Patch；按普通功能开发处理，并保持最小源码读取。"
                }),
                _ => bail!("ui_confirm_route.route 必须是 UI_DESIGN 或 NON_UI"),
            }
        }
        "ui_get_project_profile" => {
            let session = broker.session(&session_id).await?;
            json!({ "profile": super::design_bootstrap::project_profile(&session)? })
        }
        "ui_import_desktop_task" => {
            let session = broker.session(&session_id).await?;
            json!({ "result": super::desktop_task::import_desktop_task(&session, &arguments)? })
        }
        "ui_get_design_task" => {
            let session = broker.session(&session_id).await?;
            super::design_bootstrap::design_task(&session, &arguments)?
        }
        "ui_get_runtime_status" => {
            let session = broker.session(&session_id).await?;
            let view = session.view().await;
            json!({
                "phase": if view.connected { "LIVE" } else { "BOOTSTRAP" },
                "session": view,
            })
        }
        "ui_list_render_devices" => {
            let devices =
                crate::node_agent_android_inspector::adb_wireless::list_device_inventory().await?;
            let mut busy_device_ids = broker
                .renderer_devices_owned_by_other_sessions(&session_id)
                .await;
            busy_device_ids.extend(
                broker
                    .debug_runtime_preparations
                    .busy_device_ids_except(&session_id)
                    .await,
            );
            let device_availability = devices
                .iter()
                .map(|device| {
                    (
                        device.serial.clone(),
                        json!({
                            "rendererBusy":busy_device_ids.contains(device.serial.as_str()),
                            "emulatorSlotId":device.serial.starts_with("emulator-").then_some(device.serial.as_str()),
                        }),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            let recommended_device_id = devices
                .iter()
                .find(|device| {
                    device.state == "device"
                        && device.serial.starts_with("emulator-")
                        && !busy_device_ids.contains(device.serial.as_str())
                })
                .or_else(|| {
                    devices.iter().find(|device| {
                        device.state == "device"
                            && !busy_device_ids.contains(device.serial.as_str())
                    })
                })
                .map(|device| device.serial.clone());
            json!({
                "devices": devices,
                "deviceAvailability":device_availability,
                "recommendedDeviceId":recommended_device_id,
                "busyDeviceIds":busy_device_ids,
            })
        }
        "ui_prepare_debug_runtime" => {
            super::mcp_runtime_preparation::prepare_debug_runtime(broker, &session_id, &arguments)
                .await?
        }
        "ui_get_debug_integration_status" => {
            super::mcp_runtime_preparation::debug_integration_status(
                broker,
                &session_id,
                &arguments,
            )
            .await?
        }
        "ui_bind_target_design" => {
            let session = broker.session(&session_id).await?;
            let upload = super::design_bootstrap::target_design_upload(&session, &arguments)?;
            let target = persist_target_design(broker, &session_id, upload).await?;
            let existing = load_or_build_ui_ir(broker, &session_id).await?;
            let ir = bind_ui_ir(
                broker,
                &session_id,
                BindUiIrRequest {
                    snapshot: existing.snapshot,
                    selected_runtime_node_id: existing.selected_runtime_node_id,
                    source_candidates: existing.source_candidates,
                    target_design: Some(target.clone()),
                    clear_target_design: false,
                },
            )
            .await?;
            json!({ "targetDesign": target, "uiIrRevision": ir.revision })
        }
        "ui_map_annotations_to_nodes" => {
            let session = broker.session(&session_id).await?;
            let bundle = super::design_bootstrap::design_task(&session, &arguments)?;
            let (_, nodes) = broker.tree(&session_id).await?;
            super::annotation_mapping::map_annotations(&bundle, &nodes)?
        }
        "ui_create_compose_screen_scaffold" => {
            let session = broker.session(&session_id).await?;
            super::design_bootstrap::create_compose_screen_scaffold(&session, &arguments)?
        }
        "ui_create_android_screen_scaffold" => {
            let session = broker.session(&session_id).await?;
            super::design_bootstrap::create_android_screen_scaffold(&session, &arguments)?
        }
        "ui_get_screen_summary" => {
            let ir = load_or_build_ui_ir(broker, &session_id).await?;
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
        "ui_trace_window_insets_sequence" => {
            let session = broker.session(&session_id).await?;
            super::window_insets_sequence::run(&session, arguments).await?
        }
        "ui_trace_relational_layout_geometry" => {
            let session = broker.session(&session_id).await?;
            super::relational_layout_geometry::run(&session, arguments).await?
        }
        "ui_get_node" => {
            let ir = load_or_build_ui_ir(broker, &session_id).await?;
            let node = find_node(&ir, &arguments)?;
            json!({ "revision": ir.revision, "node": node })
        }
        "ui_get_subtree" => {
            let ir = load_or_build_ui_ir(broker, &session_id).await?;
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
            let ir = load_or_build_ui_ir(broker, &session_id).await?;
            source_bundle(&ir, &arguments)?
        }
        "ui_get_target_crop" => {
            let ir = load_or_build_ui_ir(broker, &session_id).await?;
            let target = ir
                .target_design
                .ok_or_else(|| anyhow!("尚未绑定目标设计图"))?;
            let session = broker.session(&session_id).await?;
            let artifact = persist_target_crop_artifact(
                &session,
                &target.path,
                parse_rect(arguments.get("rect"))?,
            )?;
            json!({ "artifact": artifact, "targetSha256": target.sha256 })
        }
        "ui_get_current_crop" => {
            let session = broker.session(&session_id).await?;
            let artifact =
                capture_latest_frame_artifact(&session, parse_rect(arguments.get("rect"))?).await?;
            json!({ "artifact": artifact, "treeRevision": session.view().await.tree_revision })
        }
        "ui_capture_android_launcher_surface" => {
            let session = broker.session(&session_id).await?;
            super::launcher_surface::capture(&session, &arguments).await?
        }
        "ui_render_android_launcher_masks" => {
            let session = broker.session(&session_id).await?;
            super::launcher_mask::render(&session, &arguments)?
        }
        "ui_get_visual_diff" => visual_diff_from_ir(broker, &session_id, &arguments).await?,
        "ui_propose_live_patch" => propose_live_patch(broker, &session_id, &arguments).await?,
        "ui_apply_live_patch" => {
            let patch: LiveStylePatch =
                serde_json::from_value(arguments.get("patch").cloned().unwrap_or(arguments))
                    .context("patch 参数不符合 LiveStylePatch")?;
            let ack = broker.apply_patch(&session_id, patch).await?;
            json!({ "ack": ack })
        }
        "ui_run_visual_solver" => {
            let request: VisualSolverRequest =
                serde_json::from_value(arguments).context("视觉求解参数无效")?;
            json!({ "result": solve_visual_style(broker, &session_id, request).await? })
        }
        "ui_start_fit_run" => {
            super::mcp_fit_start::start(broker, fit_runs, &session_id, &arguments).await?
        }
        "ui_get_fit_run" => {
            let context = fit_session_context(broker, &session_id).await?;
            if let Some(run_id) = arguments.get("runId").and_then(Value::as_str) {
                json!({ "run": fit_runs.get_run(&context, run_id)? })
            } else {
                json!({ "runs": fit_runs.list_runs(&context)? })
            }
        }
        "ui_control_fit_run" => {
            let run_id = arguments
                .get("runId")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("缺少 runId"))?;
            let action = arguments
                .get("action")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("缺少 action"))?;
            let context = fit_session_context(broker, &session_id).await?;
            if action.trim().eq_ignore_ascii_case("ATTACH_STATE_REPLAY") {
                let request = attach_state_replay_request(&arguments)?;
                json!({ "result": fit_runs.attach_state_replay(context, run_id, request).await? })
            } else {
                let command = fit_command(action, &arguments)?;
                json!({ "result": fit_runs.command(context, run_id, command).await? })
            }
        }
        "ui_check_capabilities" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::check_capabilities(&session, &arguments).await?
        }
        "ui_check_workflow_completion" => {
            super::task_completion::verify(broker, fit_runs, &session_id, &arguments).await?
        }
        "ui_write_cross_platform_verification" => super::cross_platform_verification::write(
            broker.session(&session_id).await?.as_ref(),
            &arguments,
        )?,
        "ui_report_capability_gap" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::report_gap(&session, &arguments).await?
        }
        "ui_get_capability_gap" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::get_gap(&session, &arguments)?
        }
        "ui_control_capability_gap" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::control_gap(&session, &arguments)?
        }
        "ui_start_capability_upgrade" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::start_capability_upgrade(&session, &arguments)?
        }
        "ui_complete_capability_upgrade" => {
            let session = broker.session(&session_id).await?;
            super::capability_gap::complete_capability_upgrade(&session, &arguments)?
        }
        "ui_get_commit_plan" => {
            let session = broker.session(&session_id).await?;
            json!({ "plan": build_source_commit_plan(session).await? })
        }
        "ui_commit_bound_styles" => {
            let source_revision = arguments
                .get("sourceRevision")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("缺少 sourceRevision"))?
                .to_string();
            let session = broker.session(&session_id).await?;
            json!({
                "result": commit_source(session, SourceCommitRequest { source_revision }).await?
            })
        }
        "ui_activate_preview_scenario" => {
            let request: PreviewOpenRequest =
                serde_json::from_value(arguments).context("Preview 场景激活参数无效")?;
            let host_port = crate::node_agent_admin_open::admin_port_from_env();
            json!({
                "result": activate_preview_scenario(broker, &session_id, request, host_port).await?
            })
        }
        "ui_build_and_verify" => {
            let progress =
                if let Some(operation_id) = arguments.get("operationId").and_then(Value::as_str) {
                    broker
                        .build_verifications
                        .poll(&session_id, operation_id)
                        .await?
                } else {
                    let request: BuildVerifyRequest =
                        serde_json::from_value(arguments).context("构建验收参数无效")?;
                    let host_port = crate::node_agent_admin_open::admin_port_from_env();
                    broker
                        .build_verifications
                        .start(broker.clone(), session_id.clone(), request, host_port)
                        .await?
                };
            json!({ "result": progress })
        }
        name if super::design_tools::is_tool(name) => {
            let session = broker.session(&session_id).await?;
            super::design_tools::call(&session, name, arguments).await?
        }
        crate::node_agent_pwa_runtime::TOOL_NAME => {
            let session = broker.session(&session_id).await?;
            crate::node_agent_pwa_runtime::capture_tool(session.project_root.as_deref(), arguments)
                .await
        }
        super::verification_workflow::TOOL_NAME => {
            super::verification_workflow::verify(broker, &session_id, &arguments).await?
        }
        _ => bail!("未知 UI MCP 工具: {name}"),
    };
    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value)? }],
        "structuredContent": value,
        "isError": false,
    }))
}
pub(super) async fn fit_session_context(
    broker: &LiveUiBroker,
    session_id: &str,
) -> Result<FitSessionContext> {
    let session = broker.session(session_id).await?;
    let view = session.view().await;
    let project_root = session
        .project_root
        .clone()
        .ok_or_else(|| anyhow!("FitRun 需要本机项目目录"))?;
    let source_revision = workspace_fingerprint(&project_root)?;
    Ok(FitSessionContext {
        session_id: session.id.clone(),
        project_root,
        package_name: session.package_name.clone(),
        device_id: session.device_id.clone(),
        runtime_build_id: view.runtime_build_id,
        tree_revision: view.tree_revision,
        source_revision,
    })
}
fn attach_state_replay_request(arguments: &Value) -> Result<AttachStateReplayRequest> {
    let required = |key: &str| -> Result<String> {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("ATTACH_STATE_REPLAY 缺少 {key}"))
    };
    let state_replay: FitStateReplay = serde_json::from_value(
        arguments
            .get("stateReplay")
            .cloned()
            .ok_or_else(|| anyhow!("ATTACH_STATE_REPLAY 缺少 stateReplay"))?,
    )
    .context("ATTACH_STATE_REPLAY stateReplay 参数无效")?;
    Ok(AttachStateReplayRequest {
        command_id: format!("mcp_{}", uuid::Uuid::new_v4().simple()),
        project_root: required("projectRoot")?,
        scenario: required("scenario")?,
        state_replay,
        target_runtime_node_id: required("targetRuntimeNodeId")?,
        target_definition_id: required("targetDefinitionId")?,
        target_instance_key: arguments
            .get("targetInstanceKey")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

fn find_node<'a>(
    ir: &'a UiIrDocument,
    arguments: &Value,
) -> Result<&'a super::protocol::LiveUiNode> {
    super::node_selector::resolve(&ir.nodes, arguments)
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
    let session = broker.session(session_id).await?;
    let reusable = arguments
        .get("currentArtifact")
        .cloned()
        .map(serde_json::from_value::<super::frame_artifact::ReusableFrameArtifact>)
        .transpose()
        .context("currentArtifact 参数无效")?;
    let (current_path, current_frame) = if let Some(artifact) = reusable {
        let path = super::frame_artifact::validate_launcher_crop_artifact(&session, &artifact)?;
        (
            path.display().to_string(),
            json!({
                "source": artifact.source,
                "path": path,
                "sha256": artifact.sha256,
                "reused": true,
            }),
        )
    } else {
        let current = capture_latest_frame_artifact(&session, None).await?;
        (current.path.clone(), json!(current))
    };
    let request = VisualDiffRequest {
        target_path: target.path,
        current_path,
        target_rect: parse_rect(arguments.get("targetRect"))?,
        current_rect: parse_rect(arguments.get("currentRect"))?,
        projected_current_rect: parse_rect(arguments.get("projectedCurrentRect"))?,
        mask: serde_json::from_value(arguments.get("mask").cloned().unwrap_or_else(|| json!({})))
            .context("mask 参数无效")?,
    };
    Ok(json!({ "diff": compare_images(&request)?, "currentFrame": current_frame }))
}

async fn propose_live_patch(
    broker: &LiveUiBroker,
    session_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let node = find_node(&ir, arguments)?;
    let target = parse_rect(arguments.get("projectedCurrentRect"))?
        .ok_or_else(|| anyhow!("缺少校准后的 projectedCurrentRect"))?;
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
