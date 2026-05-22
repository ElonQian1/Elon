use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    ai_cli, image_generation, tools,
    types::{AgentConfig, AiBackend, AppState, UserAgentConfig, WsMessage},
};

/// AI 代理工具定义列表（告诉 LLM 它可以用哪些工具）
fn tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "init_project",
                "description": "在用户工作区初始化项目模板。用户第一次请求开发 Android 应用时必须先调用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_type": {
                            "type": "string",
                            "enum": ["android"],
                            "description": "项目类型，目前支持 android"
                        }
                    },
                    "required": ["project_type"]
                }
            }
        },
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
                            "description": "相对于项目根目录的文件路径，如 app/src/main/kotlin/com/template/app/MainActivity.kt"
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
fn system_prompt(workspace: &str) -> String {
    format!(
        r#"你是「一龙」云端 Android 应用开发平台的 AI 编程助手。
用户通过手机描述需求，你负责在服务器上帮他们开发完整的 Android APK，并编译好供下载体验。

用户工作区: {workspace}
（这是用户自己的项目目录，与平台代码完全隔离）

=== 开发流程 ===
1. **新项目**（工作区为空时）:
   - 先调用 init_project("android") 初始化模板
   - 读取模板中的关键文件了解结构
   - 用 write_file 修改/新增文件实现用户功能
   - git_commit 提交
   - build_project("android") 编译打包

2. **已有项目**（继续迭代）:
   - list_dir(".") 查看当前结构
   - read_file 读取要修改的文件
   - write_file 写入修改
   - git_commit 提交
   - build_project("android") 重新编译

=== Android 项目关键文件 ===
- settings.gradle           → 应用名称
- app/build.gradle          → 包名(applicationId)、SDK版本、依赖
- app/src/main/AndroidManifest.xml → 权限、Activity 声明
- app/src/main/kotlin/...   → Kotlin 代码（主要写这里）
- app/src/main/res/layout/  → XML 布局文件
- app/src/main/res/values/  → strings.xml, colors.xml

=== 规则 ===
- 如果用户需求不是“修改/生成/构建 Android APK 或项目代码”，不要调用工具。请直接用简洁中文回复用户，说明当前平台主要用于应用开发；例如图片、壁纸、闲聊、资料查询等需求，应直接回答能做什么/不能做什么。
- 只有在确实需要读取、修改、提交或构建用户项目时，才调用 read_file/write_file/git_commit/build_project 等工具。
- 修改文件前必须先 read_file 读取原内容
- 每完成一个功能点就 git_commit（中文描述）
- build_project 失败时分析错误，最多修复 3 次
- 回复用户用简洁中文，告知进度
- 编译成功后系统会自动生成下载链接给用户"#,
        workspace = workspace
    )
}

/// 运行 AI 代理的主循环
/// - user_id:      用户标识（决定工作区目录）
/// - user_message: 用户发送的消息
/// - agent_name:   可选，指定使用哪个 AI 代理
pub async fn run(
    user_id: &str,
    workspace_user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    if let Err(e) = run_dispatch(
        user_id,
        workspace_user_id,
        user_message,
        agent_name,
        state,
        &tx,
    )
    .await
    {
        error!("AI 代理运行出错: {}", e);
        let _ = tx.send(
            WsMessage::Error {
                message: e.to_string(),
            }
            .to_json(),
        );
    }
}

async fn run_dispatch(
    user_id: &str,
    workspace_user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    // 文生图是原有独立能力。配置存在时继续直接处理，避免 Codex CLI 只能返回文字说明。
    if state.image_model.is_some() && should_generate_image_directly(user_message) {
        return run_api_inner(
            user_id,
            workspace_user_id,
            user_message,
            agent_name,
            state,
            tx,
        )
        .await;
    }

    let user_config_workspace = state.get_user_workspace(user_id);
    let user_config = UserAgentConfig::load(&user_config_workspace);
    let backend = choose_backend(state, user_config.as_ref(), agent_name);

    match backend {
        AiBackend::LocalCli => {
            match ai_cli::run(
                user_id,
                workspace_user_id,
                user_message,
                cli_option_id(agent_name),
                state,
                tx,
            )
            .await
            {
                Ok(()) => Ok(()),
                Err(e) if state.ai_cli.fallback_to_api && has_api_agents(state).await => {
                    warn!("本地 AI CLI 执行失败，回退到 API 代理: {}", e);
                    let _ = tx.send(
                        WsMessage::Progress {
                            message: format!("本地 AI CLI 暂不可用，正在切换原 API 代理: {}", e),
                        }
                        .to_json(),
                    );
                    run_api_inner(
                        user_id,
                        workspace_user_id,
                        user_message,
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
            run_api_inner(
                user_id,
                workspace_user_id,
                user_message,
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
) -> AiBackend {
    if let Some(name) = agent_name {
        if is_local_cli_option(state, name) {
            return AiBackend::LocalCli;
        }
        if is_api_backend_alias(name) {
            return AiBackend::Api;
        }
        return AiBackend::Api;
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

fn is_standalone_image_request(message: &str) -> bool {
    let image_words = [
        "文生图",
        "生图",
        "生成图",
        "图像",
        "图片",
        "壁纸",
        "照片",
        "头像",
        "插画",
        "海报",
        "卡通",
        "山水画",
    ];

    image_words.iter().any(|word| message.contains(word))
}

fn looks_like_app_development_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    let app_words = [
        "app",
        "apk",
        "android",
        "应用",
        "功能",
        "页面",
        "界面",
        "按钮",
        "代码",
        "开发",
        "修改",
        "添加",
        "新增",
        "编译",
        "打包",
        "安装",
        "发布",
        "登录",
        "注册",
        "首页",
        "设置",
        "接口",
        "后端",
        "服务端",
        "数据库",
        "项目",
    ];

    app_words.iter().any(|word| lower.contains(word))
}

fn should_generate_image_directly(message: &str) -> bool {
    if looks_like_app_development_request(message) {
        return false;
    }

    let intent_words = [
        "文生图",
        "生图",
        "生成",
        "画",
        "绘制",
        "做一张",
        "来一张",
        "出一张",
        "创作",
    ];
    is_standalone_image_request(message) && intent_words.iter().any(|word| message.contains(word))
}

fn image_prompt_from_message(message: &str) -> String {
    let trimmed = message.trim();
    let prefixes = [
        "请帮我生成一张",
        "帮我生成一张",
        "帮我生成",
        "请生成一张",
        "请生成",
        "生成一张",
        "生成",
        "画一张",
        "画",
        "绘制一张",
        "绘制",
        "来一张",
        "做一张",
    ];

    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let prompt = rest.trim_start_matches(['：', ':', '，', ',', ' ']).trim();
            if !prompt.is_empty() {
                return prompt.to_string();
            }
        }
    }

    trimmed.to_string()
}

fn looks_like_development_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    let dev_words = [
        "app",
        "apk",
        "android",
        "应用",
        "功能",
        "页面",
        "界面",
        "按钮",
        "代码",
        "开发",
        "修改",
        "添加",
        "新增",
        "生成",
        "做一个",
        "做个",
        "编译",
        "打包",
        "安装",
        "发布",
        "登录",
        "注册",
        "首页",
        "设置",
        "接口",
        "后端",
        "服务端",
        "数据库",
    ];

    dev_words.iter().any(|word| lower.contains(word))
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

async fn run_api_inner(
    user_id: &str,
    workspace_user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    // 每个用户操作自己的工作区，不能访问其他用户目录
    let workspace = state.get_user_workspace(workspace_user_id);
    let user_config_workspace = state.get_user_workspace(user_id);

    if should_generate_image_directly(user_message) {
        let prompt = image_prompt_from_message(user_message);
        let model = state
            .image_model
            .as_ref()
            .map(|cfg| cfg.model.as_str())
            .unwrap_or("未配置");

        let _ = tx.send(
            WsMessage::Progress {
                message: format!("正在调用文生图模型: {}", model),
            }
            .to_json(),
        );

        let image = image_generation::generate_text_to_image(state, &prompt).await?;
        let message = format!(
            "图片已生成。\n\n提示词：{}\n图片链接：{}",
            prompt, image.url
        );

        let _ = tx.send(
            WsMessage::Done {
                message,
                apk_url: None,
                image_url: Some(image.url),
            }
            .to_json(),
        );
        return Ok(());
    }

    // 优先使用用户在 APP 里配置的专属代理；否则用管理员指定/默认全局代理。
    // 普通聊天也要走模型，否则体验会像固定话术。
    let agent = resolve_agent(state, &user_config_workspace, agent_name).await?;

    if !looks_like_development_request(user_message) {
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
    let workspace_str = workspace.to_string_lossy().to_string();

    let _ = tx.send(
        WsMessage::Progress {
            message: format!("正在使用 AI 代理: {} ({})", agent.name, agent.model),
        }
        .to_json(),
    );

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

                // 执行工具
                let result = execute_tool(state, &workspace, &tool_name, &args);

                let result_str = match result {
                    Ok(r) => {
                        // build_project 成功后提取 APK 文件名，生成下载链接
                        if tool_name == "build_project" {
                            if let Some(line) = r.lines().find(|l| l.starts_with("##APK_FILE:")) {
                                let apk_name = line.trim_start_matches("##APK_FILE:").trim();
                                let base_url = std::env::var("PUBLIC_URL")
                                    .unwrap_or_else(|_| "http://182.254.168.75:8080".into());
                                apk_url = Some(format!(
                                    "{}/download/{}/{}",
                                    base_url, workspace_user_id, apk_name
                                ));
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

/// 调用 LLM API（OpenAI 兼容接口）
async fn call_llm(state: &Arc<AppState>, agent: &AgentConfig, messages: &[Value]) -> Result<Value> {
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
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
            } else {
                anyhow::anyhow!("AI 请求失败: {}", e)
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    Ok(resp.json::<Value>().await?)
}

async fn call_chat_llm(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.8,
        "max_tokens": 700,
    });

    let resp = state
        .http_client
        .post(&url)
        .bearer_auth(&agent.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
            } else {
                anyhow::anyhow!("AI 请求失败: {}", e)
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    Ok(resp.json::<Value>().await?)
}

fn friendly_ai_api_error(status: reqwest::StatusCode, body: &str) -> String {
    let lower = body.to_lowercase();
    if status.as_u16() == 402
        || lower.contains("free_quota_exhausted")
        || lower.contains("payment required")
        || lower.contains("endpoint is inactive")
    {
        return "当前 AI 模型额度已用尽或接口不可用，请切换可用模型，或联系管理员补充额度后重试"
            .into();
    }
    if status.as_u16() == 401 || lower.contains("unauthorized") || lower.contains("invalid api key")
    {
        return "当前 AI 模型密钥无效或权限不足，请检查 AI 设置或切换可用模型".into();
    }
    if status.as_u16() == 429 || lower.contains("rate limit") || lower.contains("too many requests")
    {
        return "当前 AI 模型请求过于频繁，请稍后重试或切换可用模型".into();
    }
    if status.as_u16() >= 500 {
        return "AI 服务暂时不可用，请稍后重试".into();
    }

    let compact = body.lines().collect::<Vec<_>>().join(" ");
    let visible = compact.chars().take(120).collect::<String>();
    if visible.trim().is_empty() {
        format!("AI 服务返回错误 {}", status)
    } else {
        format!("AI 服务返回错误 {}：{}", status, visible)
    }
}

/// 根据工具名和参数，调用对应的工具函数
fn execute_tool(
    _state: &Arc<AppState>,
    workspace: &std::path::Path,
    tool_name: &str,
    args: &Value,
) -> Result<String> {
    match tool_name {
        "init_project" => {
            let project_type = args["project_type"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 project_type 参数"))?;
            tools::init_project(workspace, project_type)
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            tools::read_file(workspace, path)
        }
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;
            tools::write_file(workspace, path, content)
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            tools::list_dir(workspace, path)
        }
        "run_shell" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 command 参数"))?;
            tools::run_shell(workspace, command)
        }
        "git_commit" => {
            let message = args["message"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 message 参数"))?;
            tools::git_commit(workspace, message)
        }
        "build_project" => {
            let target = args["target"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 target 参数"))?;
            tools::build_project(workspace, target)
        }
        _ => Err(anyhow::anyhow!("未知工具: {}", tool_name)),
    }
}
