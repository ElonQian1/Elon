use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    tools,
    types::{AppState, WsMessage},
};

/// AI 代理工具定义列表（告诉 LLM 它可以用哪些工具）
fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "读取项目中某个文件的内容。修改任何文件前必须先用此工具读取原内容。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的文件路径，如 server/src/main.rs"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "写入或修改项目中某个文件的完整内容。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的文件路径"
                        },
                        "content": {
                            "type": "string",
                            "description": "文件的完整新内容"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "列出项目中某个目录的文件列表，用于了解代码结构。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的目录路径，如 server/src"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_shell",
                "description": "执行允许的只读 shell 命令（cargo check/test/clippy、git log/diff/status、ls、cat、grep 等）。不可执行写操作命令。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "要执行的命令"
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "git_commit",
                "description": "将所有已修改的文件执行 git add -A && git commit。修改完所有文件后调用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "中文 commit message，格式：feat/fix/style: 描述用户需求"
                        }
                    },
                    "required": ["message"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "build_project",
                "description": "编译构建项目某个模块。target 可选: rust（编译服务端）、android（打包APK）、frontend（构建前端）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "enum": ["rust", "android", "frontend"],
                            "description": "构建目标"
                        }
                    },
                    "required": ["target"]
                }
            }
        }
    ])
}

/// 系统提示词（告诉 LLM 它的角色和规则）
fn system_prompt(project_root: &str) -> String {
    format!(
        r#"你是一龙云端APK开发平台的 AI 编程代理。
用户通过手机APK向你描述他们想要的功能，你负责修改服务器上的代码来实现。

项目根目录: {project_root}
项目结构:
- server/     Rust 服务端代码
- android/    Android APK 代码
- frontend/   前端 Web 代码（如有）
- scripts/    构建部署脚本

你的工作流程:
1. 先用 list_dir 了解项目结构
2. 用 read_file 读取要修改的文件（修改前必须先读）
3. 用 write_file 精确写入修改后的文件
4. 用 git_commit 提交变更（中文 commit message）
5. 如需要，用 build_project 触发编译

重要规则:
- 修改文件前必须先读取原内容，不允许盲改
- 不要删除用户未要求删除的功能
- 每次修改后必须 git commit
- 回复用户时用简洁的中文，说明你做了什么
- 如果编译失败，分析错误并修复，最多尝试3次"#,
        project_root = project_root
    )
}

/// 运行 AI 代理的主循环
/// - user_id:      用户标识（决定工作区目录）
/// - user_message: 用户发送的消息
/// - agent_name:   可选，指定使用哪个 AI 代理
pub async fn run(
    user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    if let Err(e) = run_inner(user_id, user_message, agent_name, state, &tx).await {
        error!("AI 代理运行出错: {}", e);
        let _ = tx.send(WsMessage::Error { message: e.to_string() }.to_json());
    }
}

async fn run_inner(
    user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let agent = state.get_agent(agent_name);
    // 每个用户操作自己的工作区，不能访问其他用户目录
    let workspace = state.get_user_workspace(user_id);
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
    let workspace_str = workspace.to_string_lossy().to_string();

    let _ = tx.send(WsMessage::Progress {
        message: format!("正在使用 AI 代理: {} ({})", agent.name, agent.model),
    }.to_json());

    // 初始化对话历史
    let mut messages = vec![
        json!({
            "role": "system",
            "content": system_prompt(&workspace_str)
        }),
        json!({
            "role": "user",
            "content": user_message
        }),
    ];

    let _ = tx.send(WsMessage::Progress {
        message: "AI 正在理解需求...".into(),
    }.to_json());

    // 工具调用循环（最多 20 轮，防止死循环）
    for _round in 0..20 {
        let response = call_llm(state, agent_name, &messages).await?;

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

            let _ = tx.send(WsMessage::Done {
                message: final_text,
                apk_url: None, // TODO: 编译完成后填入 APK 下载链接
            }.to_json());
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
                let tool_name = tool_call["function"]["name"].as_str().unwrap_or("").to_string();
                let args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                info!("工具调用: {} {:?}", tool_name, args);

                let _ = tx.send(WsMessage::ToolCall {
                    tool: tool_name.clone(),
                    args: args.clone(),
                }.to_json());

                // 执行工具
                let result = execute_tool(state, &workspace, &tool_name, &args);

                let result_str = match result {
                    Ok(r) => r,
                    Err(e) => format!("错误: {}", e),
                };

                let _ = tx.send(WsMessage::ToolResult {
                    tool: tool_name.clone(),
                    result: result_str[..result_str.len().min(500)].to_string(),
                }.to_json());

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

    let _ = tx.send(WsMessage::Done {
        message: "任务执行完毕".into(),
        apk_url: None,
    }.to_json());

    Ok(())
}

/// 调用 LLM API（OpenAI 兼容接口）
async fn call_llm(
    state: &Arc<AppState>,
    agent_name: Option<&str>,
    messages: &[Value],
) -> Result<Value> {
    let agent = state.get_agent(agent_name);
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "tools": tool_definitions(),
        "tool_choice": "auto",
    });

    let resp = state
        .http_client
        .post(&url)
        .bearer_auth(&agent.api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("AI API 返回错误 {}: {}", status, text));
    }

    Ok(resp.json::<Value>().await?)
}

/// 根据工具名和参数，调用对应的工具函数
fn execute_tool(
    _state: &Arc<AppState>,
    workspace: &std::path::Path,
    tool_name: &str,
    args: &Value,
) -> Result<String> {
    match tool_name {
        "read_file" => {
            let path = args["path"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            tools::read_file(workspace, path)
        }
        "write_file" => {
            let path = args["path"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            let content = args["content"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;
            tools::write_file(workspace, path, content)
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            tools::list_dir(workspace, path)
        }
        "run_shell" => {
            let command = args["command"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 command 参数"))?;
            tools::run_shell(workspace, command)
        }
        "git_commit" => {
            let message = args["message"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 message 参数"))?;
            tools::git_commit(workspace, message)
        }
        "build_project" => {
            let target = args["target"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 target 参数"))?;
            tools::build_project(workspace, target)
        }
        _ => Err(anyhow::anyhow!("未知工具: {}", tool_name)),
    }
}
