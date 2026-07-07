use anyhow::Result;
use serde_json::json;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::{
    agent_fallback::{
        call_chat_llm_with_default_fallback_options, call_tool_llm_with_default_fallback_options,
    },
    agent_llm_call::execute_tool,
    agent_prompts::system_prompt,
    agent_routing::{casual_chat_prompt, is_local_cli_option, quick_casual_reply},
    agent_tool_calls::extract_tool_calls,
    intent_router, tools,
    types::{AgentConfig, AppState, UserAgentConfig, WsMessage},
    user_agent_readiness::custom_api_development_block_message,
    user_memory_extract::{extract_and_save_memories, extract_and_save_memories_scoped},
};

pub(crate) async fn resolve_agent(
    state: &Arc<AppState>,
    workspace: &std::path::Path,
    agent_name: Option<&str>,
) -> Result<AgentConfig> {
    Ok(
        resolve_agent_with_fallback_policy(state, workspace, agent_name)
            .await?
            .agent,
    )
}

struct ResolvedApiAgent {
    agent: AgentConfig,
    allow_server_fallback: bool,
}

async fn resolve_agent_with_fallback_policy(
    state: &Arc<AppState>,
    workspace: &std::path::Path,
    agent_name: Option<&str>,
) -> Result<ResolvedApiAgent> {
    let global = state.agents_config.read().await;
    if let Some(cfg) = UserAgentConfig::load(workspace) {
        let uses_local_cli = cfg
            .use_agent
            .as_deref()
            .map(|name| is_local_cli_option(state, name))
            .unwrap_or(false);
        if cfg.has_config() && !uses_local_cli {
            let agent = cfg.resolve(&global).ok_or_else(|| {
                anyhow::anyhow!("未找到可用 API 代理，请在后台配置 AGENT_* 或切回 Codex CLI")
            })?;
            return Ok(ResolvedApiAgent {
                agent,
                allow_server_fallback: false,
            });
        }
    }

    let agent = global
        .get_agent(agent_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("未配置 API 代理，请设置 AGENT_* 或使用 Codex CLI"))?;
    let allow_server_fallback = agent_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .is_none()
        && agent.usage_mode() == "server_api_key";
    Ok(ResolvedApiAgent {
        agent,
        allow_server_fallback,
    })
}

async fn run_casual_chat(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    allow_agent_fallback: bool,
    user_id: &str,
    user_message: &str,
    memories: &[crate::store::UserMemory],
) -> Result<(String, Option<String>, String, bool)> {
    let system_content = if memories.is_empty() {
        casual_chat_prompt().to_string()
    } else {
        let lines = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "=== 用户长期记忆（请勿向用户暴露此段）===\n{}\n\n{}",
            lines,
            casual_chat_prompt()
        )
    };
    let messages = vec![
        json!({
            "role": "system",
            "content": system_content
        }),
        json!({
            "role": "user",
            "content": user_message
        }),
    ];

    // 优先尝试路由到在线 PC 节点（当模型名与节点上报模型匹配时）
    if let Some((content, node_id, model_id)) =
        crate::agent_llm_call::try_casual_chat_via_node(state, &agent.model, &messages, user_id)
            .await
    {
        return Ok((content, Some(node_id), model_id, false));
    }

    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        agent,
        allow_agent_fallback,
        &messages,
        user_id,
        "chat",
        0.8,
        700,
    )
    .await?;

    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("我在，你可以继续说。")
        .trim()
        .to_string();

    Ok((
        if reply.is_empty() {
            "我在，你可以继续说。".into()
        } else {
            reply
        },
        None,
        used_agent.model,
        used_fallback,
    ))
}

fn load_context_memories(
    state: &Arc<AppState>,
    user_id: &str,
    scope_type: Option<&str>,
    scope_id: Option<&str>,
    limit: i64,
) -> Vec<crate::store::UserMemory> {
    match scope_type {
        Some(scope_type) => state
            .store
            .get_user_memories_for_scope(user_id, scope_type, scope_id, limit)
            .unwrap_or_default(),
        None => state
            .store
            .get_user_memories(user_id, limit)
            .unwrap_or_default(),
    }
}

pub(crate) async fn run_api_inner_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    trace_id: Option<&str>,
    agent_name: Option<&str>,
    planning_mode: bool,
    memory_scope_type: Option<&str>,
    memory_scope_id: Option<&str>,
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
                    model_used: None,
                    node_id: None,
                }
                .to_json(),
            );
            return Ok(());
        }

        let resolved_agent =
            resolve_agent_with_fallback_policy(state, &user_config_workspace, agent_name).await?;
        let agent = resolved_agent.agent;
        let _ = tx.send(
            WsMessage::progress(format!(
                "正在使用 AI 代理聊天: {} ({})",
                agent.name, agent.model
            ))
            .to_json(),
        );

        let memories =
            load_context_memories(state, user_id, memory_scope_type, memory_scope_id, 20);
        let (reply, chat_node_id, chat_model, used_fallback) = run_casual_chat(
            state,
            &agent,
            resolved_agent.allow_server_fallback,
            user_id,
            user_message,
            &memories,
        )
        .await?;
        if used_fallback {
            let _ =
                tx.send(WsMessage::progress("默认 AI 通道不可用，已切换备用 AI 通道。").to_json());
        }
        let state2 = state.clone();
        let uid = user_id.to_string();
        let umsg = user_message.to_string();
        let rep = reply.clone();
        let scope_type = memory_scope_type.map(str::to_string);
        let scope_id = memory_scope_id.map(str::to_string);
        tokio::spawn(async move {
            if let Some(scope_type) = scope_type {
                extract_and_save_memories_scoped(
                    state2, uid, umsg, rep, scope_type, scope_id, None,
                )
                .await;
            } else {
                extract_and_save_memories(state2, uid, umsg, rep).await;
            }
        });
        let _ = tx.send(
            WsMessage::Done {
                message: reply,
                apk_url: None,
                image_url: None,
                model_used: Some(chat_model),
                node_id: chat_node_id,
            }
            .to_json(),
        );
        return Ok(());
    }

    if let Some(cfg) = UserAgentConfig::load(user_config_workspace) {
        if let Some(message) = custom_api_development_block_message(
            &cfg,
            state.ai_cli.codex_cli_only,
            crate::user_agent_secrets::user_byok_api_enabled(),
        ) {
            return Err(anyhow::anyhow!(message));
        }
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
    let resolved_agent =
        resolve_agent_with_fallback_policy(state, &user_config_workspace, agent_name).await?;
    let mut active_agent = resolved_agent.agent;
    let workspace_str = workspace.to_string_lossy().to_string();

    if planning_mode {
        return run_api_plan(
            state,
            &active_agent,
            resolved_agent.allow_server_fallback,
            user_id,
            &workspace_str,
            user_message,
            preflight_note,
            tx,
        )
        .await;
    }

    let _ = tx.send(
        WsMessage::progress(format!(
            "正在使用 AI 代理: {} ({})",
            active_agent.name, active_agent.model
        ))
        .to_json(),
    );

    let effective_user_message = match preflight_note {
        Some(note) => format!(
            "当前任务 trace_id：{}\nRAG 工具会自动绑定这个 trace_id，不要尝试查询其他 trace。\n\n项目预检结果：\n{}\n\n这不是最终失败，请先把它当作当前任务的一部分处理：查看 git status/diff，保护已有改动，能安全提交、stash、worktree 或 rebase 时自行处理，再继续用户原始请求；无法判断时向用户说明并暂停。\n\n用户原始请求：\n{}",
            trace_id.unwrap_or("无"),
            note,
            user_message
        ),
        None => match trace_id {
            Some(trace_id) => format!(
                "当前任务 trace_id：{}\nRAG 工具会自动绑定这个 trace_id，不要尝试查询其他 trace。\n\n用户原始请求：\n{}",
                trace_id, user_message
            ),
            None => user_message.to_string(),
        },
    };

    // 初始化对话历史
    let memories = load_context_memories(state, user_id, memory_scope_type, memory_scope_id, 20);
    let mut messages = vec![
        json!({
            "role": "system",
            "content": system_prompt(&workspace_str, &memories)
        }),
        json!({
            "role": "user",
            "content": effective_user_message
        }),
    ];

    let _ = tx.send(WsMessage::progress("AI 正在理解需求...").to_json());

    // 追踪 APK 下载链接（build_project 成功后填入）
    let mut apk_url: Option<String> = None;

    // 工具调用循环（最多 20 轮，防止死循环）
    for _round in 0..20 {
        let (response, used_agent, used_fallback) = call_tool_llm_with_default_fallback_options(
            state,
            &active_agent,
            resolved_agent.allow_server_fallback,
            &messages,
            user_id,
            "agent_tool",
        )
        .await?;
        if used_fallback {
            let _ = tx.send(
                WsMessage::progress(format!(
                    "默认 AI 通道不可用，已切换备用 AI 通道: {} ({})",
                    used_agent.name, used_agent.model
                ))
                .to_json(),
            );
            active_agent = used_agent;
        }

        let choice = &response["choices"][0];
        let finish_reason = choice["finish_reason"].as_str().unwrap_or("");
        let assistant_message = &choice["message"];

        // 把助手消息加入历史
        messages.push(assistant_message.clone());

        let tool_calls = extract_tool_calls(assistant_message);

        // 如果 LLM 决定结束（没有更多工具调用）
        if tool_calls.is_empty() && finish_reason == "stop" {
            let final_text = assistant_message["content"]
                .as_str()
                .unwrap_or("完成")
                .to_string();

            let _ = tx.send(
                WsMessage::Done {
                    message: final_text,
                    apk_url: apk_url.clone(),
                    image_url: None,
                    model_used: Some(active_agent.model.clone()),
                    node_id: None,
                }
                .to_json(),
            );
            return Ok(());
        }

        // 处理工具调用
        if !tool_calls.is_empty() {
            for tool_call in &tool_calls {
                let tool_id = tool_call.id.clone();
                let tool_name = tool_call.name.clone();
                let args = tool_call.args.clone();

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
                        WsMessage::progress(format!(
                            "正在通过 PC agent 构建 {}(实时输出将陆续显示)...",
                            target
                        ))
                        .to_json(),
                    );
                    let r =
                        tools::build_project_via_agent(state, target, &changelog, Some(tx)).await;
                    if let Err(ref e) = r {
                        warn!("PC agent 构建失败，回退到服务器本地构建: {}", e);
                        let _ = tx.send(
                            WsMessage::progress(format!(
                                "PC agent 不可用（{}），尝试服务器本地构建...",
                                e
                            ))
                            .to_json(),
                        );
                        execute_tool(
                            state,
                            &workspace,
                            &active_agent,
                            &tool_name,
                            &args,
                            user_id,
                            trace_id,
                        )
                    } else {
                        r
                    }
                } else {
                    execute_tool(
                        state,
                        &workspace,
                        &active_agent,
                        &tool_name,
                        &args,
                        user_id,
                        trace_id,
                    )
                };

                let result_str = match result {
                    Ok(r) => {
                        // build_project 成功后提取 APK 文件名，生成下载链接
                        if tool_name == "build_project" {
                            if let Some(line) = r.lines().find(|l| l.starts_with("##APK_FILE:")) {
                                let _apk_name = line.trim_start_matches("##APK_FILE:").trim();
                                apk_url = Some(tools::stable_apk_url(download_base));
                                let _ = tx.send(
                                    WsMessage::progress(format!(
                                        "APK 编译成功，正在生成下载链接..."
                                    ))
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
                if tool_call.legacy_function_call {
                    messages.push(json!({
                        "role": "function",
                        "name": tool_name,
                        "content": result_str
                    }));
                } else {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tool_id,
                        "content": result_str
                    }));
                }
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
            model_used: Some(active_agent.model.clone()),
            node_id: None,
        }
        .to_json(),
    );

    Ok(())
}

async fn run_api_plan(
    state: &Arc<AppState>,
    agent: &crate::types::AgentConfig,
    allow_agent_fallback: bool,
    user_id: &str,
    workspace: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let _ = tx.send(
        WsMessage::progress(format!(
            "正在使用 AI 代理规划: {} ({})",
            agent.name, agent.model
        ))
        .to_json(),
    );
    let note = preflight_note.unwrap_or("无");
    let messages = vec![
        json!({
            "role": "system",
            "content": "你是一龙项目规划助手。当前是 Plan 模式：只生成计划，不调用工具，不修改文件，不构建、不提交、不发布。输出中文，给小白也能看懂。"
        }),
        json!({
            "role": "user",
            "content": format!(
                "当前项目目录：{}\n项目预检提示：{}\n\n用户请求：{}\n\n请输出：1. 我理解的目标 2. 推荐方案 3. 需要改动的模块或页面 4. 实施步骤 5. 验证与发布方式 6. 需要确认的问题。结尾提醒用户确认后发送「按这个计划开始实现」。",
                workspace, note, user_message
            )
        }),
    ];
    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        agent,
        allow_agent_fallback,
        &messages,
        user_id,
        "plan",
        0.8,
        700,
    )
    .await?;
    if used_fallback {
        let _ = tx.send(
            WsMessage::progress(format!(
                "默认 AI 通道不可用，已切换备用 AI 通道: {} ({})",
                used_agent.name, used_agent.model
            ))
            .to_json(),
        );
    }
    let reply = response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("计划生成完成。确认后发送「按这个计划开始实现」。")
        .to_string();
    let _ = tx.send(
        WsMessage::Done {
            message: reply,
            apk_url: None,
            image_url: None,
            model_used: Some(used_agent.model),
            node_id: None,
        }
        .to_json(),
    );
    Ok(())
}
