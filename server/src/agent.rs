use anyhow::Result;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    agent_intent::{
        has_origin_remote, is_project_delivery_request, is_project_workspace,
        is_short_build_command, is_short_resume_command,
    },
    agent_llm_call::{call_chat_llm, call_llm, execute_tool},
    agent_prompts::system_prompt,
    ai_cli,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    store::ProjectAccess,
    tools,
    types::{AgentConfig, AiBackend, AppState, UserAgentConfig, WsMessage},
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

async fn has_api_agents(state: &Arc<AppState>) -> bool {
    !state.agents_config.read().await.agents.is_empty()
}

fn choose_backend(
    state: &Arc<AppState>,
    user_config: Option<&UserAgentConfig>,
    agent_name: Option<&str>,
    route: CapabilityRoute,
) -> AiBackend {
    if state.ai_cli.codex_cli_only {
        return AiBackend::LocalCli;
    }

    if route == CapabilityRoute::ChatAgent {
        if state.ai_cli.enabled {
            return AiBackend::LocalCli;
        }
        return AiBackend::Api;
    }

    if state.ai_cli.enabled {
        return AiBackend::LocalCli;
    }

    if let Some(name) = agent_name {
        if is_local_cli_option(state, name) {
            return AiBackend::LocalCli;
        }
        if is_api_backend_alias(name) {
            return AiBackend::Api;
        }
        return AiBackend::Api;
    }

    if route == CapabilityRoute::CodeAgent && state.ai_cli.enabled {
        return AiBackend::LocalCli;
    }

    if let Some(cfg) = user_config {
        if cfg.has_config() {
            if cfg
                .use_agent
                .as_deref()
                .map(|name| is_local_cli_option(state, name))
                .unwrap_or(false)
            {
                return AiBackend::LocalCli;
            }
            return AiBackend::Api;
        }
    }

    if state.default_backend == AiBackend::LocalCli && state.ai_cli.enabled {
        AiBackend::LocalCli
    } else {
        AiBackend::Api
    }
}

fn api_agent_name<'a>(state: &Arc<AppState>, agent_name: Option<&'a str>) -> Option<&'a str> {
    agent_name.filter(|name| !is_local_cli_option(state, name) && !is_api_backend_alias(name))
}

fn cli_option_id(agent_name: Option<&str>) -> Option<&str> {
    agent_name.filter(|name| !is_cli_alias(name))
}

fn is_local_cli_option(state: &Arc<AppState>, name: &str) -> bool {
    is_cli_alias(name) || state.ai_cli.has_option(name)
}

fn is_cli_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex" | "codex_cli" | "cli" | "local" | "local_cli"
    )
}

fn is_api_backend_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "api" | "llm" | "remote"
    )
}

fn casual_chat_prompt() -> &'static str {
    r#"你是「一龙开发助手」，也是用户身边一个有经验、有温度的产品与开发搭档。
用户可能只是闲聊、犹豫、没想好要做什么，或者想让你给灵感。

你的回复要自然、有生命力，不要像客服模板，也不要一直重复“这里只能开发 App”。
你可以正常聊天、共情、追问，也可以帮用户把模糊想法整理成 App 方向。

重要边界：
- 这一次是普通聊天模式，不能声称你已经修改代码、执行工具、打包 APK。
- 如果用户还没想好，主动给 2-4 个具体方向，让用户容易继续说下去。
- 如果用户明显想开始开发，引导他补充目标用户、核心功能、界面风格或优先级。
- 回复以中文为主，简洁但有内容。"#
}

fn quick_casual_reply(user_message: &str) -> Option<&'static str> {
    match user_message.trim().to_lowercase().as_str() {
        "你好" | "你好呀" | "在吗" | "你在吗" | "在不在" | "hi" | "hello" => {
            Some("你好，我在。你可以直接告诉我想改代码、查问题、构建 APK，或者先聊聊想法。")
        }
        "谢谢" | "谢谢你" | "辛苦了" => {
            Some("不客气，我在这边。你继续说下一步想怎么改就行。")
        }
        _ => None,
    }
}

async fn resolve_agent(
    state: &Arc<AppState>,
    workspace: &std::path::Path,
    agent_name: Option<&str>,
) -> Result<AgentConfig> {
    let global = state.agents_config.read().await;
    if let Some(cfg) = UserAgentConfig::load(workspace) {
        let uses_local_cli = cfg
            .use_agent
            .as_deref()
            .map(|name| is_local_cli_option(state, name))
            .unwrap_or(false);
        if cfg.has_config() && !uses_local_cli {
            return cfg.resolve(&global).ok_or_else(|| {
                anyhow::anyhow!("未找到可用 API 代理，请在后台配置 AGENT_* 或切回 Codex CLI")
            });
        }
    }

    global
        .get_agent(agent_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("未配置 API 代理，请设置 AGENT_* 或使用 Codex CLI"))
}

async fn run_casual_chat(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    user_message: &str,
) -> Result<String> {
    let messages = vec![
        json!({
            "role": "system",
            "content": casual_chat_prompt()
        }),
        json!({
            "role": "user",
            "content": user_message
        }),
    ];

    let response = call_chat_llm(state, agent, &messages).await?;
    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("我在，你可以继续说。")
        .trim()
        .to_string();

    Ok(if reply.is_empty() {
        "我在，你可以继续说。".into()
    } else {
        reply
    })
}

async fn run_api_inner_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    // 每个用户操作自己的工作区，不能访问其他用户目录

    // 优先使用用户在 APP 里配置的专属代理；否则用管理员指定/默认全局代理。
    // 普通聊天也要走模型，否则体验会像固定话术。
    if !intent_router::looks_like_development_request(user_message) {
        if let Some(reply) = quick_casual_reply(user_message) {
            let _ = tx.send(
                WsMessage::Done {
                    message: reply.to_string(),
                    apk_url: None,
                    image_url: None,
                }
                .to_json(),
            );
            return Ok(());
        }

        let agent = resolve_agent(state, &user_config_workspace, agent_name).await?;
        let _ = tx.send(
            WsMessage::Progress {
                message: format!("正在使用 AI 代理聊天: {} ({})", agent.name, agent.model),
            }
            .to_json(),
        );

        let reply = run_casual_chat(state, &agent, user_message).await?;
        let _ = tx.send(
            WsMessage::Done {
                message: reply,
                apk_url: None,
                image_url: None,
            }
            .to_json(),
        );
        return Ok(());
    }

    // 确保用户工作区存在
    std::fs::create_dir_all(&workspace)?;
    // 初始化 git（如果还未初始化）
    if !workspace.join(".git").exists() {
        let _ = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&workspace)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.email", &format!("{}@elon.app", user_id)])
            .current_dir(&workspace)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.name", user_id])
            .current_dir(&workspace)
            .output();
    }
    let agent = resolve_agent(state, &user_config_workspace, agent_name).await?;
    let workspace_str = workspace.to_string_lossy().to_string();

    let _ = tx.send(
        WsMessage::Progress {
            message: format!("正在使用 AI 代理: {} ({})", agent.name, agent.model),
        }
        .to_json(),
    );

    let effective_user_message = match preflight_note {
        Some(note) => format!(
            "项目预检结果：\n{}\n\n这不是最终失败，请先把它当作当前任务的一部分处理：查看 git status/diff，保护已有改动，能安全提交、stash、worktree 或 rebase 时自行处理，再继续用户原始请求；无法判断时向用户说明并暂停。\n\n用户原始请求：\n{}",
            note, user_message
        ),
        None => user_message.to_string(),
    };

    // 初始化对话历史
    let mut messages = vec![
        json!({
            "role": "system",
            "content": system_prompt(&workspace_str)
        }),
        json!({
            "role": "user",
            "content": effective_user_message
        }),
    ];

    let _ = tx.send(
        WsMessage::Progress {
            message: "AI 正在理解需求...".into(),
        }
        .to_json(),
    );

    // 追踪 APK 下载链接（build_project 成功后填入）
    let mut apk_url: Option<String> = None;

    // 工具调用循环（最多 20 轮，防止死循环）
    for _round in 0..20 {
        let response = call_llm(state, &agent, &messages).await?;

        let choice = &response["choices"][0];
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
        let assistant_message = &choice["message"];

        // 把助手消息加入历史
        messages.push(assistant_message.clone());

        // 如果 LLM 决定结束（没有更多工具调用）
        if finish_reason == "stop" {
            let final_text = assistant_message["content"]
                .as_str()
                .unwrap_or("完成")
                .to_string();

            let _ = tx.send(
                WsMessage::Done {
                    message: final_text,
                    apk_url: apk_url.clone(),
                    image_url: None,
                }
                .to_json(),
            );
            return Ok(());
        }

        // 处理工具调用
        if finish_reason == "tool_calls" {
            let tool_calls = match assistant_message["tool_calls"].as_array() {
                Some(t) => t.clone(),
                None => break,
            };

            for tool_call in &tool_calls {
                let tool_id = tool_call["id"].as_str().unwrap_or("").to_string();
                let tool_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                info!("工具调用: {} {:?}", tool_name, args);

                let _ = tx.send(
                    WsMessage::ToolCall {
                        tool: tool_name.clone(),
                        args: args.clone(),
                    }
                    .to_json(),
                );

                // 执行工具：build_project 优先通过 PC agent（Windows 环境），无 agent 或失败时回退服务器本地构建
                let result: anyhow::Result<String> = if tool_name == "build_project"
                    && !state.agent_manager.list().await.is_empty()
                {
                    let target = args["target"].as_str().unwrap_or("android");
                    let changelog: String = user_message.chars().take(80).collect();
                    let _ = tx.send(
                        WsMessage::Progress {
                            message: format!(
                                "正在通过 PC agent 构建 {}（实时输出将陆续显示）...",
                                target
                            ),
                        }
                        .to_json(),
                    );
                    let r =
                        tools::build_project_via_agent(state, target, &changelog, Some(tx)).await;
                    if let Err(ref e) = r {
                        warn!("PC agent 构建失败，回退到服务器本地构建: {}", e);
                        let _ = tx.send(
                            WsMessage::Progress {
                                message: format!("PC agent 不可用（{}），尝试服务器本地构建...", e),
                            }
                            .to_json(),
                        );
                        execute_tool(state, &workspace, &tool_name, &args)
                    } else {
                        r
                    }
                } else {
                    execute_tool(state, &workspace, &tool_name, &args)
                };

                let result_str = match result {
                    Ok(r) => {
                        // build_project 成功后提取 APK 文件名，生成下载链接
                        if tool_name == "build_project" {
                            if let Some(line) = r.lines().find(|l| l.starts_with("##APK_FILE:")) {
                                let _apk_name = line.trim_start_matches("##APK_FILE:").trim();
                                apk_url = Some(tools::stable_apk_url(download_base));
                                let _ = tx.send(
                                    WsMessage::Progress {
                                        message: format!("APK 编译成功，正在生成下载链接..."),
                                    }
                                    .to_json(),
                                );
                            }
                        }
                        r
                    }
                    Err(e) => format!("错误: {}", e),
                };

                let _ = tx.send(
                    WsMessage::ToolResult {
                        tool: tool_name.clone(),
                        result: result_str.chars().take(500).collect(),
                    }
                    .to_json(),
                );

                // 把工具结果加入对话历史
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_id,
                    "content": result_str
                }));
            }
        } else {
            // 未知 finish_reason，退出循环
            warn!("未知 finish_reason: {}", finish_reason);
            break;
        }
    }

    let _ = tx.send(
        WsMessage::Done {
            message: "任务执行完毕".into(),
            apk_url,
            image_url: None,
        }
        .to_json(),
    );

    Ok(())
}

