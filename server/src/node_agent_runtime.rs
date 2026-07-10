// server/src/node_agent_runtime.rs
//! NodeRuntime — PC 节点的核心运行时状态：凭证、CLI 状态、任务注册表、lifecycle。
//! 从 node_agent_main.rs 抽取，保持原有公共接口不变。

use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{Notify, RwLock};
use tracing::{info, warn};

use homecli_proto::{ModelCapability, NodeHardwareProfile};

use crate::node_agent_active_task;
use crate::node_agent_active_task_registry;
use crate::node_agent_cli_probe::{
    cli_unavailable_after_refresh_error, probe_local_clis, LocalCliProbeSnapshot,
};
use crate::node_agent_cli_security;
use crate::node_agent_cli_sidecar;
use crate::node_agent_config::{save_persisted, Credentials, NodeConfig, PersistedState};
use crate::node_agent_full_access;
use crate::node_agent_lifecycle;
use crate::node_agent_local_admin;
use crate::node_agent_local_llm::discover_models;
use crate::node_agent_task_journal;
use crate::node_agent_tool_approval;
use crate::node_agent_workspace_match;
use crate::pc_storage_repo;

#[derive(Default)]
pub(crate) struct NodeStatus {
    pub(crate) connected: bool,
    pub(crate) last_event: String,
    pub(crate) models_cached: Vec<ModelCapability>,
}

pub(crate) struct NodeRuntime {
    pub(crate) cfg: NodeConfig,
    pub(crate) install_id: String,
    pub(crate) creds: RwLock<Option<Credentials>>,
    pub(crate) status: RwLock<NodeStatus>,
    hardware_cached: RwLock<NodeHardwareProfile>,
    cli_paths: RwLock<Vec<(String, String)>>,
    cli_probe_cached: RwLock<LocalCliProbeSnapshot>,
    pub(crate) cli_probe_refreshing: AtomicBool,
    model_scan_refreshing: AtomicBool,
    pub(crate) tts_worker_url: RwLock<Option<String>>,
    pub(crate) storage_settings: RwLock<pc_storage_repo::StorageSettings>,
    pub(crate) active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry,
    pub(crate) cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry,
    pub(crate) task_journal: node_agent_task_journal::TaskJournal,
    pub(crate) lifecycle: node_agent_lifecycle::NodeLifecycleTracker,
    pub(crate) tool_approvals: node_agent_tool_approval::ToolApprovalState,
    pub(crate) full_access_grants: node_agent_full_access::FullAccessGrantState,
    pub(crate) live_ui: std::sync::Arc<crate::node_agent_android_live::LiveUiBroker>,
    pub(crate) wake: Notify,
    local_admin_token: String,
}

impl NodeRuntime {
    pub(crate) fn new(
        cfg: NodeConfig,
        creds: Option<Credentials>,
        storage_settings: pc_storage_repo::StorageSettings,
        install_id: String,
    ) -> Self {
        let tts_url = std::env::var("NODE_TTS_WORKER_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        Self {
            cfg,
            install_id,
            creds: RwLock::new(creds),
            status: RwLock::new(NodeStatus::default()),
            hardware_cached: RwLock::new(crate::node_hardware_probe::collect_hardware_profile()),
            cli_paths: RwLock::new(Vec::new()),
            cli_probe_cached: RwLock::new(LocalCliProbeSnapshot::default()),
            cli_probe_refreshing: AtomicBool::new(false),
            model_scan_refreshing: AtomicBool::new(false),
            tts_worker_url: RwLock::new(tts_url),
            storage_settings: RwLock::new(storage_settings),
            active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry::new(),
            cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry::default(),
            task_journal: node_agent_task_journal::TaskJournal::default(),
            lifecycle: node_agent_lifecycle::NodeLifecycleTracker::start(env!("CARGO_PKG_VERSION")),
            tool_approvals: node_agent_tool_approval::ToolApprovalState::default(),
            full_access_grants: node_agent_full_access::FullAccessGrantState::load_default(),
            live_ui: std::sync::Arc::new(crate::node_agent_android_live::LiveUiBroker::new()),
            wake: Notify::new(),
            local_admin_token: node_agent_local_admin::generate_local_admin_token(),
        }
    }

    pub(crate) async fn creds(&self) -> Option<Credentials> {
        self.creds.read().await.clone()
    }

    pub(crate) fn cloud_http_url(&self) -> String {
        self.cfg.cloud_http_url.clone()
    }

    pub(crate) fn local_admin_token(&self) -> &str {
        &self.local_admin_token
    }

    pub(crate) async fn user_token(&self) -> Option<String> {
        self.creds
            .read()
            .await
            .as_ref()
            .and_then(|creds| creds.user_token.clone())
    }

    pub(crate) async fn set_cli_paths(&self, paths: Vec<(String, String)>) {
        *self.cli_paths.write().await = paths;
    }

    pub(crate) async fn cached_cli_probe(&self) -> LocalCliProbeSnapshot {
        self.cli_probe_cached.read().await.clone()
    }

    pub(crate) fn refresh_models_background(self: &Arc<Self>) {
        if self.model_scan_refreshing.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.clone();
        tokio::spawn(async move {
            let models = discover_models(&runtime.cfg).await;
            runtime.set_models(models).await;
            runtime
                .model_scan_refreshing
                .store(false, Ordering::Release);
        });
    }

    pub(crate) async fn ensure_cli_probe_background(self: &Arc<Self>, force: bool) {
        let stale = self.cached_cli_probe().await.is_stale();
        if !force && !stale {
            return;
        }
        if self.cli_probe_refreshing.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.clone();
        tokio::spawn(async move {
            let snapshot = tokio::task::spawn_blocking(probe_local_clis)
                .await
                .unwrap_or_else(|_| LocalCliProbeSnapshot::default());
            runtime.set_cli_probe_snapshot(snapshot).await;
            runtime.cli_probe_refreshing.store(false, Ordering::Release);
        });
    }

    pub(crate) async fn refresh_cli_probe_now(self: &Arc<Self>) -> LocalCliProbeSnapshot {
        if self.cli_probe_refreshing.swap(true, Ordering::AcqRel) {
            for _ in 0..24 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                if !self.cli_probe_refreshing.load(Ordering::Acquire) {
                    return self.cached_cli_probe().await;
                }
            }
            return self.cached_cli_probe().await;
        }
        let snapshot = tokio::task::spawn_blocking(probe_local_clis)
            .await
            .unwrap_or_else(|_| LocalCliProbeSnapshot::default());
        self.set_cli_probe_snapshot(snapshot.clone()).await;
        self.cli_probe_refreshing.store(false, Ordering::Release);
        snapshot
    }

    pub(crate) async fn set_cli_probe_snapshot(&self, snapshot: LocalCliProbeSnapshot) {
        let pairs = snapshot.available_pairs();
        self.set_cli_paths(pairs).await;
        *self.cli_probe_cached.write().await = snapshot;
    }

    pub(crate) async fn cli_prompt_active(&self, req_id: &str) -> bool {
        self.active_cli_prompts.contains(req_id).await
    }

    pub(crate) async fn try_register_cli_prompt(
        &self,
        handle: node_agent_active_task::ActiveCliPromptHandle,
    ) -> bool {
        self.active_cli_prompts.try_insert(handle).await
    }

    pub(crate) async fn cancel_cli_prompt(&self, req_id: &str) -> bool {
        let canceled = self
            .active_cli_prompts
            .cancel_tx(req_id)
            .await
            .map(|cancel_tx| cancel_tx.send(true).is_ok())
            .unwrap_or(false);
        if canceled {
            if let Err(error) = self.task_journal.record_cancel_requested(req_id) {
                warn!("PC 任务 journal 写入取消事件失败: {error}");
            }
            return true;
        }
        match self.cli_sidecars.record_cancel_command(req_id) {
            Ok(true) => {
                if let Err(error) = self.task_journal.record_cancel_requested(req_id) {
                    warn!("PC sidecar 任务 journal 写入取消事件失败: {error}");
                }
                true
            }
            Ok(false) => false,
            Err(error) => {
                warn!("PC sidecar 取消命令写入失败: {error}");
                false
            }
        }
    }

    pub(crate) async fn active_cli_prompt_view(
        &self,
        req_id: &str,
    ) -> Option<node_agent_active_task::ActiveCliPromptView> {
        let pending_approvals = self.tool_approvals.pending_for_req(req_id).await;
        self.active_cli_prompts
            .view(req_id, pending_approvals)
            .await
    }

    pub(crate) async fn active_cli_prompt_views_for_workspace(
        &self,
        workspace: &Path,
    ) -> Vec<node_agent_active_task::ActiveCliPromptView> {
        let workspace = node_agent_workspace_match::canonical_or_original(workspace);
        self.active_cli_prompts
            .views_without_approvals()
            .await
            .into_iter()
            .filter(|view| {
                view.cwd.as_deref().is_some_and(|cwd| {
                    node_agent_workspace_match::cwd_matches_workspace(cwd, &workspace)
                })
            })
            .collect()
    }

    pub(crate) fn task_journal_records_for_workspace(
        &self,
        workspace: &Path,
        limit: usize,
    ) -> anyhow::Result<Vec<node_agent_task_journal::TaskJournalRecord>> {
        self.task_journal
            .latest_records_for_workspace(workspace, limit)
    }

    pub(crate) fn task_journal_snapshot(
        &self,
        task_id: &str,
        since: usize,
        limit: usize,
    ) -> anyhow::Result<node_agent_task_journal::TaskJournalSnapshot> {
        self.task_journal.snapshot(task_id, since, limit)
    }

    pub(crate) async fn set_cli_prompt_os_pid(&self, req_id: &str, pid: Option<u32>) {
        self.active_cli_prompts.set_os_pid(req_id, pid).await;
    }

    /// 获取 sidecar 注册表的克隆（供 CLI 任务运行器使用）
    pub(crate) fn sidecar_registry(&self) -> node_agent_cli_sidecar::CliSidecarRegistry {
        self.cli_sidecars.clone()
    }

    pub(crate) async fn decide_tool_approval(
        &self,
        req_id: &str,
        approval_id: &str,
        decision: &str,
    ) -> bool {
        if self
            .tool_approvals
            .decide(req_id, approval_id, decision)
            .await
        {
            return true;
        }
        match self
            .cli_sidecars
            .record_tool_approval_decision(req_id, approval_id, decision)
        {
            Ok(accepted) => accepted,
            Err(error) => {
                warn!("PC sidecar 工具审批决定写入失败: {error}");
                false
            }
        }
    }

    pub(crate) async fn finish_cli_prompt(&self, req_id: &str) {
        let cleared_approvals = self.tool_approvals.clear_req(req_id).await;
        if cleared_approvals > 0 {
            info!("已清理 PC 任务 {req_id} 的 {cleared_approvals} 个遗留工具审批");
        }
        self.active_cli_prompts.remove(req_id).await;
    }

    pub(crate) async fn hardware_profile(&self) -> NodeHardwareProfile {
        self.hardware_cached.read().await.clone()
    }

    pub(crate) async fn refresh_hardware_profile(&self) -> NodeHardwareProfile {
        let hardware = crate::node_hardware_probe::collect_hardware_profile();
        *self.hardware_cached.write().await = hardware.clone();
        hardware
    }

    pub(crate) async fn resolve_cli(
        self: &Arc<Self>,
        name: &str,
    ) -> anyhow::Result<crate::node_agent_cli_security::ResolvedCli> {
        let cached_paths = self.cli_paths.read().await.clone();
        match node_agent_cli_security::resolve_cli_request(name, cached_paths.as_slice()) {
            Ok(resolved) => Ok(resolved),
            Err(cached_error) => {
                let refreshed = self.refresh_cli_probe_now().await;
                let refreshed_paths = refreshed.available_pairs();
                match node_agent_cli_security::resolve_cli_request(name, refreshed_paths.as_slice())
                {
                    Ok(resolved) => {
                        info!(
                            "PC CLI 缓存刷新后找到 {} CLI: {}",
                            resolved.name(),
                            resolved.bin()
                        );
                        Ok(resolved)
                    }
                    Err(_) => Err(cli_unavailable_after_refresh_error(
                        name,
                        cached_error,
                        &refreshed,
                    )),
                }
            }
        }
    }

    pub(crate) async fn set_creds(&self, c: Option<Credentials>) {
        let storage = self.storage_settings.read().await.clone();
        save_persisted(&PersistedState::from_parts(
            &self.install_id,
            c.as_ref(),
            &storage,
        ));
        *self.creds.write().await = c;
        self.wake.notify_waiters();
    }

    pub(crate) async fn set_storage_settings(&self, settings: pc_storage_repo::StorageSettings) {
        let creds = self.creds.read().await.clone();
        save_persisted(&PersistedState::from_parts(
            &self.install_id,
            creds.as_ref(),
            &settings,
        ));
        *self.storage_settings.write().await = settings;
        self.wake.notify_waiters();
    }

    pub(crate) async fn set_connected(&self, on: bool, evt: &str) {
        let mut s = self.status.write().await;
        s.connected = on;
        s.last_event = evt.to_string();
    }

    pub(crate) async fn set_models(&self, models: Vec<ModelCapability>) {
        self.status.write().await.models_cached = models;
    }

    /// 启动 lifecycle 心跳（供 run_agent_runtime 调用）
    pub(crate) fn spawn_lifecycle_heartbeat(&self) {
        node_agent_lifecycle::spawn_heartbeat(self.lifecycle.clone());
    }

    /// 标记计划内关闭（供 ctrl-c 信号处理调用）
    pub(crate) fn mark_lifecycle_planned_shutdown(&self, reason: &str) {
        self.lifecycle.mark_planned_shutdown(reason);
    }

    /// 标记关闭完成（供 ctrl-c 信号处理调用）
    pub(crate) fn mark_lifecycle_shutdown_completed(&self, reason: &str) {
        self.lifecycle.mark_shutdown_completed(reason);
    }
}
