use anyhow::Result;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::store::Store;

// AI 代理配置类型通过 agent_config 模块统一管理，此处重导出保持路径兼容
pub use crate::agent_config::{
    AgentConfig, AgentsConfig, AiBackend, ImageModelConfig, UserAgentConfig,
};

// ── P2P 同WiFi中继 ──────────────────────────────────────────────

/// 下载方向 seeder 发起的传输请求
pub struct PeerRequest {
    /// Seeder 把 APK 数据通过此通道回传给中继 handler
    pub response_tx: oneshot::Sender<Result<Vec<u8>, String>>,
}

/// 已注册的 seeder 节点信息
pub struct PeerEntry {
    /// 该 seeder 当前安装的 APK versionCode
    pub version_code: i64,
    /// 向该 seeder 发送传输请求的通道
    pub tx: mpsc::Sender<PeerRequest>,
}

/// 同WiFi 局域网 PC 种子节点（开发电脑发布任意产物后注册，直接对手机/客户端提供 HTTP 下载）
pub struct LanPeerEntry {
    /// PC 在局域网中的 IP 地址（如 192.168.1.100）
    pub lan_ip: String,
    /// PC 上本地 HTTP 文件服务器监听的端口
    pub port: u16,
    /// 该 PC 提供的产物版本号（APK versionCode 或其他整数版本）
    pub version_code: i64,
    /// 产物在本地 HTTP 服务器上的路径（如 "/dist/elon/user-apk"，旧版默认 "/apk"）
    pub dist_path: String,
    /// 注册时间（用于自动过期）
    pub registered_at: std::time::Instant,
}

pub use crate::cli_config::{AiCliConfig, AiCliOption, CliPromptMode};
pub use crate::ws_message::WsMessage;

/// 全局状态，在各路由间共享
pub struct AppState {
    /// SQLite 数据层，保存用户、会话、项目、任务等产品级状态
    pub store: Store,
    /// 数据目录（默认 /opt/elon/data）
    pub data_dir: std::path::PathBuf,
    /// 默认用户消息后端
    pub default_backend: AiBackend,
    /// 本地 AI CLI 配置
    pub ai_cli: AiCliConfig,
    /// AI 代理配置（RwLock 支持运行时通过管理后台修改，无需重启）
    pub agents_config: RwLock<AgentsConfig>,
    /// 用户工作区根目录（每个用户在此独立目录开发自己的项目）
    pub project_root: std::path::PathBuf,
    /// 用户工作区根目录字符串（冗余保存，方便直接传给工具层）
    pub workspace_root: String,
    /// 服务器对外公开的 URL（用于生成 APK 下载链接）
    pub public_url: String,
    /// HTTP 客户端（复用连接）
    pub http_client: reqwest::Client,
    /// 管理后台访问令牌（对应 .env 中的 ADMIN_TOKEN）
    pub admin_token: String,
    /// 是否强制所有 WebSocket 连接需要登录 token（ENV: REQUIRE_LOGIN=true）
    /// 设为 true 后，旧版无登录功能的 APK 将被拒绝连接
    pub require_login: bool,
    /// 最低允许接入的 APK versionCode（ENV: MIN_APK_VERSION_CODE，0=不限）
    /// 低于此版本的 APK 建立 WS 连接时会收到升级提示并被断开
    pub min_apk_version_code: i64,
    /// agents.json 持久化路径
    pub config_path: std::path::PathBuf,
    /// 可选文生图模型配置
    pub image_model: Option<ImageModelConfig>,
    /// 已注册的同WiFi种子节点（peer_id → PeerEntry）
    pub peer_registry: Arc<RwLock<HashMap<String, PeerEntry>>>,
    /// 已注册的局域网 PC 种子节点（peer_id → LanPeerEntry）
    pub lan_peer_registry: Arc<RwLock<HashMap<String, LanPeerEntry>>>,
    /// 分布式计算节点注册中心（node_id → NodeEntry，含 LLM 能力与在线状态）
    pub node_registry: Arc<crate::node_registry::NodeRegistry>,
    /// 当前通过 /ws/app 保持认证连接的用户 ID 与连接数（用于好友在线状态）
    pub online_users: Arc<tokio::sync::RwLock<HashMap<String, usize>>>,
    /// 反向 WSS 通道接入的 homecli PC agents（agent_id → AgentEntry）
    pub agent_manager: Arc<crate::homecli_agent::AgentManager>,
    /// Short-lived execution gates keyed by project/conversation/merge scope.
    /// Conversation worktrees allow coding work to run in parallel; merge/publish
    /// keys still serialize shared project state.
    pub project_task_scheduler: Arc<ProjectTaskScheduler>,
    /// Best-effort Codex CLI native-session prewarm throttle, scoped by project,
    /// user, conversation, agent, and workspace.
    pub codex_prewarm: Arc<CodexPrewarmRegistry>,
    /// Hot Route A runtime lease: recent verified PC node + workspace + CLI
    /// readiness, scoped by project/user/conversation/node/workspace.
    pub route_a_session_leases: Arc<RouteASessionLeaseRegistry>,
    /// Cached Codex CLI network health and circuit-breaker state.
    pub codex_network: Arc<crate::codex_health::CodexNetworkHealth>,
    /// Short-lived in-memory debug events keyed by client trace_id.
    pub server_traces: Arc<crate::server_trace::ServerTraceStore>,
    /// 本地模式 owner token（ENV: OWNER_TOKEN）。
    /// 设置后，携带该 token 的请求将以固定的 owner 身份通过认证，
    /// 无需在 SQLite 中注册账号。用于 Windows 本机单用户模式。
    pub owner_token: Option<String>,
}

pub use crate::project_task_scheduler::{
    CodexPrewarmRegistry, ProjectTaskScheduler, RouteASessionLeaseRegistry,
};

impl AppState {
    pub fn new() -> Result<Self> {
        let ai_cli = AiCliConfig::from_env();
        let requested_backend = AiBackend::from_env_value(
            &std::env::var("AI_BACKEND").unwrap_or_else(|_| "local_cli".into()),
        );
        let default_backend = if requested_backend == AiBackend::LocalCli && ai_cli.enabled {
            AiBackend::LocalCli
        } else {
            AiBackend::Api
        };

        let mut agents: HashMap<String, AgentConfig> = HashMap::new();

        let providers = [
            ("OPENAI", "https://api.openai.com/v1", "gpt-4o"),
            ("DEEPSEEK", "https://api.deepseek.com/v1", "deepseek-chat"),
            (
                "CLAUDE",
                "https://api.anthropic.com/v1",
                "claude-3-5-sonnet-20241022",
            ),
            (
                "HUNYUAN",
                "https://api.hunyuan.cloud.tencent.com/v1",
                "hunyuan-turbo",
            ),
            (
                "TOKENHUB",
                "https://tokenhub.tencentmaas.com/v1",
                "hunyuan-2.0-instruct-20251111",
            ),
            ("CUSTOM", "", ""),
        ];

        for (prefix, default_base, default_model) in providers {
            if let Some(cfg) = AgentConfig::from_env(prefix, default_base, default_model) {
                tracing::info!("已加载 AI 代理: {} -> {}", cfg.name, cfg.api_base);
                agents.insert(cfg.name.clone(), cfg);
            }
        }

        // 从 COPILOT_GITHUB_TOKEN / GITHUB_TOKEN 自动加载 Copilot 多模型代理
        for cfg in crate::cli_config::copilot_api_agents() {
            tracing::info!(
                "已加载 Copilot 代理: {} -> {} (model: {})",
                cfg.name,
                cfg.api_base,
                cfg.model
            );
            agents.entry(cfg.name.clone()).or_insert(cfg);
        }

        let config_path = std::path::PathBuf::from(
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./agents.json".into()),
        );

        // 优先从 agents.json 加载（管理后台保存的配置），否则用 .env
        let mut agents_config = if let Some(mut saved) = AgentsConfig::load_from_file(&config_path)
        {
            tracing::info!("从 {} 加载代理配置", config_path.display());
            // 将 .env 中有但 agents.json 没有的代理补充进来
            for (k, v) in &agents {
                saved.agents.entry(k.clone()).or_insert_with(|| v.clone());
            }
            saved
        } else {
            if agents.is_empty() && default_backend == AiBackend::Api {
                anyhow::bail!(
                    "至少需要配置一个 AI 代理，请设置 AGENT_OPENAI_KEY / AGENT_DEEPSEEK_KEY / AGENT_HUNYUAN_KEY 等"
                );
            }
            let default_agent = std::env::var("DEFAULT_AGENT")
                .ok()
                .or_else(|| agents.keys().next().cloned())
                .or_else(|| ai_cli.default_option.clone())
                .unwrap_or_else(|| "local_cli".into());
            AgentsConfig {
                agents,
                default_agent,
            }
        };

        if agents_config.agents.is_empty() && default_backend == AiBackend::Api {
            anyhow::bail!("AI_BACKEND=api 时至少需要配置一个 AGENT_* API 代理");
        }
        if !agents_config
            .agents
            .contains_key(&agents_config.default_agent)
        {
            if let Some(first) = agents_config.agents.keys().next().cloned() {
                agents_config.default_agent = first;
            }
        }

        let data_dir = std::path::PathBuf::from(
            std::env::var("DATA_DIR").unwrap_or_else(|_| "/opt/elon/data".into()),
        );
        let database_path = std::path::PathBuf::from(
            std::env::var("DATABASE_PATH")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| data_dir.join("elon.db").to_string_lossy().to_string()),
        );
        let store = Store::open(&database_path)?;

        let project_root_str =
            std::env::var("WORKSPACE_ROOT").unwrap_or_else(|_| "/opt/elon/workspaces".into());

        let public_url =
            std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://43.139.149.158:8080".into());

        let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| {
            tracing::warn!("未设置 ADMIN_TOKEN，使用默认值 'elon-admin'，生产环境请修改！");
            "elon-admin".into()
        });

        let require_login = std::env::var("REQUIRE_LOGIN")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let min_apk_version_code = std::env::var("MIN_APK_VERSION_CODE")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        if require_login {
            tracing::info!("已启用强制登录：旧版无登录 APK 将被拒绝");
        }
        if min_apk_version_code > 0 {
            tracing::info!("最低 APK 版本门控: versionCode >= {}", min_apk_version_code);
        }

        tracing::info!("默认 AI 后端: {}", default_backend.as_str());
        if ai_cli.enabled {
            tracing::info!(
                "已加载本地 AI CLI 选项: {}",
                ai_cli
                    .options
                    .iter()
                    .map(|opt| format!("{} -> {}", opt.id, opt.command_preview()))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        } else {
            tracing::warn!("本地 AI CLI 未启用或没有可用选项，将使用 API 代理");
        }
        if agents_config.agents.is_empty() {
            tracing::warn!("未配置 API 代理；显式切换到 API 时会返回错误");
        } else {
            tracing::info!("默认 API 代理: {}", agents_config.default_agent);
        }
        tracing::info!("用户工作区根目录: {}", project_root_str);
        tracing::info!("公开 URL: {}", public_url);

        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(120))
            .build()?;

        let image_model = ImageModelConfig::from_env();
        if let Some(cfg) = &image_model {
            tracing::info!("已加载文生图模型: {} -> {}", cfg.model, cfg.api_base);
        } else {
            tracing::warn!("未配置 IMAGE_API_KEY，文生图能力暂不可用");
        }

        let owner_token = std::env::var("OWNER_TOKEN")
            .ok()
            .filter(|v| !v.trim().is_empty());
        if let Some(ref tok) = owner_token {
            tracing::info!(
                "已启用本地 owner token（前8位: {}…），本机单用户模式",
                &tok[..tok.len().min(8)]
            );
        }

        Ok(Self {
            store,
            data_dir,
            default_backend,
            ai_cli,
            agents_config: RwLock::new(agents_config),
            project_root: std::path::PathBuf::from(&project_root_str),
            workspace_root: project_root_str,
            public_url,
            http_client,
            admin_token,
            require_login,
            min_apk_version_code,
            config_path,
            image_model,
            peer_registry: Arc::new(RwLock::new(HashMap::new())),
            lan_peer_registry: Arc::new(RwLock::new(HashMap::new())),
            node_registry: Arc::new(crate::node_registry::NodeRegistry::new()),
            online_users: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            agent_manager: Arc::new(crate::homecli_agent::AgentManager::new()),
            project_task_scheduler: Arc::new(ProjectTaskScheduler::new()),
            codex_prewarm: Arc::new(CodexPrewarmRegistry::new()),
            route_a_session_leases: Arc::new(RouteASessionLeaseRegistry::new()),
            codex_network: Arc::new(crate::codex_health::CodexNetworkHealth::from_env()),
            server_traces: Arc::new(crate::server_trace::ServerTraceStore::new()),
            owner_token,
        })
    }

    /// 获取某个用户的工作区目录（路径: {workspace_root}/{user_id}/）
    pub fn get_user_workspace(&self, user_id: &str) -> std::path::PathBuf {
        get_user_workspace(&self.workspace_root, user_id)
    }

    /// 获取项目级工作区目录（路径: {workspace_root}/projects/{workspace_key}/）
    pub fn get_project_workspace(&self, workspace_key: &str) -> std::path::PathBuf {
        self.project_root
            .join("projects")
            .join(safe_workspace_part(workspace_key, 64))
    }

    /// 解析项目实际工作区。GitHub/模板项目默认落在 workspace_root/projects 下；
    /// local_path 项目（例如一龙自身仓库）可以由项目记录指向固定路径。
    pub fn resolve_project_workspace(
        &self,
        workspace_key: &str,
        workspace_path: Option<&str>,
    ) -> std::path::PathBuf {
        workspace_path
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| self.get_project_workspace(workspace_key))
    }
}

/// 独立辅助函数：根据 workspace_root 和 user_id 计算用户工作区路径
pub fn get_user_workspace(workspace_root: &str, user_id: &str) -> std::path::PathBuf {
    let safe_id = safe_workspace_part(user_id, 128);
    std::path::PathBuf::from(workspace_root).join(if safe_id.is_empty() {
        "default".into()
    } else {
        safe_id
    })
}

fn safe_workspace_part(value: &str, max_len: usize) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(max_len)
        .collect()
}


#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
