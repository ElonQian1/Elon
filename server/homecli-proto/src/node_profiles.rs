use serde::{Deserialize, Serialize};

/// PC 节点上报的单个模型能力描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub model_id: String,
    pub display_name: String,
    pub context_len: u32,
    pub provider: String,
    pub price_per_1k_credits: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevToolchainStatus {
    pub name: String,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// PC 节点硬件画像。所有字段都是可选的，便于旧节点/受限环境渐进上报。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeHardwareProfile {
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub cpu_brand: Option<String>,
    #[serde(default)]
    pub cpu_cores: Option<u32>,
    #[serde(default)]
    pub memory_total_bytes: Option<u64>,
    #[serde(default)]
    pub gpu_names: Vec<String>,
    #[serde(default)]
    pub gpu_memory_total_bytes: Option<u64>,
    #[serde(default)]
    pub disk_free_bytes: Option<u64>,
}

/// PC 节点提供的项目代码硬盘服务能力。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeStorageProfile {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub root_path: Option<String>,
    #[serde(default)]
    pub git_base_url: Option<String>,
    /// 节点是否支持通过云端 WebSocket relay 暴露 Git HTTP 访问。
    #[serde(default)]
    pub relay_git_url_enabled: bool,
    #[serde(default)]
    pub disk_free_bytes: Option<u64>,
}

/// PC 节点内置开发运行时能力。它描述“能否创建项目工作区”，不等同于 Codex/Copilot 等 AI CLI。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeDevRuntimeProfile {
    #[serde(default)]
    pub workspace_root_path: Option<String>,
    #[serde(default)]
    pub workspace_root_writable: bool,
    #[serde(default)]
    pub git_ready: bool,
    #[serde(default)]
    pub workspace_provision_ready: bool,
    #[serde(default)]
    pub dev_env_ready: bool,
    #[serde(default)]
    pub ai_cli_ready: bool,
    /// Route A: this PC has an installed coding CLI such as Codex, Claude,
    /// Gemini, or Copilot.
    #[serde(default)]
    pub route_a_ready: bool,
    /// Route B: this PC can call an OpenAI-compatible API with a local key.
    #[serde(default)]
    pub api_runtime_ready: bool,
    /// Route C: this PC can ask the Elon server for model calls while keeping
    /// file and command execution local.
    #[serde(default)]
    pub server_runtime_ready: bool,
    /// Route C cloud status summary returned by `/api/agent/runtime/status`.
    /// This is optional so old nodes and old servers remain wire-compatible.
    #[serde(default)]
    pub server_runtime_status: Option<serde_json::Value>,
    /// Route B/C share this local tool contract. Model calls may happen on the
    /// PC or on the server, but file writes, patches, commands, approvals, and
    /// workspace limits are enforced by the PC node unless the project runtime
    /// permission is explicitly `danger_full_access`.
    #[serde(default)]
    pub local_tool_contract: NodeDevRuntimeToolContract,
    #[serde(default)]
    pub toolchains: Vec<DevToolchainStatus>,
    #[serde(default)]
    pub issues: Vec<String>,
}

/// PC 节点生命周期摘要。节点每次注册/能力刷新时都会带上它，
/// 让云端和 PC 网页在重连后知道上一轮会话是否异常结束。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeLifecycleReport {
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
    #[serde(default)]
    pub heartbeat_at_ms: Option<u64>,
    #[serde(default)]
    pub heartbeat_age_ms: Option<u64>,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub logged_in: bool,
    #[serde(default)]
    pub last_event: Option<String>,
    #[serde(default)]
    pub previous_session_id: Option<String>,
    #[serde(default)]
    pub previous_exit_kind: Option<String>,
    #[serde(default)]
    pub previous_exit_reason: Option<String>,
    #[serde(default)]
    pub previous_heartbeat_at_ms: Option<u64>,
    #[serde(default)]
    pub previous_heartbeat_age_ms: Option<u64>,
    #[serde(default)]
    pub active_task_count: u32,
    #[serde(default)]
    pub sidecar_session_count: u32,
    #[serde(default)]
    pub restart_recovery: bool,
    #[serde(default)]
    pub recommended_action: String,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeDevRuntimeToolContract {
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub supported_tools: Vec<String>,
    #[serde(default)]
    pub approval_required_tools: Vec<String>,
    #[serde(default)]
    pub path_policy: Option<String>,
    #[serde(default)]
    pub command_policy: Option<String>,
    #[serde(default)]
    pub audit_policy: Option<String>,
    #[serde(default)]
    pub recovery_policy: Option<String>,
}
