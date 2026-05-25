use anyhow::Result;
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
        api_agent_name, choose_backend, cli_option_id, has_api_agents, is_local_cli_option,
    },
    ai_cli,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    store::ProjectAccess,
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
    let user_config_workspace = state.get_user_workspace(user_id);
    let require_existing_git = matches!(project.source_type.as_str(), "local_path" | "github");
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
            WsMessage::Error {
                message: format!(
                    "当前项目被标记为 Git/local_path 项目，但 {} 不是带 origin 远端的 Git 仓库。请先把它设置成真实 git clone，并配置可用远端后再继续。",
                    workspace.display()
                ),
            }
            .to_json(),
        );
        return;
    }
    if require_git_for_this_request {
        match tools::git_pull_rebase(&workspace) {
            Ok(msg) => {
                if msg.starts_with("git pull 未成功") {
                    warn!("项目同步预检未完成，交给 AI CLI 处理: {}", msg);
                    let _ = tx.send(
                        WsMessage::Progress {
                            message: "同步检查遇到 Git 工作区问题，已交给 AI 助手处理。".into(),
                        }
                        .to_json(),
                    );
                    preflight_note = Some(msg);
                } else {
                    let _ = tx.send(WsMessage::Progress { message: msg }.to_json());
                }
            }
            Err(e) => {
                let msg = format!("git pull --rebase 执行出错: {}", e);
                warn!("项目同步预检执行出错，交给 AI CLI 处理: {}", msg);
                let _ = tx.send(
                    WsMessage::Progress {
                        message: "同步检查执行出错，已交给 AI 助手处理。".into(),
                    }
                    .to_json(),
                );
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
        let _ = tx.send(
            WsMessage::Error {
                message: e.to_string(),
            }
            .to_json(),
        );
    }
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
    let image_cli_only = matches!(
        decision.intent,
        intent_router::UserIntent::TextToImage | intent_router::UserIntent::ImageAssetForApp
    ) || matches!(
        decision.route,
        CapabilityRoute::TextToImage | CapabilityRoute::ImageThenCode
    );
    let backend_route = if codex_cli_only {
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
                WsMessage::Progress {
                    message: "当前已锁定使用 Codex CLI，不切换到其他 AI 代理。".into(),
                }
                .to_json(),
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
            WsMessage::Progress {
                message: "图片处理已切换为 Codex CLI，不调用独立图片模型。".into(),
            }
            .to_json(),
        );
        CapabilityRoute::CodeAgent
    } else {
        decision.route
    };
    let backend_agent_name = if codex_cli_only {
        Some("codex_cli")
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
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let user_config = UserAgentConfig::load(&user_config_workspace);
    let backend = choose_backend(state, user_config.as_ref(), agent_name, route);

    match backend {
        AiBackend::LocalCli => {
            match ai_cli::run_with_workspace(
                user_id,
                workspace,
                download_base,
                user_message,
                preflight_note,
                cli_option_id(agent_name),
                route,
                require_existing_git,
                native_session_scope.clone(),
                trace_id,
                state,
                tx,
            )
            .await
            {
                Ok(()) => Ok(()),
                Err(e)
                    if allow_api_fallback
                        && state.ai_cli.fallback_to_api
                        && has_api_agents(state).await =>
                {
                    warn!("本地 AI CLI 执行失败，回退到 API 代理: {}", e);
                    let _ = tx.send(
                        WsMessage::Progress {
                            message: format!("本地 AI CLI 暂不可用，正在切换原 API 代理: {}", e),
                        }
                        .to_json(),
                    );
                    run_api_inner_with_workspace(
                        user_id,
                        workspace,
                        user_config_workspace,
                        download_base,
                        user_message,
                        preflight_note,
                        api_agent_name(state, agent_name),
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
                preflight_note,
                api_agent_name(state, agent_name),
                state,
                tx,
            )
            .await
        }
    }
}

