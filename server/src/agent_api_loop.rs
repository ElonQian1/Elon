use anyhow::Result;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::{
    agent_llm_call::{call_chat_llm, call_llm, execute_tool},
    agent_prompts::system_prompt,
    agent_routing::{casual_chat_prompt, is_local_cli_option, quick_casual_reply},
    intent_router,
    tools,
    types::{AgentConfig, AppState, UserAgentConfig, WsMessage},
};

pub(crate) async fn resolve_agent(
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

pub(crate) async fn run_api_inner_with_workspace(
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
