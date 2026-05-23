use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex, OwnedMutexGuard, RwLock};

use crate::store::Store;

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

/// 单个 AI 代理的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

impl AgentConfig {
    fn from_env(prefix: &str, default_base: &str, default_model: &str) -> Option<Self> {
        let key = std::env::var(format!("AGENT_{}_KEY", prefix)).ok()?;
        Some(Self {
            name: prefix.to_lowercase(),
            api_base: std::env::var(format!("AGENT_{}_BASE", prefix))
                .unwrap_or_else(|_| default_base.into()),
            api_key: key,
            model: std::env::var(format!("AGENT_{}_MODEL", prefix))
                .unwrap_or_else(|_| default_model.into()),
        })
    }
}

/// 文生图模型配置（TokenHub / 混元生图 3.0）
#[derive(Debug, Clone)]
pub struct ImageModelConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub poll_interval_secs: u64,
    pub max_attempts: usize,
}

impl ImageModelConfig {
    fn from_env() -> Option<Self> {
        let api_key = std::env::var("IMAGE_API_KEY")
            .or_else(|_| std::env::var("TOKENHUB_IMAGE_KEY"))
            .ok()?;

        Some(Self {
            api_base: std::env::var("IMAGE_API_BASE")
                .unwrap_or_else(|_| "https://tokenhub.tencentmaas.com/v1/api/image".into()),
            api_key,
            model: std::env::var("IMAGE_MODEL").unwrap_or_else(|_| "hy-image-v3.0".into()),
            poll_interval_secs: std::env::var("IMAGE_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            max_attempts: std::env::var("IMAGE_MAX_ATTEMPTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
        })
    }
}

/// 默认处理用户消息的后端：本地 AI CLI 或原 OpenAI 兼容 API。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBackend {
    LocalCli,
    Api,
}

impl AiBackend {
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "api" | "llm" | "openai" | "remote" => Self::Api,
            _ => Self::LocalCli,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalCli => "local_cli",
            Self::Api => "api",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliPromptMode {
    Arg,
    Stdin,
}

impl CliPromptMode {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "stdin" => Self::Stdin,
            _ => Self::Arg,
        }
    }
}

/// 一个可在 APK 中选择的本地 AI CLI 模型/后端选项。
#[derive(Debug, Clone)]
pub struct AiCliOption {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub model: Option<String>,
    pub bin: String,
    pub args: Vec<String>,
    pub prompt_mode: CliPromptMode,
    pub timeout_secs: u64,
}

impl AiCliOption {
    pub fn command_preview(&self) -> String {
        let mut parts = vec![self.bin.clone()];
        parts.extend(self.args.clone());
        parts.push("<prompt>".into());
        parts.join(" ")
    }
}

/// 本地 AI CLI 配置，可同时挂 Codex CLI、Copilot CLI 等多个可选模型。
#[derive(Debug, Clone)]
pub struct AiCliConfig {
    pub enabled: bool,
    pub options: Vec<AiCliOption>,
    pub default_option: Option<String>,
    pub fallback_to_api: bool,
    /// 临时单通道模式：用户侧聊天、意图分析后的执行、代码协作都只走 Codex CLI。
    pub codex_cli_only: bool,
}

impl AiCliConfig {
    fn from_env() -> Self {
        let codex_cli_only = env_bool("AI_CODEX_CLI_ONLY", true);
        let mut options = cli_options_from_json_env();
        options.extend(provider_cli_options(
            "CODEX",
            "codex",
            "Codex CLI",
            "codex",
            "exec --sandbox workspace-write --skip-git-repo-check",
            true,
            "-m",
        ));
        options.extend(provider_cli_options(
            "COPILOT",
            "copilot",
            "Copilot CLI",
            "copilot",
            "",
            false,
            "--model",
        ));
        if codex_cli_only {
            options.retain(is_codex_cli_option);
        }

        let enabled = env_bool("AI_CLI_ENABLED", true) && !options.is_empty();
        let default_option = std::env::var("AI_CLI_DEFAULT")
            .ok()
            .filter(|id| options.iter().any(|opt| opt.id == *id))
            .or_else(|| options.first().map(|opt| opt.id.clone()));

        Self {
            enabled,
            options,
            default_option,
            fallback_to_api: !codex_cli_only
                && env_bool(
                    "AI_CLI_FALLBACK_TO_API",
                    env_bool("CODEX_CLI_FALLBACK_TO_API", true),
                ),
            codex_cli_only,
        }
    }

    pub fn find_option(&self, id: Option<&str>) -> Option<&AiCliOption> {
        let id = id
            .filter(|value| !value.trim().is_empty())
            .or(self.default_option.as_deref())?;
        self.options
            .iter()
            .find(|opt| opt.id.eq_ignore_ascii_case(id))
            .or_else(|| self.options.first())
    }

    pub fn has_option(&self, id: &str) -> bool {
        self.options
            .iter()
            .any(|opt| opt.id.eq_ignore_ascii_case(id))
    }
}

fn is_codex_cli_option(option: &AiCliOption) -> bool {
    let id = option.id.to_ascii_lowercase();
    let provider = option.provider.to_ascii_lowercase();
    let bin = option.bin.to_ascii_lowercase();
    id.contains("codex") || provider.contains("codex") || bin.contains("codex")
}

#[derive(Debug, Deserialize)]
struct AiCliOptionInput {
    id: String,
    label: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    bin: String,
    #[serde(default)]
    args: Vec<String>,
    args_text: Option<String>,
    prompt_mode: Option<String>,
    timeout_secs: Option<u64>,
}

fn cli_options_from_json_env() -> Vec<AiCliOption> {
    let raw = match std::env::var("AI_CLI_OPTIONS_JSON") {
        Ok(raw) if !raw.trim().is_empty() => raw,
        _ => return Vec::new(),
    };

    match serde_json::from_str::<Vec<AiCliOptionInput>>(&raw) {
        Ok(items) => items
            .into_iter()
            .filter_map(|item| {
                let args = if let Some(args_text) = item.args_text {
                    split_cli_args(&args_text)
                } else {
                    item.args
                };
                let provider = item.provider.unwrap_or_else(|| item.id.clone());
                let label = item.label.unwrap_or_else(|| {
                    item.model
                        .as_ref()
                        .map(|model| format!("{} / {}", provider, model))
                        .unwrap_or_else(|| provider.clone())
                });
                Some(AiCliOption {
                    id: item.id,
                    label,
                    provider,
                    model: item.model,
                    bin: item.bin,
                    args,
                    prompt_mode: CliPromptMode::from_env_value(
                        item.prompt_mode.as_deref().unwrap_or("arg"),
                    ),
                    timeout_secs: item.timeout_secs.unwrap_or(1800),
                })
            })
            .collect(),
        Err(e) => {
            tracing::warn!("AI_CLI_OPTIONS_JSON 解析失败: {}", e);
            Vec::new()
        }
    }
}

fn provider_cli_options(
    prefix: &str,
    provider: &str,
    default_label: &str,
    default_bin: &str,
    default_args: &str,
    default_enabled: bool,
    default_model_arg: &str,
) -> Vec<AiCliOption> {
    let enabled = env_bool(&format!("{}_CLI_ENABLED", prefix), default_enabled);
    if !enabled {
        return Vec::new();
    }

    let bin = std::env::var(format!("{}_CLI_BIN", prefix)).unwrap_or_else(|_| default_bin.into());
    let label =
        std::env::var(format!("{}_CLI_LABEL", prefix)).unwrap_or_else(|_| default_label.into());
    let args_raw =
        std::env::var(format!("{}_CLI_ARGS", prefix)).unwrap_or_else(|_| default_args.into());
    let prompt_mode = CliPromptMode::from_env_value(
        &std::env::var(format!("{}_CLI_PROMPT_MODE", prefix)).unwrap_or_else(|_| "arg".into()),
    );
    let timeout_secs = std::env::var(format!("{}_CLI_TIMEOUT_SECS", prefix))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    let model_arg = std::env::var(format!("{}_CLI_MODEL_ARG", prefix))
        .unwrap_or_else(|_| default_model_arg.into());

    let mut models =
        split_list(&std::env::var(format!("{}_CLI_MODELS", prefix)).unwrap_or_default());
    if models.is_empty() {
        if let Ok(model) = std::env::var(format!("{}_CLI_MODEL", prefix)) {
            if !model.trim().is_empty() {
                models.push(model.trim().to_string());
            }
        }
    }

    if models.is_empty() {
        return vec![AiCliOption {
            id: format!("{}_cli", provider),
            label,
            provider: provider.into(),
            model: None,
            bin,
            args: split_cli_args(&args_raw),
            prompt_mode,
            timeout_secs,
        }];
    }

    models
        .into_iter()
        .map(|model| AiCliOption {
            id: format!("{}:{}", provider, model),
            label: format!("{} / {}", label, model),
            provider: provider.into(),
            model: Some(model.clone()),
            bin: bin.clone(),
            args: args_for_model(&args_raw, &model, &model_arg),
            prompt_mode,
            timeout_secs,
        })
        .collect()
}

fn args_for_model(args_raw: &str, model: &str, model_arg: &str) -> Vec<String> {
    if args_raw.contains("{model}") {
        return split_cli_args(&args_raw.replace("{model}", model));
    }

    let mut args = Vec::new();
    if !model_arg.trim().is_empty() {
        args.extend(split_cli_args(model_arg));
        args.push(model.to_string());
    }
    args.extend(split_cli_args(args_raw));
    args
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "false" | "no" | "off")
        })
        .unwrap_or(default)
}

fn split_cli_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' | '\'' if quote == Some(ch) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(ch),
            c if c.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// 可动态修改的 AI 代理配置集合（被 RwLock 包裹，支持运行时热更新）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub agents: HashMap<String, AgentConfig>,
    pub default_agent: String,
}

impl AgentsConfig {
    /// 获取指定名称的代理，不存在则返回默认代理
    pub fn get_agent(&self, name: Option<&str>) -> Option<&AgentConfig> {
        let key = name.unwrap_or(&self.default_agent);
        self.agents
            .get(key)
            .or_else(|| self.agents.get(&self.default_agent))
            .or_else(|| self.agents.values().next())
    }

    /// 从 JSON 文件加载配置
    pub fn load_from_file(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 持久化配置到 JSON 文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

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
    /// agents.json 持久化路径
    pub config_path: std::path::PathBuf,
    /// 可选文生图模型配置
    pub image_model: Option<ImageModelConfig>,
    /// 已注册的同WiFi种子节点（peer_id → PeerEntry）
    pub peer_registry: Arc<RwLock<HashMap<String, PeerEntry>>>,
    /// 反向 WSS 通道接入的 homecli PC agents（agent_id → AgentEntry）
    pub agent_manager: Arc<crate::homecli_agent::AgentManager>,
    /// Project-scoped execution gates. Different projects can run in parallel; one
    /// project runs one workspace-mutating task at a time until per-task worktrees
    /// are introduced.
    pub project_task_scheduler: Arc<ProjectTaskScheduler>,
}

pub struct ProjectTaskScheduler {
    locks: AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

pub struct ProjectTaskPermit {
    was_queued: bool,
    _guard: OwnedMutexGuard<()>,
}

impl ProjectTaskScheduler {
    pub fn new() -> Self {
        Self {
            locks: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn acquire<F>(&self, project_id: &str, on_queued: F) -> ProjectTaskPermit
    where
        F: FnOnce(),
    {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(project_id.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        match lock.clone().try_lock_owned() {
            Ok(guard) => ProjectTaskPermit {
                was_queued: false,
                _guard: guard,
            },
            Err(_) => {
                on_queued();
                let guard = lock.lock_owned().await;
                ProjectTaskPermit {
                    was_queued: true,
                    _guard: guard,
                }
            }
        }
    }
}

impl ProjectTaskPermit {
    pub fn was_queued(&self) -> bool {
        self.was_queued
    }
}

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
            config_path,
            image_model,
            peer_registry: Arc::new(RwLock::new(HashMap::new())),
            agent_manager: Arc::new(crate::homecli_agent::AgentManager::new()),
            project_task_scheduler: Arc::new(ProjectTaskScheduler::new()),
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
mod tests {
    use super::{get_user_workspace, ProjectTaskScheduler};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };

    #[test]
    fn legacy_ws_workspace_keeps_project_suffix() {
        let workspace = get_user_workspace(
            "/tmp/elon",
            "82ee3288e852435c90ed2a609e474aaf__677b1bb2-09c9-419a-b998-960dd0539796",
        );

        assert_eq!(
            workspace.file_name().and_then(|name| name.to_str()),
            Some("82ee3288e852435c90ed2a609e474aaf__677b1bb2-09c9-419a-b998-960dd0539796")
        );
    }

    #[tokio::test]
    async fn project_task_scheduler_queues_same_project() {
        let scheduler = Arc::new(ProjectTaskScheduler::new());
        let first = scheduler.acquire("project-a", || {}).await;
        let queued_notice_sent = Arc::new(AtomicBool::new(false));

        let task_scheduler = scheduler.clone();
        let task_notice = queued_notice_sent.clone();
        let waiting_task = tokio::spawn(async move {
            let permit = task_scheduler
                .acquire("project-a", || {
                    task_notice.store(true, Ordering::SeqCst);
                })
                .await;
            permit.was_queued()
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(queued_notice_sent.load(Ordering::SeqCst));
        assert!(!waiting_task.is_finished());

        drop(first);
        assert!(waiting_task.await.unwrap());
    }

    #[tokio::test]
    async fn project_task_scheduler_allows_different_projects() {
        let scheduler = ProjectTaskScheduler::new();
        let _first = scheduler.acquire("project-a", || {}).await;
        let second = scheduler
            .acquire("project-b", || panic!("different projects must not queue"))
            .await;

        assert!(!second.was_queued());
    }
}

/// 用户自定义 AI 代理配置（存储在用户工作区 agent_config.json）
/// 各字段均可为 None，表示回退到全局配置对应的值
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAgentConfig {
    /// 使用指定名称的全局代理（None = 使用服务器默认）
    pub use_agent: Option<String>,
    /// 自定义 API 地址（None = 使用所选全局代理的地址）
    pub api_base: Option<String>,
    /// 自定义 API 密钥（None = 使用所选全局代理的密钥）
    pub api_key: Option<String>,
    /// 自定义模型名（None = 使用所选全局代理的模型）
    pub model: Option<String>,
    /// 用户昵称（可选，仅用于管理后台展示）
    pub nickname: Option<String>,
    /// 最后更新时间
    pub updated_at: Option<String>,
}

impl UserAgentConfig {
    /// 从用户工作区加载配置
    pub fn load(workspace: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(workspace.join("agent_config.json")).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 持久化到用户工作区
    pub fn save(&self, workspace: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(workspace)?;
        std::fs::write(
            workspace.join("agent_config.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    /// 用户是否设置了任何自定义配置
    pub fn has_config(&self) -> bool {
        self.use_agent.is_some()
            || self.api_base.is_some()
            || self.api_key.is_some()
            || self.model.is_some()
    }

    /// 解析为实际可使用的 AgentConfig（以全局代理为基础，用自定义值覆盖）
    pub fn resolve(&self, global: &AgentsConfig) -> Option<AgentConfig> {
        let base = global.get_agent(self.use_agent.as_deref())?.clone();
        Some(AgentConfig {
            name: format!("{}(用户自定义)", base.name),
            api_base: self.api_base.clone().unwrap_or(base.api_base),
            api_key: self.api_key.clone().unwrap_or(base.api_key),
            model: self.model.clone().unwrap_or(base.model),
        })
    }
}

/// WebSocket 消息格式（发给 APK）
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// AI 思考/操作进度
    Progress { message: String },
    /// AI 正在执行的工具
    ToolCall {
        tool: String,
        args: serde_json::Value,
    },
    /// 工具执行结果
    ToolResult { tool: String, result: String },
    /// 最终回复
    Done {
        message: String,
        apk_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image_url: Option<String>,
    },
    /// 发生错误
    Error { message: String },
}

impl WsMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"type":"error","message":"序列化失败"}"#.into())
    }
}
