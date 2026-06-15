use anyhow::Result;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    agent_api_loop::run_api_inner_with_workspace,
    agent_intent::{
        has_origin_remote, is_project_delivery_request, is_project_workspace,
        is_short_build_command, is_short_resume_command,
    },
    agent_routing::{
        api_agent_name, choose_backend, has_api_agents, is_local_cli_option, resolve_cli_option_id,
    },
    ai_cli, context_compiler,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    source_hygiene,
    store::{ProjectAccess, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AiBackend, AppState, UserAgentConfig, WsMessage},
};

/// 一龙自项目路径（默认 /root/Elon，可由 ELON_SELF_PATH 环境变量覆盖）
pub fn elon_self_workspace() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("ELON_SELF_PATH").unwrap_or_else(|_| "/root/Elon".into()),
    )
}

pub async fn run_for_project(
    user_id: &str,
    project: &ProjectAccess,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    run_for_project_in_workspace(
        user_id,
        project,
        &workspace,
        download_base,
        conversation_id,
        user_message,
        agent_name,
        trace_id,
        state,
        tx,
    )
    .await;
}

pub async fn run_for_project_in_workspace(
    user_id: &str,
    project: &ProjectAccess,
    workspace: &Path,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    if let Some((agent_id, pc_workspace)) = pc_project_binding(project) {
        // 从 agent_name 查 AiCliOption 获取 CLI 类型、Copilot model ID 和显示标签
        let option = agent_name.and_then(|name| state.ai_cli.find_option(Some(name)).cloned());
        let pc_cli = option
            .as_ref()
            .map(|o| {
                if o.provider == "codex" || o.id.to_lowercase().contains("codex") {
                    "codex"
                } else {
                    "copilot"
                }
            })
            .or_else(|| {
                agent_name.map(|name| {
                    let lower = name.to_lowercase();
                    if lower.contains("codex") {
                        "codex"
                    } else {
                        "copilot"
                    }
                })
            })
            .unwrap_or("copilot");
        // Copilot CLI 的 model ID（用于 --model 参数）
        let copilot_model = option
            .as_ref()
            .and_then(|o| o.model.as_deref())
            .map(String::from);
        // Codex 的 reasoning_effort（用于 -c model_reasoning_effort="..." 参数）
        let codex_reasoning_effort = option
            .as_ref()
            .and_then(|o| o.reasoning_effort.as_deref())
            .map(String::from);
        // 用户可见的模型标签（用于气泡属指）
        let model_label = option
            .as_ref()
            .map(|o| o.label.clone())
            .or_else(|| agent_name.map(String::from));
        let _ = tx.send(
            WsMessage::progress(format!(
                "正在连接 PC 节点 {} 使用 {} 处理本地项目。",
                agent_id,
                model_label.as_deref().unwrap_or(pc_cli)
            ))
            .to_json(),
        );
        let session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: cid.to_string(),
        });
        if let Err(e) = ai_cli::run_with_pc_agent_workspace(
            agent_id,
            user_id,
            pc_workspace,
            user_message,
            None,
            ai_cli::AiCliRequestMode::Execute,
            session_scope,
            Some(pc_cli),
            copilot_model.as_deref(),
            codex_reasoning_effort.as_deref(),
            model_label.as_deref(),
            state,
            &tx,
        )
        .await
        {
            error!("PC 本地项目代理运行出错: {}", e);
            let _ = tx.send(WsMessage::error(e.to_string()).to_json());
        }
        return;
    }

    let user_config_workspace = state.get_user_workspace(user_id);
    let require_existing_git = matches!(
        project.source_type.as_str(),
        "local_path" | "github" | "pc_managed"
    );
    let decision = intent_router::classify(user_message);
    let requires_project_workflow = decision.route != CapabilityRoute::ChatAgent
        || is_short_resume_command(user_message, &workspace)
        || is_short_build_command(user_message, &workspace)
        || is_project_delivery_request(user_message, &workspace);
    let require_git_for_this_request = require_existing_git && requires_project_workflow;
    let mut preflight_note: Option<String> = None;
    if require_git_for_this_request
        && (!workspace.join(".git").exists() || !has_origin_remote(&workspace))
    {
        let _ = tx.send(
            WsMessage::error(format!(
                "当前项目被标记为 Git/local_path 项目，但 {} 不是带 origin 远端的 Git 仓库。请先把它设置成真实 git clone，并配置可用远端后再继续。",
                workspace.display()
            ))
            .to_json(),
        );
        return;
    }
    if require_git_for_this_request {
        match tools::git_fetch_status(&workspace) {
            Ok(msg) => {
                if msg.starts_with("git fetch 未成功")
                    || msg.contains("本地落后")
                    || msg.contains("未推送提交")
                    || msg.contains("已分叉")
                {
                    warn!("项目同步预检未完成，交给 AI CLI 处理: {}", msg);
                    let _ = tx.send(
                        WsMessage::progress("同步状态检查已完成，远端状态已交给 AI 助手处理。")
                            .to_json(),
                    );
                    preflight_note = Some(msg);
                } else {
                    let _ = tx.send(WsMessage::progress(msg).to_json());
                }
            }
            Err(e) => {
                let msg = format!("git fetch origin main 执行出错: {}", e);
                warn!("项目同步预检执行出错，交给 AI CLI 处理: {}", msg);
                let _ = tx
                    .send(WsMessage::progress("同步检查执行出错，已交给 AI 助手处理。").to_json());
                preflight_note = Some(msg);
            }
        }
    }
    if let Err(e) = run_dispatch_with_workspace(
        user_id,
        &workspace,
        &user_config_workspace,
        download_base,
        Some(ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.unwrap_or("default").to_string(),
        }),
        user_message,
        preflight_note.as_deref(),
        agent_name,
        trace_id,
        require_git_for_this_request,
        state,
        &tx,
    )
    .await
    {
        error!("项目级 AI 代理运行出错: {}", e);
        let _ = tx.send(WsMessage::error(e.to_string()).to_json());
    }
}

pub async fn plan_for_project_in_workspace(
    user_id: &str,
    project: &ProjectAccess,
    workspace: &Path,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    if let Some((agent_id, pc_workspace)) = pc_project_binding(project) {
        let option = agent_name.and_then(|name| state.ai_cli.find_option(Some(name)).cloned());
        let pc_cli = option
            .as_ref()
            .map(|o| {
                if o.provider == "codex" || o.id.to_lowercase().contains("codex") {
                    "codex"
                } else {
                    "copilot"
                }
            })
            .or_else(|| {
                agent_name.map(|name| {
                    if name.to_lowercase().contains("codex") {
                        "codex"
                    } else {
                        "copilot"
                    }
                })
            })
            .unwrap_or("copilot");
        let copilot_model = option
            .as_ref()
            .and_then(|o| o.model.as_deref())
            .map(String::from);
        let codex_reasoning_effort = option
            .as_ref()
            .and_then(|o| o.reasoning_effort.as_deref())
            .map(String::from);
        let model_label = option
            .as_ref()
            .map(|o| o.label.clone())
            .or_else(|| agent_name.map(String::from));
        let _ = tx.send(
            WsMessage::progress(format!(
                "正在连接 PC 节点 {} 使用 {} 规划本地项目。",
                agent_id,
                model_label.as_deref().unwrap_or(pc_cli)
            ))
            .to_json(),
        );
        if let Err(e) = ai_cli::run_with_pc_agent_workspace(
            agent_id,
            user_id,
            pc_workspace,
            user_message,
            None,
            ai_cli::AiCliRequestMode::Plan,
            conversation_id.map(|cid| ai_cli::NativeSessionScope {
                project_id: project.id.clone(),
                user_id: user_id.to_string(),
                conversation_id: cid.to_string(),
            }),
            Some(pc_cli),
            copilot_model.as_deref(),
            codex_reasoning_effort.as_deref(),
            model_label.as_deref(),
            state,
            &tx,
        )
        .await
        {
            error!("PC 本地项目规划运行出错: {}", e);
            let _ = tx.send(WsMessage::error(e.to_string()).to_json());
        }
        return;
    }

    let user_config_workspace = state.get_user_workspace(user_id);
    if let Err(e) = run_backend_with_workspace(
        user_id,
        workspace,
        &user_config_workspace,
        download_base,
        Some(ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.unwrap_or("default").to_string(),
        }),
        user_message,
        None,
        agent_name,
        trace_id,
        CapabilityRoute::CodeAgent,
        true,
        false,
        true,
        state,
        &tx,
    )
    .await
    {
        error!("项目级 AI 规划运行出错: {}", e);
        let _ = tx.send(WsMessage::error(e.to_string()).to_json());
    }
}

fn pc_project_binding(project: &ProjectAccess) -> Option<(&str, &str)> {
    match (
        project.node_id.as_deref(),
        project.workspace_path.as_deref(),
    ) {
        (Some(agent_id), Some(workspace)) if !agent_id.is_empty() && !workspace.is_empty() => {
            Some((agent_id, workspace))
        }
        _ => None,
    }
}

fn pc_project_note(workspace: &str, planning: bool) -> String {
    let mode = if planning {
        "当前是 Plan 模式，只生成计划，不改文件、不运行发布。"
    } else {
        "当前是执行模式，可以按项目规则修改、验证、提交和发布。"
    };
    format!(
        "当前项目绑定在 PC 节点本地工作区：{}。CLI 已以该目录作为当前工作目录启动；不要改到其他仓库。{}",
        workspace, mode
    )
}

async fn run_dispatch_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    native_session_scope: Option<ai_cli::NativeSessionScope>,
    user_message: &str,
    preflight_note: Option<&str>,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    require_existing_git: bool,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let resume_command = is_short_resume_command(user_message, workspace);
    let delivery_request = is_project_delivery_request(user_message, workspace);
    if delivery_request && !resume_command {
        if tools::find_latest_apk(workspace).is_some() {
            let apk_url = tools::stable_apk_url(download_base);
            let _ = tx.send(
                WsMessage::Done {
                    message: "我看了当前项目状态，APK 已经生成了。你现在最需要的是下载安装测试，所以我先把下载链接给你。".into(),
                    apk_url: Some(apk_url),
                    image_url: None,
                    model_used: None,
                    node_id: None,
                }
                .to_json(),
            );
            return Ok(());
        }
    }

    let mut decision = intent_router::classify(user_message);
    if resume_command || delivery_request || is_short_build_command(user_message, workspace) {
        decision = RoutingDecision {
            intent: intent_router::UserIntent::AppDevelopment,
            route: CapabilityRoute::CodeAgent,
            confidence: 88,
            needs_image_generation: false,
            needs_code_change: true,
            allow_user_agent_preference: true,
            reason: "project_resume_command",
        };
    }
    info!("intent routing decision: {:?}", decision);

    let codex_cli_only = state.ai_cli.codex_cli_only;
    let user_byok_api_override = codex_cli_only
        && crate::user_agent_secrets::user_byok_api_enabled()
        && UserAgentConfig::load(user_config_workspace)
            .as_ref()
            .map(|cfg| cfg.has_direct_custom_api())
            .unwrap_or(false);
    // ImageThenCode: 有文生图模型时走两步管道（先生图，再集成到代码）；否则降级为 CodeAgent
    let is_image_then_code =
        matches!(decision.route, CapabilityRoute::ImageThenCode) && state.image_model.is_some();
    let image_cli_only = !is_image_then_code
        && (matches!(
            decision.intent,
            intent_router::UserIntent::TextToImage | intent_router::UserIntent::ImageAssetForApp
        ) || matches!(
            decision.route,
            CapabilityRoute::TextToImage | CapabilityRoute::ImageThenCode
        ));
    let backend_route = if codex_cli_only && !user_byok_api_override {
        if !state.ai_cli.enabled {
            return Err(anyhow::anyhow!(
                "当前已锁定只使用 Codex CLI，但服务端没有可用的 Codex CLI 选项"
            ));
        }
        if agent_name
            .map(|name| !is_local_cli_option(state, name))
            .unwrap_or(false)
        {
            let _ = tx.send(
                WsMessage::progress("当前已锁定使用 Codex CLI，不切换到其他 AI 代理。").to_json(),
            );
        }
        decision.route
    } else if image_cli_only {
        if !state.ai_cli.enabled {
            return Err(anyhow::anyhow!(
                "图片处理测试模式仅使用 Codex CLI，但本地 AI CLI 未启用"
            ));
        }
        let _ = tx.send(
            WsMessage::progress("图片处理已切换为 Codex CLI，不调用独立图片模型。").to_json(),
        );
        CapabilityRoute::CodeAgent
    } else {
        decision.route
    };
    let backend_agent_name = if codex_cli_only && !user_byok_api_override {
        agent_name.filter(|name| is_local_cli_option(state, name))
    } else if state.ai_cli.enabled {
        agent_name.filter(|name| is_local_cli_option(state, name))
    } else if image_cli_only {
        match agent_name {
            Some(name) if is_local_cli_option(state, name) => agent_name,
            _ => Some("codex_cli"),
        }
    } else {
        agent_name
    };

    // ImageThenCode 两步管道：先文生图，把 URL 注入消息，再走代码 Agent 集成
    if is_image_then_code {
        let image_model = state.image_model.as_ref().map(|cfg| cfg.model.clone());
        let mut image_billing_call = if let Some(model) = image_model.as_deref() {
            let key = crate::billing_lifecycle::new_compute_call_id("image_then_code");
            Some(
                crate::compute_usage::reserve_image_generation(
                    &state.store,
                    user_id,
                    &key,
                    "image_then_code",
                    model,
                    user_message,
                )
                .map_err(|msg| anyhow::anyhow!(msg))?,
            )
        } else {
            crate::billing::check_can_call(&state.store, user_id)
                .map_err(|msg| anyhow::anyhow!(msg))?;
            None
        };
        let _ = tx.send(WsMessage::progress("正在生成图片资源...").to_json());
        match crate::image_generation::generate_text_to_image(state, user_message).await {
            Ok(img) => {
                if let (Some(model), Some(billing_call)) =
                    (image_model.as_deref(), image_billing_call.as_mut())
                {
                    crate::compute_usage::record_image_generation_with_key(
                        &state.store,
                        user_id,
                        "image_then_code",
                        model,
                        user_message,
                        Some(billing_call.key()),
                    );
                    billing_call.mark_settled();
                }
                let injected_message = format!(
                    "{}\n\n[已生成图片: {}]\n请将上方图片 URL 下载后集成到项目中作为所需的图片资源。",
                    user_message, img.url
                );
                let _ = tx.send(WsMessage::progress("图片生成完成，正在集成到代码...").to_json());
                return run_backend_with_workspace(
                    user_id,
                    workspace,
                    user_config_workspace,
                    download_base,
                    native_session_scope,
                    &injected_message,
                    preflight_note,
                    backend_agent_name,
                    trace_id,
                    CapabilityRoute::CodeAgent,
                    true,
                    require_existing_git,
                    false, // planning_mode: 图片集成走执行路径，不进入规划
                    state,
                    tx,
                )
                .await;
            }
            Err(e) => {
                warn!("文生图失败，降级到纯代码路径: {}", e);
                let _ = tx.send(
                    WsMessage::progress(format!("图片生成失败（{}），将尝试用代码实现。", e))
                        .to_json(),
                );
            }
        }
    }

    run_backend_with_workspace(
        user_id,
        workspace,
        user_config_workspace,
        download_base,
        native_session_scope,
        user_message,
        preflight_note,
        backend_agent_name,
        trace_id,
        backend_route,
        !(codex_cli_only || image_cli_only),
        require_existing_git,
        false, // planning_mode: dispatch 走执行路径，规划走 plan_for_project_in_workspace
        state,
        tx,
    )
    .await
}

async fn run_backend_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    native_session_scope: Option<ai_cli::NativeSessionScope>,
    user_message: &str,
    preflight_note: Option<&str>,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    route: CapabilityRoute,
    allow_api_fallback: bool,
    require_existing_git: bool,
    planning_mode: bool,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let user_config = UserAgentConfig::load(&user_config_workspace);
    let backend = choose_backend(state, user_config.as_ref(), agent_name, route);
    let source_hygiene_note = if route == CapabilityRoute::ChatAgent {
        None
    } else {
        source_hygiene::source_size_preflight_note(workspace)
    };
    let base_preflight_note =
        combine_preflight_notes(preflight_note, source_hygiene_note.as_deref());
    let context_compiler_note = if route == CapabilityRoute::ChatAgent {
        None
    } else {
        context_compiler::compile_preflight_note(state, workspace, user_id, user_message, trace_id)
            .await
    };
    let combined_preflight_note = combine_preflight_notes(
        base_preflight_note.as_deref(),
        context_compiler_note.as_deref(),
    );
    let memory_scope_project_id = native_session_scope
        .as_ref()
        .map(|scope| scope.project_id.as_str());
    let memory_scope_type = memory_scope_project_id.map(|_| MEMORY_SCOPE_PROJECT);

    match backend {
        AiBackend::LocalCli => {
            let preferred_local_agent = agent_name.or_else(|| {
                user_config
                    .as_ref()
                    .and_then(|cfg| cfg.use_agent.as_deref())
                    .filter(|name| is_local_cli_option(state, name))
            });
            let primary_option_owned = resolve_cli_option_id(state, preferred_local_agent);
            let primary_option = primary_option_owned.as_deref();
            let cli_result = if planning_mode {
                ai_cli::run_plan_with_workspace(
                    user_id,
                    workspace,
                    download_base,
                    user_message,
                    combined_preflight_note.as_deref(),
                    primary_option,
                    native_session_scope.clone(),
                    trace_id,
                    state,
                    tx,
                )
                .await
            } else {
                ai_cli::run_with_workspace(
                    user_id,
                    workspace,
                    download_base,
                    user_message,
                    combined_preflight_note.as_deref(),
                    primary_option,
                    route,
                    require_existing_git,
                    native_session_scope.clone(),
                    trace_id,
                    state,
                    tx,
                )
                .await
            };

            match cli_result {
                Ok(()) => Ok(()),
                Err(e) if state.ai_cli.fallback_cli_option.is_some() => {
                    let fallback_id = state.ai_cli.fallback_cli_option.clone().unwrap();
                    // 主 CLI 与备用 CLI 不同时才回退，避免无效重试
                    let primary_resolved = state
                        .ai_cli
                        .find_option(primary_option)
                        .map(|opt| opt.id.as_str())
                        .unwrap_or("");
                    if primary_resolved.eq_ignore_ascii_case(&fallback_id) {
                        return Err(e);
                    }
                    warn!(
                        "主 CLI 执行失败（{}），正在切换备用 CLI {}: {}",
                        primary_resolved, fallback_id, e
                    );
                    let _ = tx.send(
                        WsMessage::progress(format!(
                            "主 AI CLI 暂不可用，正在切换备用 CLI ({fallback_id})…"
                        ))
                        .to_json(),
                    );
                    let fallback_result = if planning_mode {
                        ai_cli::run_plan_with_workspace(
                            user_id,
                            workspace,
                            download_base,
                            user_message,
                            combined_preflight_note.as_deref(),
                            Some(fallback_id.as_str()),
                            native_session_scope.clone(),
                            trace_id,
                            state,
                            tx,
                        )
                        .await
                    } else {
                        ai_cli::run_with_workspace(
                            user_id,
                            workspace,
                            download_base,
                            user_message,
                            combined_preflight_note.as_deref(),
                            Some(fallback_id.as_str()),
                            route,
                            require_existing_git,
                            native_session_scope.clone(),
                            trace_id,
                            state,
                            tx,
                        )
                        .await
                    };
                    match fallback_result {
                        Ok(()) => Ok(()),
                        Err(fallback_e)
                            if allow_api_fallback
                                && state.ai_cli.fallback_to_api
                                && has_api_agents(state).await =>
                        {
                            warn!("备用 CLI 也失败，回退到 API 代理: {}", fallback_e);
                            let _ = tx.send(
                                WsMessage::progress(format!(
                                    "本地 AI CLI 均不可用，正在切换 API 代理: {}",
                                    fallback_e
                                ))
                                .to_json(),
                            );
                            run_api_inner_with_workspace(
                                user_id,
                                workspace,
                                user_config_workspace,
                                download_base,
                                user_message,
                                combined_preflight_note.as_deref(),
                                api_agent_name(state, agent_name),
                                planning_mode,
                                memory_scope_type,
                                memory_scope_project_id,
                                state,
                                tx,
                            )
                            .await
                        }
                        Err(fallback_e) => Err(fallback_e),
                    }
                }
                Err(e)
                    if allow_api_fallback
                        && state.ai_cli.fallback_to_api
                        && has_api_agents(state).await =>
                {
                    warn!("本地 AI CLI 执行失败，回退到 API 代理: {}", e);
                    let _ = tx.send(
                        WsMessage::progress(format!(
                            "本地 AI CLI 暂不可用，正在切换原 API 代理: {}",
                            e
                        ))
                        .to_json(),
                    );
                    run_api_inner_with_workspace(
                        user_id,
                        workspace,
                        user_config_workspace,
                        download_base,
                        user_message,
                        combined_preflight_note.as_deref(),
                        api_agent_name(state, agent_name),
                        planning_mode,
                        memory_scope_type,
                        memory_scope_project_id,
                        state,
                        tx,
                    )
                    .await
                }
                Err(e) => Err(e),
            }
        }
        AiBackend::Api => {
            run_api_inner_with_workspace(
                user_id,
                workspace,
                user_config_workspace,
                download_base,
                user_message,
                combined_preflight_note.as_deref(),
                api_agent_name(state, agent_name),
                planning_mode,
                memory_scope_type,
                memory_scope_project_id,
                state,
                tx,
            )
            .await
        }
    }
}

fn combine_preflight_notes(git_note: Option<&str>, source_note: Option<&str>) -> Option<String> {
    match (git_note, source_note) {
        (Some(git), Some(source)) => Some(format!("{git}\n\n{source}")),
        (Some(git), None) => Some(git.to_string()),
        (None, Some(source)) => Some(source.to_string()),
        (None, None) => None,
    }
}
