use anyhow::Result;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    ai_cli,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    store::ProjectAccess,
    tools,
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

/// 一龙自项目路径（默认 /root/Elon，可由 ELON_SELF_PATH 环境变量覆盖）
pub fn elon_self_workspace() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("ELON_SELF_PATH").unwrap_or_else(|_| "/root/Elon".into()),
    )
}

/// 系统提示词（告诉 LLM 它的角色和规则）
fn system_prompt(workspace: &str) -> String {
    format!(
        r#"你是「一龙」云端 Git 项目开发平台的 AI 编程助手。
用户通过手机描述需求，你负责在服务器上的项目工作区里读取说明、修改代码、验证、提交，并在需要时编译 APK 或服务端。

用户工作区: {workspace}
（这是当前项目目录。它可能是模板项目、GitHub 导入项目，也可能是平台自身源码；不要用特殊旁路处理，先按普通 Git 项目理解。）

=== 项目说明读取顺序 ===
进入任何项目后，优先 list_dir(".") 观察结构；如果存在以下文件，先读取再行动：
- AGENTS.md
- CODEX.md
- .github/copilot-instructions.md
- .github/instructions/*.md
- README.md
- docs/ 中与任务相关的文档

=== 通用项目工作流（所有 APK 项目通用）===
1. 先确认当前目录、Git 状态、origin、当前分支和远端写权限；local_path/GitHub 项目如果不是可读写 Git 仓库，必须明确报错。
2. 新项目或未知项目必须先读取项目自己的说明文档；如果项目没有说明文档，使用平台默认流程，但不要假装已经拥有项目记忆。
3. 一龙自项目只是普通项目；不要把它当特殊路径，也不要把它的发布规则套到无关项目，除非该项目文档明确要求。
4. 同一项目的共享工作区由服务器排队保护；以后 task worktree 可让编码阶段并行，但 merge、版本号递增、APK 发布、服务器部署必须串行。
5. 如果本次发现项目缺少必要流程说明，应建议用户补充 AGENTS.md/CODEX.md/README，而不是靠 CLI 记忆。

=== 开发流程 ===
1. **新项目**（工作区为空时）:
   - 先调用 init_project("android") 初始化模板
   - 读取模板中的关键文件了解结构
   - 用 write_file 修改/新增文件实现用户功能
   - git_commit 提交
   - build_project("android") 编译打包

2. **已有 Git 项目**（继续迭代）:
   - list_dir(".") 查看当前结构
   - 读取项目说明文档和目标文件
   - read_file 读取要修改的文件
   - write_file 写入修改
   - git_commit 提交
   - 按项目类型运行 build_project("android" / "rust" / "frontend") 或必要检查

=== Android 项目关键文件 ===
- settings.gradle           → 应用名称
- app/build.gradle          → 包名(applicationId)、SDK版本、依赖
- app/src/main/AndroidManifest.xml → 权限、Activity 声明
- app/src/main/kotlin/...   → Kotlin 代码（主要写这里）
- app/src/main/res/layout/  → XML 布局文件
- app/src/main/res/values/  → strings.xml, colors.xml

=== 规则 ===
- 如果用户需求不是“修改/生成/构建项目代码”，不要调用工具。请直接用简洁中文回复。
- 只有在确实需要读取、修改、提交或构建项目时，才调用 read_file/write_file/git_commit/build_project 等工具。
- 修改文件前必须先 read_file 读取原内容
- 每完成一个功能点就 git_commit（中文描述）
- git_commit 会自动执行 git push（如工作区有配置 origin）
- build_project("android") 会自动递增 versionCode 和 versionName，无需手动改版本号
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

pub async fn run_for_project(
    user_id: &str,
    project: &ProjectAccess,
    download_base: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let user_config_workspace = state.get_user_workspace(user_id);
    let require_existing_git = matches!(project.source_type.as_str(), "local_path" | "github");
    if require_existing_git && (!workspace.join(".git").exists() || !has_origin_remote(&workspace))
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
    if require_existing_git {
        match tools::git_pull_rebase(&workspace) {
            Ok(msg) => {
                if msg.starts_with("git pull 未成功") {
                    let _ = tx.send(WsMessage::Error { message: msg }.to_json());
                    return;
                }
                let _ = tx.send(WsMessage::Progress { message: msg }.to_json());
            }
            Err(e) => {
                let _ = tx.send(
                    WsMessage::Error {
                        message: format!("git pull --rebase 失败: {}", e),
                    }
                    .to_json(),
                );
                return;
            }
        }
    }
    if let Err(e) = run_dispatch_with_workspace(
        user_id,
        &workspace,
        &user_config_workspace,
        download_base,
        user_message,
        agent_name,
        require_existing_git,
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

async fn run_dispatch(
    user_id: &str,
    workspace_user_id: &str,
    user_message: &str,
    agent_name: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let workspace = state.get_user_workspace(workspace_user_id);
    let user_config_workspace = state.get_user_workspace(user_id);
    let download_base = format!("{}/download/{}", state.public_url, workspace_user_id);
    run_dispatch_with_workspace(
        user_id,
        &workspace,
        &user_config_workspace,
        &download_base,
        user_message,
        agent_name,
        false,
        state,
        tx,
    )
    .await
}

async fn run_dispatch_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    user_message: &str,
    agent_name: Option<&str>,
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
            || decision.route == CapabilityRoute::ChatAgent
        {
            let _ = tx.send(
                WsMessage::Progress {
                    message: "当前已锁定使用 Codex CLI，不切换到其他 AI 代理。".into(),
                }
                .to_json(),
            );
        }
        CapabilityRoute::CodeAgent
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
        user_message,
        backend_agent_name,
        backend_route,
        !(codex_cli_only || image_cli_only),
        require_existing_git,
        state,
        tx,
    )
    .await
}

fn is_short_resume_command(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    if normalized.contains("继续完成上一次未完成")
        || normalized.contains("未完成的开发任务")
        || normalized.contains("继续当前项目")
        || (normalized.contains("检查当前项目状态") && normalized.contains("apk"))
    {
        return true;
    }
    matches!(
        normalized.as_str(),
        "继续"
            | "继续吧"
            | "继续开发"
            | "继续做"
            | "继续完成"
            | "重试"
            | "再试一次"
            | "重新开始"
            | "再来一次"
    )
}

fn is_project_delivery_request(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    let asks_for_apk = normalized.contains("apk")
        || normalized.contains("安装包")
        || normalized.contains("下载包");
    let asks_for_delivery = normalized.contains("地址")
        || normalized.contains("链接")
        || normalized.contains("下载")
        || normalized.contains("发给我")
        || normalized.contains("给我")
        || normalized.contains("做好")
        || normalized.contains("做完")
        || normalized.contains("完成");

    asks_for_apk && asks_for_delivery
}

fn is_short_build_command(user_message: &str, workspace: &Path) -> bool {
    if !is_project_workspace(workspace) {
        return false;
    }
    let normalized = user_message.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "打包" | "编译" | "生成apk" | "生成 apk" | "打包apk" | "打包 apk"
    )
}

fn is_project_workspace(workspace: &Path) -> bool {
    workspace.join(".git").exists()
        || workspace.join("gradlew").exists()
        || workspace.join("android").join("gradlew").exists()
        || workspace.join("Cargo.toml").exists()
        || workspace.join("server").join("Cargo.toml").exists()
        || workspace.join("package.json").exists()
        || workspace
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.contains("__"))
            .unwrap_or(false)
}

fn has_origin_remote(workspace: &Path) -> bool {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

async fn run_backend_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    user_message: &str,
    agent_name: Option<&str>,
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
                cli_option_id(agent_name),
                require_existing_git,
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
        return AiBackend::Api;
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
