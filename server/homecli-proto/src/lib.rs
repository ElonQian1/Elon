use serde::{Deserialize, Serialize};
mod project_workspace;
pub use project_workspace::*;
mod cli_durable_types;
pub use cli_durable_types::{
    CliCodexCredentialBinding, CliCompletionEnvelope, CliCompletionProducerIdentity,
};
mod android_device_host;
pub use android_device_host::{AndroidDeviceHostRequest, CAP_ANDROID_DEVICE_HOST_V1};
mod cancel;
pub use cancel::{CancelRequestAudit, InterruptionSource};
pub const PROTO_VERSION: u32 = 7;
/// The node applies project-scoped build-cache routing, admission, leases, and cleanup.
pub const CAP_PROJECT_BUILD_CACHE_V1: &str = "project_build_cache_v1";
mod project_workspace_status;
pub use project_workspace_status::{
    ProjectGitWorktreeAudit, ProjectGitWorktreeEntry, ProjectWorkspaceInspectStatus,
};

/// PC 节点上报的单个模型能力描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    /// 模型唯一 ID，如 "llama3:8b" 或 "lm_studio/qwen2"
    pub model_id: String,
    /// 用户可见显示名称
    pub display_name: String,
    /// 上下文长度（token 数）
    pub context_len: u32,
    /// 提供方："ollama" | "lm_studio" | "custom"
    pub provider: String,
    /// 每 1000 tokens 消耗的平台积分（节点所有者自定义）
    pub price_per_1k_credits: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgent {
    Exec {
        task_id: String,
        cli: String,
        args: Vec<String>,
        cwd: String,
        #[serde(default)]
        env: Vec<(String, String)>,
        /// Project ownership for node-local cache routing and quota admission.
        /// The server only dispatches project-scoped work to nodes advertising
        /// [`CAP_PROJECT_BUILD_CACHE_V1`].
        #[serde(default)]
        project_context: Option<CliProjectContext>,
    },
    Cancel {
        task_id: String,
        #[serde(flatten)]
        audit: CancelRequestAudit,
    },
    ToolApprovalDecision {
        req_id: String,
        approval_id: String,
        /// 单次派发 ID。ACK 必须带回同一个 ID，避免迟到 ACK 误确认后续重试。
        #[serde(default)]
        dispatch_id: String,
        decision: String,
    },
    /// The server has durably stored a replayable CLI completion.
    ///
    /// Nodes may remove or mark their local outbox row only when `accepted` is
    /// true. `deduplicated=true` means the same `event_id` had already reached
    /// durable server storage, so it is equally safe to acknowledge locally.
    CliCompletionAck {
        event_id: String,
        req_id: String,
        accepted: bool,
        #[serde(default)]
        deduplicated: bool,
        /// A rejected completion should remain pending only when this is true.
        #[serde(default)]
        retryable: bool,
        #[serde(default)]
        error: Option<String>,
    },
    Ping {
        nonce: Option<String>,
    },
    /// 云端把 APK 的 HTTP 请求转发给 PC 端本地 server 处理
    HttpRequest {
        /// 唯一请求 ID，用于匹配响应
        req_id: String,
        method: String,
        /// 相对路径，如 "/health" 或 "/api/me"
        path: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        /// base64 编码的 body（GET 等无 body 时为 None）
        body_b64: Option<String>,
    },
    /// Project-authorized shared Android device-host request. This is distinct
    /// from the legacy generic relay so a node can keep the local-admin token
    /// private and reject arbitrary `/api/pc-relay` traffic.
    AndroidDeviceHostRequest {
        request: AndroidDeviceHostRequest,
    },
    /// 云端把 AI 提示发给 PC，让 PC 用本地 CLI（copilot/codex）执行，流式返回结果
    CliPrompt {
        /// 唯一请求 ID，用于匹配 CliChunk/CliDone
        req_id: String,
        /// CLI 可执行文件名，如 "copilot" 或 "codex"
        cli: String,
        /// 额外参数（在 -p/prompt 之前），可为空
        #[serde(default)]
        extra_args: Vec<String>,
        /// 可选工作目录；为空时由 PC relay 使用自身默认工作目录
        #[serde(default)]
        cwd: Option<String>,
        /// 项目会话上下文。新版 PC 节点会用它把 CLI cwd 切到会话隔离 worktree；
        /// 服务端只会向声明 [`CAP_PROJECT_BUILD_CACHE_V1`] 的节点派发项目任务。
        #[serde(default)]
        project_context: Option<CliProjectContext>,
        /// Frozen credential source authorized before dispatch. Protocol-v5
        /// nodes reject cloud Codex work when this binding is missing/mismatched.
        #[serde(default)]
        codex_credential_binding: Option<CliCodexCredentialBinding>,
        /// True when execution depends on a shared/platform resource whose
        /// authorization must be continuously controlled by the cloud session.
        #[serde(default)]
        requires_cloud_control: bool,
        /// Frozen absolute RFC3339 authorization deadline.
        #[serde(default)]
        cloud_control_deadline: Option<String>,
        /// Server wall-clock time at which `cloud_control_ttl_ms` was frozen.
        #[serde(default)]
        cloud_control_issued_at: Option<String>,
        /// Remaining authorization TTL frozen immediately before dispatch.
        /// Protocol-v7 nodes convert this to a monotonic deadline on receipt.
        #[serde(default)]
        cloud_control_ttl_ms: Option<u64>,
        /// 完整的提示内容
        prompt: String,
    },
    /// 云端要求 PC 节点读取某个本机 CLI 任务的 journal / sidecar 恢复快照。
    ///
    /// 这个请求走节点 WS 内部协议，不复用本机管理 HTTP token；用于服务器重启后
    /// 把云端任务快照和本机 journal 恢复合同重新接上。
    InspectCliTaskJournal {
        req_id: String,
        task_id: String,
        #[serde(default)]
        since: usize,
        #[serde(default)]
        limit: usize,
    },
    /// 云端向 PC 节点发起 LLM 推理请求（流式）
    LlmStreamRequest {
        req_id: String,
        model: String,
        messages: Vec<serde_json::Value>,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
    /// 云端要求 PC 节点在受控根目录下创建一个新项目工作区。
    ProvisionProjectWorkspace {
        req_id: String,
        project_id: String,
        user_id: String,
        name: String,
        template: String,
        /// Optional authoritative Git remote for rebuilding the project on another PC node.
        #[serde(default)]
        repo_url: Option<String>,
        /// Optional branch to check out after clone/fetch. Missing means the remote default branch.
        #[serde(default)]
        branch: Option<String>,
    },
    /// 云端要求硬盘节点在本机存储根目录创建或复用一个项目裸 Git 仓库。
    PrepareProjectStorageRepo {
        req_id: String,
        project_id: String,
        user_id: String,
        name: String,
        #[serde(default)]
        branch: Option<String>,
        #[serde(default)]
        access_token: Option<String>,
        /// Whether the storage PC should also create a normal owner checkout
        /// next to the bare repo. This keeps the user's Windows PC as a real
        /// project entry even when another PC provides AI compute.
        #[serde(default)]
        prepare_worktree: bool,
    },
    /// 云端要求 PC 节点检查某个项目工作区是否仍可执行。
    InspectProjectWorkspace {
        req_id: String,
        workspace_path: String,
    },
    /// 云端要求 PC 节点审计某个 Git 仓库登记的所有 worktree。
    AuditProjectGitWorktrees {
        req_id: String,
        workspace_path: String,
    },
    /// 云端要求 PC 节点读取项目内固定 AI/说明文档。
    ReadProjectDocuments {
        req_id: String,
        workspace_path: String,
        #[serde(default)]
        seed_defaults: bool,
        #[serde(default)]
        catalog_only: bool,
    },
    /// Read one Markdown document on the PC that owns the project workspace.
    ReadProjectDocumentFile {
        req_id: String,
        workspace_path: String,
        document_path: String,
    },
    /// Create or replace one Markdown document with optimistic concurrency.
    WriteProjectDocumentFile {
        req_id: String,
        workspace_path: String,
        document_path: String,
        content: String,
        #[serde(default)]
        expected_revision: Option<String>,
    },
    /// 云端要求 PC 节点清理一个由平台创建的项目工作区。
    CleanupProjectWorkspace {
        req_id: String,
        project_id: String,
        workspace_path: String,
    },
    /// 云端把 TTS 合成请求转发给有 GPU 的 PC 节点处理
    TtsSynthesizeRequest {
        req_id: String,
        text: String,
        #[serde(default)]
        voice_id: Option<String>,
        #[serde(default)]
        emotion_id: Option<String>,
        #[serde(default)]
        intensity: Option<String>,
        #[serde(default)]
        provider: Option<String>,
    },
    /// 服务端通知 PC 节点检查并安装新版本（无感自动更新）。
    /// 节点收到后：已安装版本调用内置更新程序；否则自行下载替换并重启。
    UpdateClient {
        /// 可选：新版本号，仅用于日志。
        #[serde(default)]
        version: Option<String>,
        /// 可选：下载 URL（默认用服务端标准下载路径）。
        #[serde(default)]
        download_url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Register {
        agent_id: String,
        version: String,
        proto_version: u32,
        /// Optional protocol features implemented by this concrete node binary.
        /// Missing on legacy register frames and therefore defaults to empty.
        #[serde(default)]
        capabilities: Vec<String>,
        #[serde(default)]
        allowed_clis: Vec<String>,
        #[serde(default)]
        allowed_cwds: Vec<String>,
        /// 节点归属用户 ID（分布式节点功能，旧版 homecli 不发此字段）
        #[serde(default)]
        owner_user_id: Option<String>,
        /// PC 设备名，仅用于展示
        #[serde(default)]
        device_name: Option<String>,
        /// Win 端安装实例 ID，用于同账号下节点凭证幂等续约。
        #[serde(default)]
        install_id: Option<String>,
        /// PC 硬件画像，用于市场展示和算力供给筛选。
        #[serde(default)]
        hardware: Option<NodeHardwareProfile>,
        /// 项目代码硬盘服务能力；旧节点不发送也兼容。
        #[serde(default)]
        storage: Option<NodeStorageProfile>,
        /// PC 开发运行时能力；旧节点不发送时服务端会按 allowed_clis 做兼容推断。
        #[serde(default)]
        dev_runtime: Option<NodeDevRuntimeProfile>,
        /// PC 节点生命周期摘要；旧节点不发送也兼容。
        #[serde(default)]
        lifecycle: Option<NodeLifecycleReport>,
    },
    TaskStarted {
        task_id: String,
        pid: u32,
    },
    TaskStdout {
        task_id: String,
        data: String,
    },
    TaskStderr {
        task_id: String,
        data: String,
    },
    TaskExit {
        task_id: String,
        code: Option<i32>,
    },
    TaskError {
        task_id: String,
        message: String,
    },
    Pong {
        nonce: Option<String>,
    },
    /// PC 端本地 server 处理完 HTTP 请求后回传响应
    HttpResponse {
        req_id: String,
        status: u16,
        #[serde(default)]
        headers: Vec<(String, String)>,
        /// base64 编码的响应 body
        body_b64: Option<String>,
    },
    /// PC 端处理失败（如本地 server 未启动）
    HttpError {
        req_id: String,
        message: String,
    },
    /// PC 节点已收到 CLI 请求；后续仍会继续返回 CliChunk/CliDone。
    CliPromptAccepted {
        req_id: String,
        /// 本机节点实际解析到的 CLI 名称，例如 codex / copilot。
        #[serde(default)]
        cli: Option<String>,
        /// 本机节点最终使用的工作目录；为空表示使用节点默认目录。
        #[serde(default)]
        cwd: Option<String>,
        /// 本轮 Route A 权限，仅用于云端诊断和前端过程展示。
        #[serde(default)]
        runtime_permission: Option<String>,
    },
    /// PC CLI 执行的流式输出片段
    CliChunk {
        req_id: String,
        text: String,
    },
    /// PC CLI 执行完毕（最后一帧）
    CliDone {
        req_id: String,
        exit_ok: bool,
        #[serde(default)]
        error: Option<String>,
        /// Native CLI thread/session id. For Codex this can be opened by the
        /// desktop app as `codex://threads/<id>`. Old nodes omit this field.
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        prompt_tokens: Option<u64>,
        #[serde(default)]
        cached_input_tokens: Option<u64>,
        #[serde(default)]
        completion_tokens: Option<u64>,
        #[serde(default)]
        reasoning_tokens: Option<u64>,
        #[serde(default)]
        total_tokens: Option<u64>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        workspace_status: Option<CliWorkspaceStatus>,
    },
    /// Replay a completion that was persisted locally before the original
    /// WebSocket could deliver/confirm it. This message is intentionally not
    /// routed through the transient request `pending` map; the server handles it
    /// through its durable completion inbox and responds with
    /// `ServerToAgent::CliCompletionAck`.
    CliCompletionReplay {
        completion: CliCompletionEnvelope,
    },
    /// PC 节点返回某个本机 CLI 任务的 journal / sidecar 恢复快照。
    CliTaskJournalSnapshot {
        req_id: String,
        task_id: String,
        ok: bool,
        #[serde(default)]
        snapshot: Option<serde_json::Value>,
        #[serde(default)]
        error: Option<String>,
    },
    /// PC 节点确认 ToolApprovalDecision 是否已交给对应待审批调用。
    ToolApprovalDecisionAck {
        req_id: String,
        approval_id: String,
        dispatch_id: String,
        accepted: bool,
    },
    /// PC 节点上报本机支持的模型列表
    RegisterCapabilities {
        models: Vec<ModelCapability>,
        /// 能力刷新时更新本机可用的 AI/开发 CLI。
        #[serde(default)]
        allowed_clis: Vec<String>,
        /// 本机 TTS Worker HTTP 地址（如 http://127.0.0.1:5011），为空表示无 TTS 能力
        #[serde(default)]
        tts_worker_url: Option<String>,
        /// 能力刷新时顺带更新硬件画像，旧节点不发送也兼容。
        #[serde(default)]
        hardware: Option<NodeHardwareProfile>,
        /// 能力刷新时顺带更新硬盘服务状态。
        #[serde(default)]
        storage: Option<NodeStorageProfile>,
        /// 能力刷新时顺带更新 PC 开发运行时状态。
        #[serde(default)]
        dev_runtime: Option<NodeDevRuntimeProfile>,
        /// 能力刷新时顺带更新 PC 生命周期摘要。
        #[serde(default)]
        lifecycle: Option<NodeLifecycleReport>,
    },
    /// LLM 推理流式输出片段
    LlmStreamChunk {
        req_id: String,
        delta: String,
    },
    /// LLM 推理完成（最后一帧）
    LlmStreamEnd {
        req_id: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        finish_reason: String,
    },
    /// LLM 推理出错
    LlmStreamError {
        req_id: String,
        message: String,
    },
    /// PC 节点已创建或复用项目工作区。
    ProjectWorkspaceProvisioned {
        req_id: String,
        project_id: String,
        workspace_path: String,
        #[serde(default)]
        git_head: Option<String>,
        #[serde(default)]
        git_remote_origin: Option<String>,
        #[serde(default)]
        git_branch: Option<String>,
        created: bool,
    },
    /// PC 节点创建项目工作区失败。
    ProjectWorkspaceProvisionError {
        req_id: String,
        project_id: String,
        message: String,
    },
    /// 硬盘节点已创建或复用项目裸 Git 仓库。
    ProjectStorageRepoReady {
        req_id: String,
        project_id: String,
        storage_repo_path: String,
        #[serde(default)]
        storage_repo_url: Option<String>,
        #[serde(default)]
        storage_worktree_path: Option<String>,
        #[serde(default)]
        branch: Option<String>,
        created: bool,
    },
    /// 硬盘节点准备项目裸仓库失败。
    ProjectStorageRepoError {
        req_id: String,
        project_id: String,
        message: String,
    },
    /// PC 节点返回项目工作区巡检结果。
    ProjectWorkspaceInspected {
        req_id: String,
        status: ProjectWorkspaceInspectStatus,
    },
    /// PC 节点检查项目工作区失败。
    ProjectWorkspaceInspectError {
        req_id: String,
        message: String,
    },
    /// PC 节点返回项目 Git worktree 审计结果。
    ProjectGitWorktreesAudited {
        req_id: String,
        audit: ProjectGitWorktreeAudit,
    },
    /// PC 节点审计项目 Git worktree 失败。
    ProjectGitWorktreeAuditError {
        req_id: String,
        message: String,
    },
    /// PC 节点返回项目文档频道快照。
    ProjectDocumentsRead {
        req_id: String,
        snapshot: ProjectDocumentsSnapshot,
    },
    /// PC 节点读取项目文档失败。
    ProjectDocumentsReadError {
        req_id: String,
        message: String,
    },
    /// PC 节点返回单篇项目文档。
    ProjectDocumentFileRead {
        req_id: String,
        path: String,
        content: String,
        revision: String,
        byte_len: u64,
    },
    /// PC 节点读取单篇项目文档失败。
    ProjectDocumentFileReadError {
        req_id: String,
        message: String,
    },
    /// PC 节点完成单篇项目文档写入。
    ProjectDocumentFileWritten {
        req_id: String,
        path: String,
        revision: String,
        byte_len: u64,
    },
    /// PC 节点写入单篇项目文档失败。
    ProjectDocumentFileWriteError {
        req_id: String,
        message: String,
        #[serde(default)]
        conflict: bool,
    },
    /// PC 节点已清理项目工作区。
    ProjectWorkspaceCleaned {
        req_id: String,
        project_id: String,
        removed_paths: Vec<String>,
        skipped_paths: Vec<String>,
    },
    /// PC 节点清理项目工作区失败。
    ProjectWorkspaceCleanupError {
        req_id: String,
        project_id: String,
        message: String,
    },
    /// PC 节点 TTS 合成完成
    TtsSynthesizeResponse {
        req_id: String,
        /// base64 编码的音频字节
        audio_b64: String,
        /// MIME 类型，如 "audio/wav"
        mime: String,
        #[serde(default)]
        worker_voice: Option<String>,
    },
    /// PC 节点 TTS 合成失败
    TtsSynthesizeError {
        req_id: String,
        message: String,
    },
}

impl AgentToServer {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::TaskStdout { task_id, .. }
            | Self::TaskStderr { task_id, .. }
            | Self::TaskExit { task_id, .. }
            | Self::TaskError { task_id, .. } => Some(task_id.as_str()),
            Self::Register { .. }
            | Self::Pong { .. }
            | Self::HttpResponse { .. }
            | Self::HttpError { .. }
            | Self::CliPromptAccepted { .. }
            | Self::CliChunk { .. }
            | Self::CliDone { .. }
            | Self::CliCompletionReplay { .. }
            | Self::CliTaskJournalSnapshot { .. }
            | Self::ToolApprovalDecisionAck { .. }
            | Self::RegisterCapabilities { .. }
            | Self::LlmStreamChunk { .. }
            | Self::LlmStreamEnd { .. }
            | Self::LlmStreamError { .. }
            | Self::ProjectWorkspaceProvisioned { .. }
            | Self::ProjectWorkspaceProvisionError { .. }
            | Self::ProjectStorageRepoReady { .. }
            | Self::ProjectStorageRepoError { .. }
            | Self::ProjectWorkspaceInspected { .. }
            | Self::ProjectWorkspaceInspectError { .. }
            | Self::ProjectGitWorktreesAudited { .. }
            | Self::ProjectGitWorktreeAuditError { .. }
            | Self::ProjectDocumentsRead { .. }
            | Self::ProjectDocumentsReadError { .. }
            | Self::ProjectDocumentFileRead { .. }
            | Self::ProjectDocumentFileReadError { .. }
            | Self::ProjectDocumentFileWritten { .. }
            | Self::ProjectDocumentFileWriteError { .. }
            | Self::ProjectWorkspaceCleaned { .. }
            | Self::ProjectWorkspaceCleanupError { .. }
            | Self::TtsSynthesizeResponse { .. }
            | Self::TtsSynthesizeError { .. } => None,
        }
    }

    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::HttpResponse { req_id, .. }
            | Self::HttpError { req_id, .. }
            | Self::CliPromptAccepted { req_id, .. }
            | Self::CliChunk { req_id, .. }
            | Self::CliDone { req_id, .. }
            | Self::CliTaskJournalSnapshot { req_id, .. }
            | Self::LlmStreamChunk { req_id, .. }
            | Self::LlmStreamEnd { req_id, .. }
            | Self::LlmStreamError { req_id, .. }
            | Self::ProjectWorkspaceProvisioned { req_id, .. }
            | Self::ProjectWorkspaceProvisionError { req_id, .. }
            | Self::ProjectStorageRepoReady { req_id, .. }
            | Self::ProjectStorageRepoError { req_id, .. }
            | Self::ProjectWorkspaceInspected { req_id, .. }
            | Self::ProjectWorkspaceInspectError { req_id, .. }
            | Self::ProjectGitWorktreesAudited { req_id, .. }
            | Self::ProjectGitWorktreeAuditError { req_id, .. }
            | Self::ProjectDocumentsRead { req_id, .. }
            | Self::ProjectDocumentsReadError { req_id, .. }
            | Self::ProjectDocumentFileRead { req_id, .. }
            | Self::ProjectDocumentFileReadError { req_id, .. }
            | Self::ProjectDocumentFileWritten { req_id, .. }
            | Self::ProjectDocumentFileWriteError { req_id, .. }
            | Self::ProjectWorkspaceCleaned { req_id, .. }
            | Self::ProjectWorkspaceCleanupError { req_id, .. }
            | Self::TtsSynthesizeResponse { req_id, .. }
            | Self::TtsSynthesizeError { req_id, .. } => Some(req_id.as_str()),
            _ => None,
        }
    }

    /// 流式消息需要保留在 pending map 中（还有后续），其余 req_id 消息在发送后删除。
    pub fn is_final_req_msg(&self) -> bool {
        !matches!(
            self,
            Self::CliPromptAccepted { .. } | Self::CliChunk { .. } | Self::LlmStreamChunk { .. }
        )
    }
}

#[cfg(test)]
mod tests;
